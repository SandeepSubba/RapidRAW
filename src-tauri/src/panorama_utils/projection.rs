//! Projection support for panorama stitching.
//!
//! Planar (perspective) composition projects every frame onto the reference
//! frame's image plane; past ~60-90° of total field of view the edge frames
//! stretch without bound. Real stitchers (Photoshop Photomerge, PTGui,
//! OpenCV's Stitcher) fix this by estimating the focal length from the
//! pairwise homographies (rotation-only camera assumption) and warping every
//! frame onto a cylinder first, after which inter-frame motion is nearly a
//! pure translation and composition stays bounded.
//!
//! Focal-from-homography follows OpenCV's `focalsFromHomography`
//! (modules/stitching/src/autocalib.cpp), which implements the method from
//! Szeliski & Shum, "Creating full view panoramic image mosaics and
//! environment maps" (SIGGRAPH 1997). The homography must be expressed with
//! the principal point at the origin, so callers conjugate the pixel-space
//! homography with centering translations first.

use image::{GrayImage, Rgb32FImage};
use nalgebra::Matrix3;
use rayon::prelude::*;

use super::stitching::sample_bilinear;

/// Focal candidates (f0 for the source image, f1 for the destination) from a
/// single center-normalized homography. Port of OpenCV's focalsFromHomography.
fn focals_from_homography(h: &Matrix3<f64>) -> (Option<f64>, Option<f64>) {
    // nalgebra Matrix3 is column-major; index as (row, col) to match the
    // row-major h[0..9] layout the OpenCV formulas are written in.
    let h = [
        h[(0, 0)],
        h[(0, 1)],
        h[(0, 2)],
        h[(1, 0)],
        h[(1, 1)],
        h[(1, 2)],
        h[(2, 0)],
        h[(2, 1)],
        h[(2, 2)],
    ];

    let f1 = {
        let d1 = h[6] * h[7];
        let d2 = (h[7] - h[6]) * (h[7] + h[6]);
        let mut v1 = -(h[0] * h[1] + h[3] * h[4]) / d1;
        let mut v2 = (h[0] * h[0] + h[3] * h[3] - h[1] * h[1] - h[4] * h[4]) / d2;
        if v1 < v2 {
            std::mem::swap(&mut v1, &mut v2);
        }
        if v1 > 0.0 && v2 > 0.0 {
            Some(if d1.abs() > d2.abs() { v1 } else { v2 }.sqrt())
        } else if v1 > 0.0 {
            Some(v1.sqrt())
        } else {
            None
        }
    };

    let f0 = {
        let d1 = h[0] * h[3] + h[1] * h[4];
        let d2 = h[0] * h[0] + h[1] * h[1] - h[3] * h[3] - h[4] * h[4];
        let mut v1 = -h[2] * h[5] / d1;
        let mut v2 = (h[5] * h[5] - h[2] * h[2]) / d2;
        if v1 < v2 {
            std::mem::swap(&mut v1, &mut v2);
        }
        if v1 > 0.0 && v2 > 0.0 {
            Some(if d1.abs() > d2.abs() { v1 } else { v2 }.sqrt())
        } else if v1 > 0.0 {
            Some(v1.sqrt())
        } else {
            None
        }
    };

    (f0, f1)
}

/// Median focal length (in full-resolution pixels) over every matched pair.
///
/// `pairs` yields (homography mapping full-res pixels of image i -> full-res
/// pixels of image j, (w_i, h_i), (w_j, h_j)). Returns None when no pair
/// produces a usable estimate (e.g. pure-translation homographies).
pub fn estimate_focal(
    pairs: impl Iterator<Item = (Matrix3<f64>, (u32, u32), (u32, u32))>,
) -> Option<f64> {
    let mut focals: Vec<f64> = Vec::new();
    for (h, (wi, hi), (wj, hj)) in pairs {
        // Conjugate to principal-point-centered coordinates.
        let t_i = Matrix3::new(
            1.0,
            0.0,
            wi as f64 / 2.0,
            0.0,
            1.0,
            hi as f64 / 2.0,
            0.0,
            0.0,
            1.0,
        );
        let t_j_inv = Matrix3::new(
            1.0,
            0.0,
            -(wj as f64) / 2.0,
            0.0,
            1.0,
            -(hj as f64) / 2.0,
            0.0,
            0.0,
            1.0,
        );
        let h_centered = t_j_inv * h * t_i;
        let (f0, f1) = focals_from_homography(&h_centered);
        if let (Some(f0), Some(f1)) = (f0, f1) {
            focals.push((f0 * f1).sqrt());
        }
    }
    if focals.is_empty() {
        return None;
    }
    focals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = focals.len();
    Some(if n % 2 == 1 {
        focals[n / 2]
    } else {
        (focals[n / 2 - 1] + focals[n / 2]) / 2.0
    })
}

/// Warp a frame onto a cylinder of radius `focal` (pixels). Returns the
/// warped image and a validity mask (255 where the output pixel maps back
/// inside the source frame — the warp of a rectangle is barrel-shaped, and
/// the curved gaps must not be blended into the panorama as black content).
pub fn cylindrical_warp(src: &Rgb32FImage, focal: f64) -> (Rgb32FImage, GrayImage) {
    let (w, h) = src.dimensions();
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;

    let theta_max = (cx / focal).atan();
    let out_w = ((2.0 * focal * theta_max).ceil() as u32).clamp(1, w.max(1));
    let out_h = h;
    let ocx = out_w as f64 / 2.0;
    let ocy = out_h as f64 / 2.0;

    let mut buffer = vec![0.0f32; out_w as usize * out_h as usize * 3];
    let mut mask_buf = vec![0u8; out_w as usize * out_h as usize];

    buffer
        .par_chunks_mut(out_w as usize * 3)
        .zip(mask_buf.par_chunks_mut(out_w as usize))
        .enumerate()
        .for_each(|(y, (row, mask_row))| {
            let v = (y as f64 - ocy) / focal;
            for x in 0..out_w as usize {
                let theta = (x as f64 - ocx) / focal;
                let xs = cx + focal * theta.tan();
                let ys = cy + v * focal / theta.cos();
                if xs >= 0.0 && xs <= (w - 1) as f64 && ys >= 0.0 && ys <= (h - 1) as f64 {
                    let p = sample_bilinear(src, xs, ys);
                    let base = x * 3;
                    row[base] = p[0];
                    row[base + 1] = p[1];
                    row[base + 2] = p[2];
                    mask_row[x] = 255;
                }
            }
        });

    let image = Rgb32FImage::from_raw(out_w, out_h, buffer)
        .expect("cylindrical warp buffer matches dimensions");
    let mask =
        GrayImage::from_raw(out_w, out_h, mask_buf).expect("cylindrical mask matches dimensions");
    (image, mask)
}
