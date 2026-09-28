use crate::app_settings::load_settings;
use crate::app_state::AppState;
use crate::file_management::parse_virtual_path;
use base64::{Engine as _, engine::general_purpose};
use image::ImageFormat;
use image::{DynamicImage, GenericImageView, GrayImage, Rgb32FImage};
use nalgebra::Matrix3;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use crate::formats::is_raw_file;
use crate::image_processing::apply_cpu_default_raw_processing;
use crate::panorama_utils::{processing, stitching};

pub const BRIEF_DESCRIPTOR_SIZE: usize = 256;
pub type Descriptor = [u8; BRIEF_DESCRIPTOR_SIZE / 8];

#[derive(Debug, Clone, Copy)]
pub struct KeyPoint {
    pub x: u32,
    pub y: u32,
}

pub struct Feature {
    pub keypoint: KeyPoint,
    pub descriptor: Descriptor,
}

#[derive(Debug, Clone, Copy)]
pub struct Match {
    pub index1: usize,
    pub index2: usize,
}

pub struct ImageInfo {
    pub id: usize,
    pub filename: String,
    pub image: Rgb32FImage,
    // 255 where `image` holds real content. None = fully valid (unwarped
    // frames); cylindrically warped frames are barrel-shaped inside their
    // bounding rect and the gaps must be excluded from seams and blending.
    pub valid_mask: Option<GrayImage>,
    pub low_detail_mask: GrayImage,
    pub scale_factor: f64,
    pub features: Vec<Feature>,
}

#[derive(Clone)]
pub struct MatchInfo {
    pub homography: Matrix3<f64>,
    pub inliers: usize,
}

#[tauri::command]
pub async fn stitch_panorama(
    paths: Vec<String>,
    projection: Option<String>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if paths.len() < 2 {
        return Err("Please select at least two images to stitch.".to_string());
    }

    let source_paths: Vec<String> = paths
        .iter()
        .map(|p| parse_virtual_path(p).0.to_string_lossy().into_owned())
        .collect();

    let panorama_result_handle = state.panorama_result.clone();

    let task = tokio::task::spawn_blocking(move || {
        let panorama_result = stitch_images(source_paths, projection, app_handle.clone());

        match panorama_result {
            Ok(panorama_image) => {
                let _ = app_handle.emit("panorama-progress", "Creating preview...");

                let (w, h) = panorama_image.dimensions();
                let (new_w, new_h) = if w > h {
                    (800, (800.0 * h as f32 / w as f32).round() as u32)
                } else {
                    ((800.0 * w as f32 / h as f32).round() as u32, 800)
                };

                let preview_f32 =
                    crate::image_processing::downscale_f32_image(&panorama_image, new_w, new_h);

                let preview_u8 = preview_f32.to_rgb8();

                let mut buf = Cursor::new(Vec::new());

                if let Err(e) = preview_u8.write_to(&mut buf, ImageFormat::Png) {
                    return Err(format!("Failed to encode panorama preview: {}", e));
                }

                let base64_str = general_purpose::STANDARD.encode(buf.get_ref());
                let final_base64 = format!("data:image/png;base64,{}", base64_str);

                *panorama_result_handle.lock().unwrap() = Some(panorama_image);

                let _ = app_handle.emit(
                    "panorama-complete",
                    serde_json::json!({
                        "base64": final_base64,
                    }),
                );
                Ok(())
            }
            Err(e) => {
                let _ = app_handle.emit("panorama-error", e.clone());
                Err(e)
            }
        }
    });

    match task.await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(join_err) => Err(format!("Panorama task failed: {}", join_err)),
    }
}

#[tauri::command]
pub async fn save_panorama(
    first_path_str: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let panorama_image = state
        .panorama_result
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| {
            "No panorama image found in memory to save. It might have already been saved."
                .to_string()
        })?;

    let (first_path, _) = parse_virtual_path(&first_path_str);
    let parent_dir = first_path
        .parent()
        .ok_or_else(|| "Could not determine parent directory of the first image.".to_string())?;
    let stem = first_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("panorama");

    let (output_filename, image_to_save): (String, DynamicImage) =
        if panorama_image.color().has_alpha() {
            (
                format!("{}_Pano.png", stem),
                DynamicImage::ImageRgba8(panorama_image.to_rgba8()),
            )
        } else if panorama_image.as_rgb32f().is_some() {
            (format!("{}_Pano.tiff", stem), panorama_image)
        } else {
            (
                format!("{}_Pano.png", stem),
                DynamicImage::ImageRgb8(panorama_image.to_rgb8()),
            )
        };

    let output_path = parent_dir.join(output_filename);

    image_to_save
        .save(&output_path)
        .map_err(|e| format!("Failed to save panorama image: {}", e))?;

    let (real_path, _) = crate::file_management::parse_virtual_path(&first_path_str);
    let _ =
        crate::exif_processing::write_rrexif_sidecar(&real_path.to_string_lossy(), &output_path);

    Ok(output_path.to_string_lossy().to_string())
}

/// Panorama projection surface, selectable from the Stitch Panorama dialog
/// (mirrors Photoshop Photomerge's layout options).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Projection {
    /// Cylindrical when the estimated field of view is wide, else perspective.
    Auto,
    /// Compose on the reference frame's plane (best under ~60° of view;
    /// keeps straight lines straight).
    Perspective,
    /// Warp frames onto a cylinder first (bounded distortion at any width).
    Cylindrical,
}

impl Projection {
    fn parse(s: Option<&str>) -> Self {
        match s.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("perspective") | Some("planar") => Projection::Perspective,
            Some("cylindrical") | Some("cylinder") => Projection::Cylindrical,
            _ => Projection::Auto,
        }
    }
}

/// Auto mode switches to cylindrical past this estimated horizontal span.
/// Perspective composition visibly stretches edge frames from roughly 60-70°.
const AUTO_CYLINDRICAL_SPAN_DEG: f64 = 65.0;

/// Build an ImageInfo (grayscale, features, low-detail and scale metadata)
/// from a full-resolution frame — used for the initial load and again after
/// cylindrical warping, so both passes go through identical preparation.
fn prepare_image_info(
    id: usize,
    filename: &str,
    image_f32: Rgb32FImage,
    valid_mask: Option<GrayImage>,
    brief_pairs: &[(nalgebra::Point2<i32>, nalgebra::Point2<i32>)],
) -> ImageInfo {
    let color_full_u8 = DynamicImage::ImageRgb32F(image_f32.clone()).to_rgb8();
    let gray_full = image::imageops::colorops::grayscale(&color_full_u8);

    let (w, h) = gray_full.dimensions();
    let (new_w, new_h, scale_factor) = processing::calculate_downscale_dimensions(w, h);

    let gray_small = image::imageops::resize(
        &gray_full,
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let low_detail_mask = processing::generate_low_detail_mask(&gray_full);
    let features = processing::find_features(&gray_small, brief_pairs);
    println!("    Found {} features in '{}'", features.len(), filename);

    ImageInfo {
        id,
        filename: filename.to_string(),
        image: image_f32,
        valid_mask,
        low_detail_mask,
        scale_factor,
        features,
    }
}

/// All pairwise RANSAC homographies (in full-resolution pixel coordinates)
/// between frames with enough shared features.
fn compute_pairwise_matches(image_data: &[ImageInfo]) -> HashMap<(usize, usize), MatchInfo> {
    let mut pairwise_matches: HashMap<(usize, usize), MatchInfo> = HashMap::new();

    let pairs_to_check: Vec<(usize, usize)> = (0..image_data.len())
        .flat_map(|i| (i + 1..image_data.len()).map(move |j| (i, j)))
        .collect();

    let match_results: Vec<Option<((usize, usize), MatchInfo)>> = pairs_to_check
        .par_iter()
        .map(|&(i, j)| {
            let features1 = &image_data[i].features;
            let features2 = &image_data[j].features;

            let initial_matches = processing::match_features(features1, features2);
            if initial_matches.len() < processing::MIN_INLIERS_FOR_CONNECTION {
                return None;
            }

            let keypoints1: Vec<KeyPoint> = features1.iter().map(|f| f.keypoint).collect();
            let keypoints2: Vec<KeyPoint> = features2.iter().map(|f| f.keypoint).collect();

            if let Some((_h_small, inliers)) =
                processing::find_homography_ransac(&initial_matches, &keypoints1, &keypoints2)
                && inliers.len() >= processing::MIN_INLIERS_FOR_CONNECTION
            {
                println!(
                    "  - Good match found: '{}' <-> '{}' ({} inliers)",
                    Path::new(&image_data[i].filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    Path::new(&image_data[j].filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    inliers.len()
                );

                let inlier_points: Vec<(nalgebra::Point2<f64>, nalgebra::Point2<f64>)> = inliers
                    .iter()
                    .map(|m| {
                        let p1 = keypoints1[m.index1];
                        let p2 = keypoints2[m.index2];
                        (
                            nalgebra::Point2::new(p1.x as f64, p1.y as f64),
                            nalgebra::Point2::new(p2.x as f64, p2.y as f64),
                        )
                    })
                    .collect();

                if let Some(h_refined) = processing::compute_homography(&inlier_points) {
                    let s1 = image_data[i].scale_factor;
                    let s2 = image_data[j].scale_factor;
                    let scale_mat_i_inv =
                        Matrix3::new(1.0 / s1, 0.0, 0.0, 0.0, 1.0 / s1, 0.0, 0.0, 0.0, 1.0);
                    let scale_mat_j = Matrix3::new(s2, 0.0, 0.0, 0.0, s2, 0.0, 0.0, 0.0, 1.0);
                    let h_full = scale_mat_j * h_refined * scale_mat_i_inv;

                    let match_info = MatchInfo {
                        homography: h_full,
                        inliers: inliers.len(),
                    };
                    return Some(((i, j), match_info));
                }
            }
            None
        })
        .collect();

    for result in match_results.into_iter().flatten() {
        pairwise_matches.insert(result.0, result.1);
    }
    pairwise_matches
}

/// Estimated horizontal angular span of the panorama, from the planar
/// composition extents and the estimated focal length.
fn estimate_span_degrees(
    image_data: &[ImageInfo],
    ordered_indices: &[usize],
    global_homographies: &HashMap<usize, Matrix3<f64>>,
    focal: f64,
) -> f64 {
    let reference = ordered_indices[0];
    let (rw, _rh) = image_data[reference].image.dimensions();
    let ref_cx = rw as f64 / 2.0;

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    for &i in ordered_indices {
        let h = &global_homographies[&i];
        let (w, hgt) = image_data[i].image.dimensions();
        for (cx, cy) in [
            (0.0, 0.0),
            (w as f64, 0.0),
            (w as f64, hgt as f64),
            (0.0, hgt as f64),
        ] {
            let tp = h * nalgebra::Point3::new(cx, cy, 1.0);
            if tp.z.abs() > 1e-9 {
                let x = tp.x / tp.z;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }
    let left = ((min_x - ref_cx) / focal).atan();
    let right = ((max_x - ref_cx) / focal).atan();
    (right - left).to_degrees()
}

fn stitch_images(
    image_paths: Vec<String>,
    projection: Option<String>,
    app_handle: AppHandle,
) -> Result<DynamicImage, String> {
    if image_paths.len() < 2 {
        return Err("At least two images are required for a panorama.".to_string());
    }
    let projection = Projection::parse(projection.as_deref());

    let _ = app_handle.emit("panorama-progress", "Starting panorama process...");
    println!(
        "Starting panorama stitching process for {} images...",
        image_paths.len()
    );

    let settings = load_settings(app_handle.clone()).unwrap_or_default();

    let start_time = Instant::now();
    let _ = app_handle.emit("panorama-progress", "Loading and preparing images...");
    println!("Loading and preparing images (in parallel)...");
    let brief_pairs = processing::generate_brief_pairs();

    let image_data_results: Vec<Result<ImageInfo, String>> = image_paths
        .par_iter()
        .enumerate()
        .map(|(i, filename)| {
            let _ = app_handle.emit(
                "panorama-progress",
                format!(
                    "Processing '{}'",
                    Path::new(filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ),
            );
            println!("  - Processing '{}'", filename);

            let file_bytes = fs::read(filename)
                .map_err(|e| format!("Failed to read image {}: {}", filename, e))?;

            let mut dynamic_image = crate::image_loader::load_base_image_from_bytes(
                &file_bytes,
                filename,
                false,
                &settings,
                None,
            )
            .map_err(|e| format!("Failed to load image {}: {}", filename, e))?;

            if is_raw_file(filename) {
                apply_cpu_default_raw_processing(&mut dynamic_image);
            }

            let image_f32 = dynamic_image.to_rgb32f();

            Ok(prepare_image_info(i, filename, image_f32, None, &brief_pairs))
        })
        .collect();

    let mut image_data = Vec::new();
    for result in image_data_results {
        {
            let info = result?;
            image_data.push(info)
        }
    }

    println!(
        "Image loading and feature detection completed in {:.2?}\n",
        start_time.elapsed()
    );

    let start_time = Instant::now();
    let _ = app_handle.emit("panorama-progress", "Finding image matches...");
    println!("Finding all pairwise matches (in parallel)...");
    let mut pairwise_matches = compute_pairwise_matches(&image_data);
    println!(
        "Pairwise matching completed in {:.2?}\n",
        start_time.elapsed()
    );

    if pairwise_matches.is_empty() {
        return Err(
            "No suitable matches found between any pair of images. Cannot create a panorama."
                .to_string(),
        );
    }

    // ------------------------------------------------------------------
    // Projection selection. Perspective composition is only well-behaved
    // for narrow panoramas; for wide ones, estimate the focal length from
    // the pairwise homographies (rotation-only assumption, OpenCV-style)
    // and re-warp every frame onto a cylinder before the real stitch.
    // ------------------------------------------------------------------
    if projection != Projection::Perspective {
        let focal = crate::panorama_utils::projection::estimate_focal(
            pairwise_matches.iter().map(|(&(i, j), m)| {
                (
                    m.homography,
                    image_data[i].image.dimensions(),
                    image_data[j].image.dimensions(),
                )
            }),
        );

        let use_cylindrical = match (projection, focal) {
            (Projection::Cylindrical, _) => true,
            (Projection::Auto, Some(f)) => {
                let (ordered, globals, _) = build_stitching_order(&image_data, &pairwise_matches);
                if ordered.len() < 2 {
                    false
                } else {
                    let span = estimate_span_degrees(&image_data, &ordered, &globals, f);
                    println!(
                        "Estimated field of view: {:.1} deg (focal {:.0}px)",
                        span, f
                    );
                    span > AUTO_CYLINDRICAL_SPAN_DEG
                }
            }
            _ => false,
        };

        if use_cylindrical {
            // Explicit cylindrical without a focal estimate falls back to a
            // ~70 deg horizontal FOV assumption.
            let max_w = image_data
                .iter()
                .map(|d| d.image.width())
                .max()
                .unwrap_or(1) as f64;
            let f = focal.unwrap_or(0.7 * max_w);
            let _ = app_handle.emit(
                "panorama-progress",
                "Projecting frames onto a cylinder...",
            );
            println!("Cylindrical projection selected (focal {:.0}px)", f);

            image_data = image_data
                .into_par_iter()
                .map(|info| {
                    let (warped, mask) =
                        crate::panorama_utils::projection::cylindrical_warp(&info.image, f);
                    prepare_image_info(
                        info.id,
                        &info.filename,
                        warped,
                        Some(mask),
                        &brief_pairs,
                    )
                })
                .collect();

            let _ = app_handle.emit("panorama-progress", "Re-matching projected frames...");
            println!("Re-matching cylindrically projected frames...");
            pairwise_matches = compute_pairwise_matches(&image_data);
            if pairwise_matches.is_empty() {
                return Err(
                    "No suitable matches found after cylindrical projection. Try the Perspective projection instead."
                        .to_string(),
                );
            }
        }
    }

    let start_time = Instant::now();
    let _ = app_handle.emit("panorama-progress", "Determining stitching order...");
    println!("Determining stitching order...");
    let (ordered_indices, global_homographies, mst_adj) =
        build_stitching_order(&image_data, &pairwise_matches);

    if ordered_indices.len() < 2 {
        return Err("Could not find a connected sequence of at least two images.".to_string());
    }

    // Equalize exposure across frames before blending: auto-exposure drift
    // between shots otherwise shows up as brightness steps at every seam,
    // no matter how good the seam itself is.
    let _ = app_handle.emit("panorama-progress", "Equalizing exposure between frames...");
    let gains = compute_gain_compensation(
        &image_data,
        &mst_adj,
        &global_homographies,
        ordered_indices[0],
    );
    for info in image_data.iter_mut() {
        if let Some(&g) = gains.get(&info.id) {
            if (g - 1.0).abs() > 0.005 {
                println!(
                    "  - Gain {:.3} applied to '{}'",
                    g,
                    Path::new(&info.filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                let g = g as f32;
                for p in info.image.pixels_mut() {
                    p.0[0] *= g;
                    p.0[1] *= g;
                    p.0[2] *= g;
                }
            }
        }
    }

    let ordered_filenames: Vec<_> = ordered_indices
        .iter()
        .map(|&i| {
            Path::new(&image_data[i].filename)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    println!("Stitching order determined: {:?}", ordered_filenames);
    let _ = app_handle.emit(
        "panorama-progress",
        format!("Stitching order: {}", ordered_filenames.join(" -> ")),
    );

    let stitched_images_info: Vec<&ImageInfo> =
        ordered_indices.iter().map(|&i| &image_data[i]).collect();
    let unstitched_count = image_data.len() - stitched_images_info.len();
    if unstitched_count > 0 {
        let warning_msg = format!(
            "Warning: {} image(s) could not be matched and will be excluded.",
            unstitched_count
        );
        println!("{}", warning_msg);
        let _ = app_handle.emit("panorama-warning", warning_msg);
    }
    println!(
        "Global homography calculation completed in {:.2?}\n",
        start_time.elapsed()
    );

    let start_time = Instant::now();
    let _ = app_handle.emit("panorama-progress", "Warping and blending images...");
    println!("Warping and blending full-resolution images with progressive optimal seams...");

    let panorama = stitching::progressive_seam_stitcher(
        &stitched_images_info,
        &global_homographies,
        app_handle.clone(),
    );

    println!("Stitching completed in {:.2?}\n", start_time.elapsed());

    let _ = app_handle.emit("panorama-progress", "Finalizing panorama...");

    Ok(DynamicImage::ImageRgb32F(panorama))
}

struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            self.parent[i] = self.find(self.parent[i]);
            self.parent[i]
        }
    }

    fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}

fn build_stitching_order(
    images: &[ImageInfo],
    matches: &HashMap<(usize, usize), MatchInfo>,
) -> (
    Vec<usize>,
    HashMap<usize, Matrix3<f64>>,
    HashMap<usize, Vec<usize>>,
) {
    if images.is_empty() {
        return (vec![], HashMap::new(), HashMap::new());
    }
    let n = images.len();
    if n < 2 {
        let mut homographies = HashMap::new();
        if n == 1 {
            homographies.insert(0, Matrix3::identity());
        }
        return ((0..n).collect(), homographies, HashMap::new());
    }

    let mut edges = Vec::new();
    for (&(i, j), m) in matches {
        edges.push((m.inliers, i, j));
    }
    edges.sort_by_key(|&(inliers, _, _)| std::cmp::Reverse(inliers));

    let mut mst_adj: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut dsu = Dsu::new(n);
    let mut num_edges = 0;

    for &(_, i, j) in &edges {
        if dsu.find(i) != dsu.find(j) {
            dsu.union(i, j);
            mst_adj.entry(i).or_default().push(j);
            mst_adj.entry(j).or_default().push(i);
            num_edges += 1;
            if num_edges == n - 1 {
                break;
            }
        }
    }

    // Anchor the panorama on the CENTER of the match tree, not a leaf.
    // Every other image is projected onto the reference frame's plane, so
    // projective distortion grows with hop distance — starting from an end
    // frame makes the far end stretch grotesquely, while the tree center
    // splits the chain in half and keeps the warp symmetric and small.
    let bfs_eccentricity = |s: usize| -> usize {
        let mut dist: HashMap<usize, usize> = HashMap::new();
        dist.insert(s, 0);
        let mut q = VecDeque::from([s]);
        let mut ecc = 0usize;
        while let Some(u) = q.pop_front() {
            let d = dist[&u];
            ecc = ecc.max(d);
            if let Some(nbrs) = mst_adj.get(&u) {
                for &v in nbrs {
                    if !dist.contains_key(&v) {
                        dist.insert(v, d + 1);
                        q.push_back(v);
                    }
                }
            }
        }
        ecc
    };
    let start_node = (0..n)
        .filter(|i| mst_adj.contains_key(i))
        .min_by_key(|&i| (bfs_eccentricity(i), i))
        .unwrap_or_else(|| mst_adj.keys().next().copied().unwrap_or(0));

    let mut ordered_indices = Vec::new();
    let mut global_homographies = HashMap::new();
    let mut q = VecDeque::new();
    let mut visited = HashSet::new();

    q.push_back((start_node, Matrix3::identity()));
    visited.insert(start_node);

    while let Some((u, h_u_global)) = q.pop_front() {
        ordered_indices.push(u);
        global_homographies.insert(u, h_u_global);

        if let Some(neighbors) = mst_adj.get(&u) {
            for &v in neighbors {
                if !visited.contains(&v) {
                    visited.insert(v);

                    let h_vu = if let Some(m) = matches.get(&(v, u)) {
                        m.homography
                    } else if let Some(m) = matches.get(&(u, v)) {
                        m.homography
                            .try_inverse()
                            .expect("Failed to invert homography for MST edge")
                    } else {
                        panic!("Match not found for MST edge between {} and {}", u, v);
                    };

                    let h_v_global = h_u_global * h_vu;
                    q.push_back((v, h_v_global));
                }
            }
        }
    }

    (ordered_indices, global_homographies, mst_adj)
}

/// Per-image multiplicative gains that equalize exposure across the panorama.
///
/// For every match-tree edge, the mean luminance of both frames is sampled
/// over their overlap region (via the global homographies); gains then
/// propagate outward from the reference frame so each pair agrees in the
/// overlap, and are finally normalized to a geometric mean of 1.0 so the
/// panorama's overall brightness stays where the photographer put it.
fn compute_gain_compensation(
    images: &[ImageInfo],
    mst_adj: &HashMap<usize, Vec<usize>>,
    global_homographies: &HashMap<usize, Matrix3<f64>>,
    reference: usize,
) -> HashMap<usize, f64> {
    let mut gains: HashMap<usize, f64> = HashMap::new();
    gains.insert(reference, 1.0);
    let mut visited = HashSet::from([reference]);
    let mut q = VecDeque::from([reference]);

    while let Some(u) = q.pop_front() {
        let g_u = gains[&u];
        let Some(nbrs) = mst_adj.get(&u) else { continue };
        for &v in nbrs {
            if visited.contains(&v) {
                continue;
            }
            visited.insert(v);
            let ratio = match (global_homographies.get(&u), global_homographies.get(&v)) {
                (Some(h_u), Some(h_v)) => overlap_luminance_ratio(&images[u], &images[v], h_u, h_v),
                _ => 1.0,
            };
            // A single bad overlap must not blow up the whole chain.
            let g_v = (g_u * ratio).clamp(0.4, 2.5);
            gains.insert(v, g_v);
            q.push_back(v);
        }
    }

    // Geometric-mean normalization: correct relative differences without
    // shifting the panorama's average exposure.
    let log_mean =
        gains.values().map(|g| g.ln()).sum::<f64>() / gains.len().max(1) as f64;
    let norm = (-log_mean).exp();
    for g in gains.values_mut() {
        *g *= norm;
    }
    gains
}

/// mean_lum(u) / mean_lum(v) over the overlap of frames u and v, sampled on a
/// coarse grid of v's pixels mapped through the global homographies. Returns
/// 1.0 when the overlap is too small to trust.
fn overlap_luminance_ratio(
    info_u: &ImageInfo,
    info_v: &ImageInfo,
    h_u: &Matrix3<f64>,
    h_v: &Matrix3<f64>,
) -> f64 {
    let Some(h_u_inv) = h_u.try_inverse() else {
        return 1.0;
    };
    let h_v_to_u = h_u_inv * h_v;

    let (wv, hv) = info_v.image.dimensions();
    let (wu, hu) = info_u.image.dimensions();
    let step = (wv.max(hv) / 100).max(4) as usize;

    let mut sum_u = 0.0f64;
    let mut sum_v = 0.0f64;
    let mut count = 0usize;

    for y in (0..hv).step_by(step) {
        for x in (0..wv).step_by(step) {
            let p = nalgebra::Point3::new(x as f64, y as f64, 1.0);
            let tp = h_v_to_u * p;
            if tp.z.abs() < 1e-9 {
                continue;
            }
            let ux = tp.x / tp.z;
            let uy = tp.y / tp.z;
            if ux < 0.0 || ux >= (wu - 1) as f64 || uy < 0.0 || uy >= (hu - 1) as f64 {
                continue;
            }
            if let Some(m) = &info_v.valid_mask {
                if m.get_pixel(x, y)[0] == 0 {
                    continue;
                }
            }
            if let Some(m) = &info_u.valid_mask {
                let mx = (ux.round() as u32).min(m.width() - 1);
                let my = (uy.round() as u32).min(m.height() - 1);
                if m.get_pixel(mx, my)[0] == 0 {
                    continue;
                }
            }
            let pv = info_v.image.get_pixel(x, y);
            let pu = stitching::sample_bilinear(&info_u.image, ux, uy);
            sum_v += (pv[0] as f64 + pv[1] as f64 + pv[2] as f64) / 3.0;
            sum_u += (pu[0] as f64 + pu[1] as f64 + pu[2] as f64) / 3.0;
            count += 1;
        }
    }

    if count < 50 || sum_v <= 1e-6 || sum_u <= 1e-6 {
        return 1.0;
    }
    sum_u / sum_v
}
