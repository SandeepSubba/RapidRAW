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

/// General Pannini forward map for a view direction at longitude `theta` and
/// cylinder height `h` (tan of the elevation): returns normalized (x, y).
/// d = 0 is rectilinear, d = 1 classic Pannini.
fn pannini_forward(theta: f64, h: f64, d: f64) -> (f64, f64) {
    let s = (d + 1.0) / (d + theta.cos());
    (s * theta.sin(), s * h)
}

/// Longitude for a normalized Pannini x. From x·(d + cosθ) = (d + 1)·sinθ:
/// with R = hypot(d + 1, x) and α = atan2(x, d + 1), sin(θ − α) = x·d / R.
fn pannini_theta(x: f64, d: f64) -> f64 {
    let a = d + 1.0;
    let r = (a * a + x * x).sqrt();
    x.atan2(a) + (x * d / r).clamp(-1.0, 1.0).asin()
}

/// Re-project a stitched cylindrical panorama so straight lines come out
/// straight: the "Straight lines" projection for interiors and architecture.
///
/// A cylinder keeps verticals straight but bows every horizontal line off the
/// horizon, and how much depends on the longitude, so no radial distortion or
/// keystone slider can undo it. The fix has to be a re-projection. This uses
/// the general Pannini family: `strength` 1.0 (d = 0) is rectilinear, where
/// every straight line is straight but the edges stretch as 1/cos²θ (about 8x
/// at 140° of view); 0.0 (d = 1) is classic Pannini, where verticals and lines
/// through the centre stay straight, the edges compress, and other horizontals
/// keep a slight bow. Verticals stay vertical at every strength because x
/// depends on the longitude alone.
///
/// `focal` is the cylinder radius in pixels and `horizon_y` the canvas row of
/// the optical axis (h = 0). The view is centred on the middle of the content.
/// The output is scaled down when full-strength stretching would otherwise
/// blow up its size.
pub fn straighten_cylindrical(
    pano: &Rgb32FImage,
    mask: &GrayImage,
    focal: f64,
    horizon_y: f64,
    strength: f64,
) -> (Rgb32FImage, GrayImage) {
    let d = 1.0 - strength.clamp(0.0, 1.0);
    let (w, h) = pano.dimensions();

    let (mut x0, mut x1, mut y0, mut y1) = (u32::MAX, 0u32, u32::MAX, 0u32);
    for (x, y, p) in mask.enumerate_pixels() {
        if p.0[0] > 0 {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
    }
    if x0 > x1 || y0 > y1 || focal <= 0.0 {
        return (pano.clone(), mask.clone());
    }

    let center_x = (x0 as f64 + x1 as f64 + 1.0) / 2.0;
    // Past ±85° a rectilinear map runs off to infinity.
    let limit = 85f64.to_radians();
    let th_min = ((x0 as f64 - center_x) / focal).max(-limit);
    let th_max = ((x1 as f64 + 1.0 - center_x) / focal).min(limit);
    let h_min = (y0 as f64 - horizon_y) / focal;
    let h_max = (y1 as f64 + 1.0 - horizon_y) / focal;

    // S grows with |θ|, so the vertical extent peaks at the widest edge.
    let s_edge = pannini_forward(th_min, 1.0, d).1.max(pannini_forward(th_max, 1.0, d).1);
    let x_min = pannini_forward(th_min, 0.0, d).0;
    let x_max = pannini_forward(th_max, 0.0, d).0;
    let y_min = if h_min < 0.0 { s_edge * h_min } else { h_min };
    let y_max = if h_max > 0.0 { s_edge * h_max } else { h_max };

    let natural_w = (x_max - x_min) * focal;
    let natural_h = (y_max - y_min) * focal;
    const MAX_SIDE: f64 = 24000.0;
    let pixel_budget = 1.5 * w as f64 * h as f64;
    let scale = 1.0f64
        .min((pixel_budget / (natural_w * natural_h).max(1.0)).sqrt())
        .min(MAX_SIDE / natural_w.max(1.0))
        .min(MAX_SIDE / natural_h.max(1.0));
    let f_out = focal * scale;
    if scale < 1.0 {
        println!(
            "  - Straight lines: output scaled to {:.0}% to keep the stretched edges in budget",
            scale * 100.0
        );
    }

    let out_w = ((x_max - x_min) * f_out).ceil().max(1.0) as u32;
    let out_h = ((y_max - y_min) * f_out).ceil().max(1.0) as u32;
    let mut buffer = vec![0.0f32; out_w as usize * out_h as usize * 3];
    let mut mask_buf = vec![0u8; out_w as usize * out_h as usize];

    buffer
        .par_chunks_mut(out_w as usize * 3)
        .zip(mask_buf.par_chunks_mut(out_w as usize))
        .enumerate()
        .for_each(|(v, (row, mask_row))| {
            let y_norm = y_min + (v as f64 + 0.5) / f_out;
            for u in 0..out_w as usize {
                let x_norm = x_min + (u as f64 + 0.5) / f_out;
                let theta = pannini_theta(x_norm, d);
                if theta < th_min || theta > th_max {
                    continue;
                }
                let s = pannini_forward(theta, 1.0, d).1;
                let xs = center_x + focal * theta - 0.5;
                let ys = horizon_y + focal * (y_norm / s) - 0.5;
                if xs < 0.0 || ys < 0.0 || xs > (w - 1) as f64 || ys > (h - 1) as f64 {
                    continue;
                }
                if mask.get_pixel(xs.round() as u32, ys.round() as u32).0[0] == 0 {
                    continue;
                }
                let p = sample_bilinear(pano, xs, ys);
                let base = u * 3;
                row[base] = p[0];
                row[base + 1] = p[1];
                row[base + 2] = p[2];
                mask_row[u] = 255;
            }
        });

    let image = Rgb32FImage::from_raw(out_w, out_h, buffer)
        .expect("straight-lines buffer matches dimensions");
    let mask =
        GrayImage::from_raw(out_w, out_h, mask_buf).expect("straight-lines mask matches dimensions");
    (image, mask)
}

#[cfg(test)]
mod straight_lines_tests {
    use super::*;

    #[test]
    fn pannini_theta_inverts_forward() {
        for &d in &[0.0, 0.3, 0.7, 1.0] {
            for i in -16..=16 {
                let theta = i as f64 * 0.1; // ±1.6 rad, inside ±85° is ±1.48
                if theta.abs() > 85f64.to_radians() {
                    continue;
                }
                let (x, _) = pannini_forward(theta, 0.0, d);
                let back = pannini_theta(x, d);
                assert!((back - theta).abs() < 1e-9, "d={} theta={} back={}", d, theta, back);
            }
        }
    }

    #[test]
    fn full_strength_is_rectilinear() {
        // d = 0 maps a direction exactly as a pinhole camera would.
        for &(x3, y3, z3) in &[(1.0, 0.5, 2.0), (-2.5, -0.8, 1.0), (0.3, 1.2, 4.0)] {
            let theta = (x3 as f64).atan2(z3);
            let h = y3 / (x3 * x3 + z3 * z3 as f64).sqrt();
            let (x, y) = pannini_forward(theta, h, 0.0);
            assert!((x - x3 / z3).abs() < 1e-9);
            assert!((y - y3 / z3).abs() < 1e-9);
        }
    }

    #[test]
    fn full_strength_straightens_a_wall_line() {
        // A horizontal edge on a wall facing the camera (constant height Y,
        // depth Z) is a curve on the cylinder but a flat row after the remap.
        let (y3, z3) = (-0.6f64, 2.0f64);
        let rows: Vec<f64> = (-20..=20)
            .map(|i| {
                let x3 = i as f64 * 0.2;
                let theta = x3.atan2(z3);
                let h = y3 / (x3 * x3 + z3 * z3).sqrt();
                pannini_forward(theta, h, 0.0).1
            })
            .collect();
        let spread = rows.iter().cloned().fold(f64::MIN, f64::max)
            - rows.iter().cloned().fold(f64::MAX, f64::min);
        assert!(spread < 1e-9, "line bows by {}", spread);
    }

    #[test]
    fn verticals_stay_vertical_at_any_strength() {
        // Points on one vertical edge share θ, so they share x.
        for &d in &[0.0, 0.5, 1.0] {
            let xs: Vec<f64> = [-0.9, -0.2, 0.4, 1.1]
                .iter()
                .map(|&h| pannini_forward(0.7, h, d).0)
                .collect();
            assert!(xs.windows(2).all(|p| (p[0] - p[1]).abs() < 1e-12));
        }
    }

    #[test]
    fn remap_keeps_content_and_straightens_the_bow() {
        // A synthetic cylinder panorama: a bright band along a wall edge that
        // the cylinder bows, drawn at y = horizon + f·h(θ).
        // ±28° of view keeps the output at scale 1, so the band stays 3 px thick.
        let (w, h, f) = (400u32, 300u32, 400.0f64);
        let horizon = 150.0;
        let mut pano = Rgb32FImage::new(w, h);
        let mask = GrayImage::from_pixel(w, h, image::Luma([255]));
        // Bows ~12 px on the cylinder between the centre and the edges.
        let (y3, z3) = (-0.25f64, 1.0f64);
        for x in 0..w {
            let theta = (x as f64 + 0.5 - w as f64 / 2.0) / f;
            let x3 = z3 * theta.tan();
            let hh = y3 / (x3 * x3 + z3 * z3).sqrt();
            let row = (horizon + f * hh).round() as i64;
            for dy in -1..=1 {
                let yy = row + dy;
                if yy >= 0 && yy < h as i64 {
                    pano.put_pixel(x, yy as u32, image::Rgb([1.0, 1.0, 1.0]));
                }
            }
        }
        let (out, out_mask) = straighten_cylindrical(&pano, &mask, f, horizon, 1.0);
        assert!(out.width() > 0 && out.height() > 0);
        assert!(out_mask.pixels().any(|p| p.0[0] > 0));
        // The band's row, per column, must be (nearly) constant after the remap.
        let mut rows = Vec::new();
        for x in (out.width() / 4)..(3 * out.width() / 4) {
            let mut best = None;
            for y in 0..out.height() {
                if out.get_pixel(x, y).0[0] > 0.5 {
                    best = Some(y);
                    break;
                }
            }
            if let Some(y) = best {
                rows.push(y as i64);
            }
        }
        assert!(rows.len() > 10, "band not found after remap");
        let spread = rows.iter().max().unwrap() - rows.iter().min().unwrap();
        assert!(spread <= 2, "band still bows by {} px", spread);
    }
}
