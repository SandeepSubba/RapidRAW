use super::*;

#[derive(Serialize, Clone)]
pub struct HistogramData {
    red: Vec<f32>,
    green: Vec<f32>,
    blue: Vec<f32>,
    luma: Vec<f32>,
}

pub fn calculate_histogram_from_image(image: &DynamicImage) -> Result<HistogramData, String> {
    let init_hist = || ([0u32; 256], [0u32; 256], [0u32; 256], [0u32; 256]);

    let reduce_hist = |mut a: ([u32; 256], [u32; 256], [u32; 256], [u32; 256]),
                       b: ([u32; 256], [u32; 256], [u32; 256], [u32; 256])| {
        for i in 0..256 {
            a.0[i] += b.0[i];
            a.1[i] += b.1[i];
            a.2[i] += b.2[i];
            a.3[i] += b.3[i];
        }
        a
    };

    let (r_c, g_c, b_c, l_c) = match image {
        DynamicImage::ImageRgb32F(f32_img) => {
            let raw = f32_img.as_raw();
            raw.par_chunks(30_000)
                .fold(init_hist, |mut acc, chunk| {
                    for pixel in chunk.chunks_exact(3).step_by(2) {
                        let r = (pixel[0].clamp(0.0, 1.0) * 255.0) as usize;
                        let g = (pixel[1].clamp(0.0, 1.0) * 255.0) as usize;
                        let b = (pixel[2].clamp(0.0, 1.0) * 255.0) as usize;

                        acc.0[r] += 1;
                        acc.1[g] += 1;
                        acc.2[b] += 1;

                        let luma = (r * 218 + g * 732 + b * 74) >> 10;
                        acc.3[luma.min(255)] += 1;
                    }
                    acc
                })
                .reduce(init_hist, reduce_hist)
        }
        _ => {
            let rgb = image.to_rgb8();
            let raw = rgb.as_raw();
            raw.par_chunks(30_000)
                .fold(init_hist, |mut acc, chunk| {
                    for pixel in chunk.chunks_exact(3).step_by(2) {
                        let r = pixel[0] as usize;
                        let g = pixel[1] as usize;
                        let b = pixel[2] as usize;

                        acc.0[r] += 1;
                        acc.1[g] += 1;
                        acc.2[b] += 1;

                        let luma = (r * 218 + g * 732 + b * 74) >> 10;
                        acc.3[luma.min(255)] += 1;
                    }
                    acc
                })
                .reduce(init_hist, reduce_hist)
        }
    };

    let mut red: Vec<f32> = r_c.into_iter().map(|c| c as f32).collect();
    let mut green: Vec<f32> = g_c.into_iter().map(|c| c as f32).collect();
    let mut blue: Vec<f32> = b_c.into_iter().map(|c| c as f32).collect();
    let mut luma: Vec<f32> = l_c.into_iter().map(|c| c as f32).collect();

    let smoothing_sigma = 2.0;
    apply_gaussian_smoothing(&mut red, smoothing_sigma);
    apply_gaussian_smoothing(&mut green, smoothing_sigma);
    apply_gaussian_smoothing(&mut blue, smoothing_sigma);
    apply_gaussian_smoothing(&mut luma, smoothing_sigma);

    normalize_histogram_range(&mut red, 0.99);
    normalize_histogram_range(&mut green, 0.99);
    normalize_histogram_range(&mut blue, 0.99);
    normalize_histogram_range(&mut luma, 0.99);

    Ok(HistogramData {
        red,
        green,
        blue,
        luma,
    })
}

fn apply_gaussian_smoothing(histogram: &mut [f32], sigma: f32) {
    if sigma <= 0.0 {
        return;
    }

    let kernel_radius = (sigma * 3.0).ceil() as usize;
    if kernel_radius == 0 || kernel_radius >= histogram.len() {
        return;
    }

    let kernel_size = 2 * kernel_radius + 1;
    let mut kernel = vec![0.0; kernel_size];
    let mut kernel_sum = 0.0;

    let two_sigma_sq = 2.0 * sigma * sigma;
    for (i, kernel_val) in kernel.iter_mut().enumerate() {
        let x = (i as i32 - kernel_radius as i32) as f32;
        let val = (-x * x / two_sigma_sq).exp();
        *kernel_val = val;
        kernel_sum += val;
    }

    if kernel_sum > 0.0 {
        for val in &mut kernel {
            *val /= kernel_sum;
        }
    }

    let original = histogram.to_owned();
    let len = histogram.len();

    for (i, hist_val) in histogram.iter_mut().enumerate() {
        let mut smoothed_val = 0.0;
        for (k, &kernel_val) in kernel.iter().enumerate() {
            let offset = k as i32 - kernel_radius as i32;
            let sample_index = i as i32 + offset;
            let clamped_index = sample_index.clamp(0, len as i32 - 1) as usize;
            smoothed_val += original[clamped_index] * kernel_val;
        }
        *hist_val = smoothed_val;
    }
}

fn normalize_histogram_range(histogram: &mut [f32], percentile_clip: f32) {
    if histogram.is_empty() {
        return;
    }

    let mut sorted_data = histogram.to_owned();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let clip_index = ((sorted_data.len() - 1) as f32 * percentile_clip).round() as usize;
    let max_val = sorted_data[clip_index.min(sorted_data.len() - 1)];

    if max_val > 1e-6 {
        let scale_factor = 1.0 / max_val;
        for value in histogram.iter_mut() {
            *value = (*value * scale_factor).min(1.0);
        }
    } else {
        for value in histogram.iter_mut() {
            *value = 0.0;
        }
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub rgb: String,
    pub luma: String,
    pub parade: String,
    pub vectorscope: String,
    pub width: u32,
    pub height: u32,
}

pub fn calculate_waveform_from_image(
    image: &DynamicImage,
    active_channel: Option<&str>,
) -> Result<WaveformData, String> {
    const W: usize = 256;
    const H: usize = 256;

    let (orig_w, orig_h) = image.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Err("Image has zero dimensions.".to_string());
    }

    let do_rgb = active_channel.is_none() || active_channel == Some("rgb");
    let do_luma =
        active_channel.is_none() || active_channel == Some("luma") || active_channel == Some("rgb");
    let do_parade = active_channel.is_none() || active_channel == Some("parade");
    let do_vectorscope = active_channel.is_none() || active_channel == Some("vectorscope");

    let mut red_bins = if do_rgb { vec![0u32; W * H] } else { vec![] };
    let mut green_bins = if do_rgb { vec![0u32; W * H] } else { vec![] };
    let mut blue_bins = if do_rgb { vec![0u32; W * H] } else { vec![] };
    let mut luma_bins = if do_luma { vec![0u32; W * H] } else { vec![] };
    let mut parade_bins = if do_parade { vec![0u32; W * H] } else { vec![] };
    let mut vector_bins = if do_vectorscope {
        vec![0u32; W * H]
    } else {
        vec![]
    };

    let x_scale = W as f32 / orig_w as f32;
    let mut x_buckets = vec![0usize; orig_w as usize];

    let mut x_buckets_parade_r = vec![0usize; orig_w as usize];
    let mut x_buckets_parade_g = vec![0usize; orig_w as usize];
    let mut x_buckets_parade_b = vec![0usize; orig_w as usize];

    for x in 0..(orig_w as usize) {
        x_buckets[x] = ((x as f32 * x_scale) as usize).min(W - 1);
        if do_parade {
            let relative_x = x as f32 / orig_w as f32;
            x_buckets_parade_r[x] = (relative_x * 82.0) as usize % 82;
            x_buckets_parade_g[x] = 87 + (relative_x * 82.0) as usize % 82;
            x_buckets_parade_b[x] = 174 + (relative_x * 82.0) as usize % 82;
        }
    }

    let mut process_pixel = |r: u8, g: u8, b: u8, out_x: usize, orig_x: usize| {
        if do_rgb {
            red_bins[(255 - r as usize) * W + out_x] += 1;
            green_bins[(255 - g as usize) * W + out_x] += 1;
            blue_bins[(255 - b as usize) * W + out_x] += 1;
        }
        if do_luma {
            let l = ((r as u32 * 218 + g as u32 * 732 + b as u32 * 74) >> 10).min(255) as usize;
            luma_bins[(255 - l) * W + out_x] += 1;
        }
        if do_parade {
            parade_bins[(255 - r as usize) * W + x_buckets_parade_r[orig_x]] += 1;
            parade_bins[(255 - g as usize) * W + x_buckets_parade_g[orig_x]] += 1;
            parade_bins[(255 - b as usize) * W + x_buckets_parade_b[orig_x]] += 1;
        }
        if do_vectorscope {
            let r_f = r as f32;
            let g_f = g as f32;
            let b_f = b as f32;

            let mut cb = (-0.1146 * r_f - 0.3854 * g_f + 0.5 * b_f) * 0.836;
            let mut cr = (0.5 * r_f - 0.4542 * g_f - 0.0458 * b_f) * 0.836;

            let dist_sq = cb * cb + cr * cr;
            if dist_sq > 16129.0 {
                let scale = 127.0 / dist_sq.sqrt();
                cb *= scale;
                cr *= scale;
            }

            let vx = (cb + 128.0).clamp(0.0, 255.0) as usize;
            let vy = (128.0 - cr).clamp(0.0, 255.0) as usize;
            vector_bins[vy * W + vx] += 1;
        }
    };

    match image {
        DynamicImage::ImageRgb32F(f32_img) => {
            let raw = f32_img.as_raw();
            let stride = orig_w as usize * 3;
            for y in 0..(orig_h as usize) {
                let row = y * stride;
                for (x, &x_bucket) in x_buckets.iter().enumerate() {
                    let i = row + x * 3;
                    process_pixel(
                        (raw[i].clamp(0.0, 1.0) * 255.0) as u8,
                        (raw[i + 1].clamp(0.0, 1.0) * 255.0) as u8,
                        (raw[i + 2].clamp(0.0, 1.0) * 255.0) as u8,
                        x_bucket,
                        x,
                    );
                }
            }
        }
        _ => {
            let rgb = image.to_rgb8();
            let raw = rgb.as_raw();
            let stride = orig_w as usize * 3;
            for y in 0..(orig_h as usize) {
                let row = y * stride;
                for (x, &x_bucket) in x_buckets.iter().enumerate() {
                    let i = row + x * 3;
                    process_pixel(raw[i], raw[i + 1], raw[i + 2], x_bucket, x);
                }
            }
        }
    }

    let build_lut = |bins: &[u32], do_calc: bool| -> (Vec<u8>, u32) {
        if !do_calc {
            return (vec![0; 1], 0);
        }
        let max_val = *bins.iter().max().unwrap_or(&0);
        if max_val == 0 {
            return (vec![0; 1], 0);
        }
        let scale = 255.0 / (1.0 + max_val as f32).ln();
        let lut = (0..=max_val)
            .map(|v| {
                if v == 0 {
                    0
                } else {
                    ((1.0 + v as f32).ln() * scale) as u8
                }
            })
            .collect();
        (lut, max_val)
    };

    let (lut_r, max_r) = build_lut(&red_bins, do_rgb);
    let (lut_g, max_g) = build_lut(&green_bins, do_rgb);
    let (lut_b, max_b) = build_lut(&blue_bins, do_rgb);
    let (lut_l, max_l) = build_lut(&luma_bins, do_luma);
    let (lut_p, max_p) = build_lut(&parade_bins, do_parade);
    let (lut_v, max_v) = build_lut(&vector_bins, do_vectorscope);

    let pixel_count = W * H;
    let byte_count = pixel_count * 4;

    let mut rgba_rgb = if do_rgb {
        vec![0u8; byte_count]
    } else {
        vec![]
    };
    let mut rgba_luma = if do_luma {
        vec![0u8; byte_count]
    } else {
        vec![]
    };
    let mut rgba_parade = if do_parade {
        vec![0u8; byte_count]
    } else {
        vec![]
    };
    let mut rgba_vector = if do_vectorscope {
        vec![0u8; byte_count]
    } else {
        vec![]
    };

    for i in 0..pixel_count {
        let x = i % W;
        let y = i / W;
        let off = i * 4;

        if do_rgb {
            let r = if red_bins[i] <= max_r {
                lut_r[red_bins[i] as usize]
            } else {
                0
            };
            let g = if green_bins[i] <= max_g {
                lut_g[green_bins[i] as usize]
            } else {
                0
            };
            let b = if blue_bins[i] <= max_b {
                lut_b[blue_bins[i] as usize]
            } else {
                0
            };
            if r > 0 || g > 0 || b > 0 {
                rgba_rgb[off] = r;
                rgba_rgb[off + 1] = g;
                rgba_rgb[off + 2] = b;
                rgba_rgb[off + 3] = r.max(g).max(b);
            }
        }

        if do_luma && luma_bins[i] > 0 && luma_bins[i] <= max_l {
            let l = lut_l[luma_bins[i] as usize];
            rgba_luma[off] = 255;
            rgba_luma[off + 1] = 255;
            rgba_luma[off + 2] = 255;
            rgba_luma[off + 3] = l;
        }

        if do_parade && parade_bins[i] > 0 && parade_bins[i] <= max_p {
            let bright = lut_p[parade_bins[i] as usize];
            if x < 82 {
                rgba_parade[off] = 255;
                rgba_parade[off + 3] = bright;
            } else if (87..169).contains(&x) {
                rgba_parade[off + 1] = 255;
                rgba_parade[off + 3] = bright;
            } else if x >= 174 {
                rgba_parade[off + 2] = 255;
                rgba_parade[off + 3] = bright;
            }
        }

        if do_vectorscope {
            let val = vector_bins[i];

            let dx = x as f32 - 128.0;
            let dy = 128.0 - y as f32;
            let min_d = dx.abs().min(dy.abs());
            let dist = (dx * dx + dy * dy).sqrt();

            if val > 0 && val <= max_v {
                let bright = lut_v[val as usize];

                let y_mid = 128.0;
                rgba_vector[off] = (y_mid + 1.402 * (dy / 0.836)).clamp(0.0, 255.0) as u8;
                rgba_vector[off + 1] = (y_mid - 0.344136 * (dx / 0.836) - 0.714136 * (dy / 0.836))
                    .clamp(0.0, 255.0) as u8;
                rgba_vector[off + 2] = (y_mid + 1.772 * (dx / 0.836)).clamp(0.0, 255.0) as u8;
                rgba_vector[off + 3] = bright;
            } else if min_d <= 1.0 {
                let alpha = (40.0 - min_d * 30.0).clamp(0.0, 255.0) as u8;
                rgba_vector[off] = 255;
                rgba_vector[off + 1] = 255;
                rgba_vector[off + 2] = 255;
                rgba_vector[off + 3] = alpha;
            } else if (dist - 127.0).abs() < 0.8 || (dist - 64.0).abs() < 0.8 {
                rgba_vector[off] = 255;
                rgba_vector[off + 1] = 255;
                rgba_vector[off + 2] = 255;
                rgba_vector[off + 3] = 15;
            } else if dx < 0.0 && dy > 0.0 && (dy + 1.53 * dx).abs() < 1.0 {
                rgba_vector[off] = 255;
                rgba_vector[off + 1] = 200;
                rgba_vector[off + 2] = 150;
                rgba_vector[off + 3] = 120;
            }
        }
    }

    Ok(WaveformData {
        rgb: if do_rgb {
            BASE64.encode(&rgba_rgb)
        } else {
            String::new()
        },
        luma: if do_luma {
            BASE64.encode(&rgba_luma)
        } else {
            String::new()
        },
        parade: if do_parade {
            BASE64.encode(&rgba_parade)
        } else {
            String::new()
        },
        vectorscope: if do_vectorscope {
            BASE64.encode(&rgba_vector)
        } else {
            String::new()
        },
        width: W as u32,
        height: H as u32,
    })
}

/// `face_median_luma`: median luma (0-255) of the dominant detected face, when one
/// exists. Auto must never brighten a face past proper skin exposure — global
/// median-targeting on portraits with dark surroundings would otherwise wash the
/// subject out while "fixing" the background.
pub fn perform_auto_analysis(
    image: &DynamicImage,
    face_median_luma: Option<f64>,
    profile: Option<&crate::app_settings::AutoProfile>,
) -> AutoAdjustmentResults {
    const ANALYSIS_MAX_DIM: u32 = 1024;
    const FACE_TARGET_LUMA: f64 = 165.0;
    const MIN_PROFILE_SAMPLES: u32 = 3;

    // Personalized targets learned from the user's own edits, falling back to the
    // stock constants until enough edited photos exist to learn from.
    let face_target = profile
        .filter(|p| p.face_samples >= MIN_PROFILE_SAMPLES)
        .map(|p| p.face_target_luma)
        .unwrap_or(FACE_TARGET_LUMA);
    let midtone_target = profile
        .filter(|p| p.global_samples >= MIN_PROFILE_SAMPLES)
        .map(|p| p.midtone_target_luma)
        .unwrap_or(128.0);

    const LUMA_R: f32 = 0.2126;
    const LUMA_G: f32 = 0.7152;
    const LUMA_B: f32 = 0.0722;

    const EXPOSURE_MIDPOINT: f64 = 128.0;
    const EXPOSURE_SCALE: f64 = 0.125;
    const WHITE_POINT_HARD_LIMIT: usize = 245;
    const HIGHLIGHT_LUMA_THRESHOLD: usize = 240;
    const CLIPPED_LUMA_THRESHOLD: usize = 250;
    const HIGHLIGHT_PERCENT_THRESHOLD: f64 = 0.02;
    const CLIPPED_PERCENT_THRESHOLD: f64 = 0.005;
    const EXPOSURE_CEILING: f64 = 250.0;

    const TARGET_RANGE: f64 = 220.0;
    const CONTRAST_SCALE: f64 = 10.0;
    const HIGHLIGHT_CONTRAST_REDUCE: f64 = 0.5;

    const SHADOW_LUMA_MAX: usize = 32;
    const SHADOW_PERCENT_THRESHOLD: f64 = 0.05;
    const SHADOW_BOOST_SCALE: f64 = 40.0;
    const SHADOW_MAX: f64 = 50.0;
    const HIGHLIGHT_BOOST_SCALE: f64 = 120.0;
    const HIGHLIGHT_MAX: f64 = 70.0;
    const HIGHLIGHT_RECOVERY_FLOOR: f64 = 30.0;
    const CLIPPED_RECOVERY_SCALE: f64 = 800.0;

    const VIBRANCY_SAT_THRESHOLD: f32 = 0.2;
    const VIBRANCY_SCALE: f64 = 120.0;

    const DEHAZE_RANGE_THRESHOLD: f64 = 120.0;
    const DEHAZE_SAT_THRESHOLD: f32 = 0.15;
    const DEHAZE_SCALE: f64 = 35.0;
    const CLARITY_RANGE_THRESHOLD: f64 = 180.0;
    const CLARITY_SCALE: f64 = 50.0;

    const VIGNETTE_CENTER_LOW: f32 = 0.25;
    const VIGNETTE_CENTER_HIGH: f32 = 0.75;

    const VIGNETTE_SCALE: f64 = 100.0;
    const VIGNETTE_CENTRE_DIFF_THRESHOLD: f32 = 0.05;
    const CENTRE_SCALE: f64 = 100.0;
    const CENTRE_MAX: f64 = 60.0;

    const MID_GRAY: f64 = 128.0;
    const BLACKS_SCALE: f64 = 0.5;
    const WHITES_SCALE: f64 = 0.2;
    const EXPOSURE_OUTPUT_SCALE: f64 = 20.0;
    const BRIGHTNESS_SCALE: f64 = 0.007;

    let analysis_preview = downscale_f32_image(image, ANALYSIS_MAX_DIM, ANALYSIS_MAX_DIM);
    let rgb_image = analysis_preview.to_rgb8();
    let total_pixels = (rgb_image.width() * rgb_image.height()) as f64;

    let (width, height) = rgb_image.dimensions();
    let cx0 = (width as f32 * VIGNETTE_CENTER_LOW) as u32;
    let cx1 = (width as f32 * VIGNETTE_CENTER_HIGH) as u32;
    let cy0 = (height as f32 * VIGNETTE_CENTER_LOW) as u32;
    let cy1 = (height as f32 * VIGNETTE_CENTER_HIGH) as u32;

    let mut luma_hist = vec![0u32; 256];
    let mut mean_saturation = 0.0f32;
    let mut center_sum = 0.0f32;
    let mut edge_sum = 0.0f32;
    let mut center_n = 0u32;
    let mut edge_n = 0u32;

    for (x, y, pixel) in rgb_image.enumerate_pixels() {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;

        let luma_f = LUMA_R * r + LUMA_G * g + LUMA_B * b;
        luma_hist[(luma_f.round() as usize).min(255)] += 1;

        let r_n = r / 255.0;
        let g_n = g / 255.0;
        let b_n = b / 255.0;
        let max_c = r_n.max(g_n).max(b_n);
        let min_c = r_n.min(g_n).min(b_n);
        if max_c > 0.0 {
            let s = (max_c - min_c) / max_c;
            mean_saturation += s;
        }

        let luma_norm = luma_f / 255.0;
        if x >= cx0 && x < cx1 && y >= cy0 && y < cy1 {
            center_sum += luma_norm;
            center_n += 1;
        } else {
            edge_sum += luma_norm;
            edge_n += 1;
        }
    }

    mean_saturation /= total_pixels as f32;

    let percentile = |hist: &Vec<u32>, p: f64| -> usize {
        let target = (total_pixels * p) as u32;
        let mut cumulative = 0u32;
        for (i, &v) in hist.iter().enumerate() {
            cumulative += v;
            if cumulative >= target {
                return i;
            }
        }
        255
    };

    let p1 = percentile(&luma_hist, 0.01);
    let p50 = percentile(&luma_hist, 0.50);
    let p99 = percentile(&luma_hist, 0.99);

    let black_point = p1;
    let white_point = p99;
    let range = (white_point as f64 - black_point as f64).max(1.0);

    let highlight_percent =
        luma_hist[HIGHLIGHT_LUMA_THRESHOLD..256].iter().sum::<u32>() as f64 / total_pixels;
    let clipped_percent =
        luma_hist[CLIPPED_LUMA_THRESHOLD..256].iter().sum::<u32>() as f64 / total_pixels;

    let mut exposure = (midtone_target - p50 as f64) * EXPOSURE_SCALE;

    let highlights_at_risk = white_point > WHITE_POINT_HARD_LIMIT
        || highlight_percent > HIGHLIGHT_PERCENT_THRESHOLD
        || clipped_percent > CLIPPED_PERCENT_THRESHOLD;

    if highlights_at_risk {
        exposure = exposure.min(0.0);
    }

    // Face-aware cap: positive boosts may only take the face up to proper skin
    // brightness, never past it.
    let face_headroom = face_median_luma.map(|fm| (face_target - fm).max(0.0));
    if let Some(headroom) = face_headroom {
        exposure = exposure.min(headroom);
    }

    if white_point as f64 + exposure > EXPOSURE_CEILING {
        exposure = EXPOSURE_CEILING - white_point as f64;
    }

    let mut contrast = 0.0f64;
    if range < TARGET_RANGE {
        contrast = ((TARGET_RANGE / range) - 1.0) * CONTRAST_SCALE;
    }
    if highlight_percent > HIGHLIGHT_PERCENT_THRESHOLD {
        contrast *= HIGHLIGHT_CONTRAST_REDUCE;
    }

    let shadow_percent = luma_hist[0..SHADOW_LUMA_MAX].iter().sum::<u32>() as f64 / total_pixels;

    let mut shadows = 0.0f64;
    if shadow_percent > SHADOW_PERCENT_THRESHOLD {
        shadows = (shadow_percent * SHADOW_BOOST_SCALE).min(SHADOW_MAX);
    }

    let mut highlights = 0.0f64;
    if highlight_percent > HIGHLIGHT_PERCENT_THRESHOLD {
        // Linear scaling alone lands at token values (a few percent of near-white
        // pixels -> -4); when highlights are genuinely at risk, recover with a
        // meaningful floor and grow with the clipped mass.
        let recovery = (highlight_percent * HIGHLIGHT_BOOST_SCALE)
            .max(HIGHLIGHT_RECOVERY_FLOOR + clipped_percent * CLIPPED_RECOVERY_SCALE);
        highlights = -recovery.min(HIGHLIGHT_MAX);
    }

    let mut vibrancy = 0.0f64;
    if mean_saturation < VIBRANCY_SAT_THRESHOLD {
        vibrancy = (VIBRANCY_SAT_THRESHOLD - mean_saturation) as f64 * VIBRANCY_SCALE;
    }

    let mut dehaze = 0.0f64;
    if range < DEHAZE_RANGE_THRESHOLD && mean_saturation < DEHAZE_SAT_THRESHOLD {
        dehaze = (1.0 - range / DEHAZE_RANGE_THRESHOLD) * DEHAZE_SCALE;
    }

    let mut clarity = 0.0f64;
    if range < CLARITY_RANGE_THRESHOLD {
        clarity = (1.0 - range / CLARITY_RANGE_THRESHOLD) * CLARITY_SCALE;
    }

    let mut vignette_amount = 0.0f64;
    let mut centre = 0.0f64;

    if center_n > 0 && edge_n > 0 {
        let c_avg = center_sum / center_n as f32;
        let e_avg = edge_sum / edge_n as f32;

        if e_avg < c_avg {
            let diff = c_avg - e_avg;
            vignette_amount = -(diff as f64 * VIGNETTE_SCALE);

            if diff > VIGNETTE_CENTRE_DIFF_THRESHOLD {
                centre = (diff as f64 * CENTRE_SCALE).min(CENTRE_MAX);
            }
        }
    }

    let mut adjusted_luma_hist = vec![0u32; 256];
    for pixel in rgb_image.pixels() {
        let r = pixel[0] as f64;
        let g = pixel[1] as f64;
        let b = pixel[2] as f64;
        let mut luma = LUMA_R as f64 * r + LUMA_G as f64 * g + LUMA_B as f64 * b;
        luma += exposure;
        luma = (luma - MID_GRAY) * (1.0 + contrast / 100.0) + MID_GRAY;
        adjusted_luma_hist[luma.clamp(0.0, 255.0).round() as usize] += 1;
    }

    let adj_p1 = percentile(&adjusted_luma_hist, 0.01);
    let adj_p50 = percentile(&adjusted_luma_hist, 0.50);
    let adj_p99 = percentile(&adjusted_luma_hist, 0.99);
    let blacks: f64 = -(adj_p1 as f64 * BLACKS_SCALE);
    let whites: f64 = (adj_p99 as f64 - 255.0) * WHITES_SCALE;
    // Brightness (the filmic post-tonemap gain) needs the same highlight guard as
    // exposure: median-targeting on a high-key scene would otherwise re-blow the
    // highlights the exposure guard just protected.
    let mut brightness: f64 = (midtone_target - adj_p50 as f64) * BRIGHTNESS_SCALE;
    if highlights_at_risk {
        brightness = brightness.min(0.0);
    }
    if let Some(headroom) = face_headroom {
        // exposure already consumed part of the face's headroom (both are ~luma-linear;
        // brightness units are luma * BRIGHTNESS_SCALE)
        let remaining = (headroom - exposure.max(0.0)).max(0.0);
        brightness = brightness.min(remaining * BRIGHTNESS_SCALE);
    }

    AutoAdjustmentResults {
        exposure: (exposure / EXPOSURE_OUTPUT_SCALE).clamp(-5.0, 5.0),
        brightness: brightness.clamp(-5.0, 5.0),
        contrast: contrast.clamp(-100.0, 100.0),
        highlights: highlights.clamp(-100.0, 100.0),
        shadows: shadows.clamp(-100.0, 100.0),
        vibrancy: vibrancy.clamp(-100.0, 100.0),
        vignette_amount: vignette_amount.clamp(-100.0, 100.0),
        temperature: 0.0,
        tint: 0.0,
        dehaze: dehaze.clamp(-100.0, 100.0),
        clarity: clarity.clamp(-100.0, 100.0),
        centre: centre.clamp(-100.0, 100.0),
        whites: whites.clamp(-100.0, 100.0),
        blacks: blacks.clamp(-100.0, 100.0),
    }
}

pub fn auto_results_to_json(results: &AutoAdjustmentResults) -> serde_json::Value {
    json!({
        "exposure": results.exposure,
        "brightness": results.brightness,
        "contrast": results.contrast,
        "highlights": results.highlights,
        "shadows": results.shadows,
        "vibrance": results.vibrancy,
        "vignetteAmount": results.vignette_amount,
        "clarity": results.clarity,
        "centré": results.centre,

        "dehaze": results.dehaze,
        "sectionVisibility": {
            "basic": true,
            "color": true,
            "effects": true
        },
        "whites": results.whites,
        "blacks": results.blacks
    })
}

/// Median luma of a region (or the whole image when `rect` is None).
fn luma_median(image: &image::GrayImage, rect: Option<(u32, u32, u32, u32)>) -> Option<f64> {
    let (x0, y0, w, h) = rect.unwrap_or((0, 0, image.width(), image.height()));
    if w < 4 || h < 4 {
        return None;
    }
    let mut hist = [0u32; 256];
    for y in y0..(y0 + h).min(image.height()) {
        for x in x0..(x0 + w).min(image.width()) {
            hist[image.get_pixel(x, y)[0] as usize] += 1;
        }
    }
    let total: u32 = hist.iter().sum();
    if total == 0 {
        return None;
    }
    let mut cumulative = 0u32;
    for (i, &v) in hist.iter().enumerate() {
        cumulative += v;
        if cumulative >= total / 2 {
            return Some(i as f64);
        }
    }
    None
}

fn median_f64(values: &mut Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(values[values.len() / 2])
}

/// Learn the user's personal auto-correct targets from photos they have already
/// edited in `folder`. Cached thumbnails are rendered WITH adjustments applied,
/// so the face / midtone brightness measured there is exactly the output this
/// user chose. Cache-only: photos without a cached edited thumbnail are skipped.
pub async fn learn_auto_profile_from_folder(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    folder: &std::path::Path,
) -> Option<crate::app_settings::AutoProfile> {
    const MIN_SAMPLES: usize = 3;
    const EDIT_KEYS: [&str; 7] = [
        "exposure",
        "brightness",
        "contrast",
        "highlights",
        "shadows",
        "whites",
        "blacks",
    ];

    let thumb_dir = crate::file_management::get_thumb_cache_dir(app_handle).ok()?;
    let face_session = crate::ai_processing::get_or_init_face_model(
        app_handle,
        &state.ai_state,
        &state.ai_init_lock,
    )
    .await
    .ok();

    let mut face_lumas: Vec<f64> = Vec::new();
    let mut global_lumas: Vec<f64> = Vec::new();

    for entry in std::fs::read_dir(folder).ok()?.flatten() {
        let sidecar = entry.path();
        if sidecar.extension().and_then(|e| e.to_str()) != Some("rrdata") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&sidecar) else {
            continue;
        };
        let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let adj = &meta["adjustments"];
        let edited = EDIT_KEYS
            .iter()
            .any(|k| adj[k].as_f64().map(|v| v.abs() > 0.001).unwrap_or(false));
        if !edited {
            continue;
        }

        // image path = sidecar path minus the .rrdata suffix
        let image_path = sidecar.with_extension("");
        let Some(image_path_str) = image_path.to_str() else {
            continue;
        };
        if !image_path.exists() {
            continue;
        }
        let Some(hash) = crate::file_management::get_cache_key_hash(image_path_str) else {
            continue;
        };
        let thumb_path = thumb_dir.join(format!("{}.jpg", hash));
        let Ok(thumb) = image::open(&thumb_path) else {
            continue;
        };
        let gray = thumb.to_luma8();

        let mut counted_face = false;
        if let Some(session) = &face_session {
            if let Ok(faces) = crate::ai_processing::run_face_detection(&thumb, session) {
                if let Some(face) = faces.first() {
                    let x0 = face.x1.max(0.0) as u32;
                    let y0 = face.y1.max(0.0) as u32;
                    let w = (face.x2 - face.x1).max(0.0) as u32;
                    let h = (face.y2 - face.y1).max(0.0) as u32;
                    if let Some(m) = luma_median(&gray, Some((x0, y0, w, h))) {
                        face_lumas.push(m);
                        counted_face = true;
                    }
                }
            }
        }
        if !counted_face {
            if let Some(m) = luma_median(&gray, None) {
                global_lumas.push(m);
            }
        }
    }

    let face_n = face_lumas.len();
    let global_n = global_lumas.len();
    if face_n < MIN_SAMPLES && global_n < MIN_SAMPLES {
        return None;
    }

    let face_target = median_f64(&mut face_lumas)
        .map(|m| m.clamp(140.0, 215.0))
        .unwrap_or(165.0);
    let midtone_target = median_f64(&mut global_lumas)
        .map(|m| m.clamp(105.0, 160.0))
        .unwrap_or(128.0);

    let learned_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Some(crate::app_settings::AutoProfile {
        face_target_luma: face_target,
        midtone_target_luma: midtone_target,
        face_samples: face_n as u32,
        global_samples: global_n as u32,
        learned_at,
    })
}

/// Median luma (0-255) of the dominant (largest) detected face, or None when no
/// face is found or the detector is unavailable (e.g. model not downloaded yet).
async fn detect_face_median_luma(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    image: &DynamicImage,
) -> Option<f64> {
    let face_session = crate::ai_processing::get_or_init_face_model(
        app_handle,
        &state.ai_state,
        &state.ai_init_lock,
    )
    .await
    .ok()?;
    let faces = crate::ai_processing::run_face_detection(image, &face_session).ok()?;
    let face = faces.first()?;
    let (w, h) = (image.width() as f32, image.height() as f32);
    let x0 = face.x1.clamp(0.0, w - 1.0) as u32;
    let y0 = face.y1.clamp(0.0, h - 1.0) as u32;
    let bw = ((face.x2 - face.x1).max(1.0) as u32).min(image.width() - x0);
    let bh = ((face.y2 - face.y1).max(1.0) as u32).min(image.height() - y0);
    if bw < 8 || bh < 8 {
        return None;
    }
    let crop = image.crop_imm(x0, y0, bw, bh).to_luma8();
    let mut hist = [0u32; 256];
    for p in crop.pixels() {
        hist[p[0] as usize] += 1;
    }
    let half = (crop.width() * crop.height()) / 2;
    let mut cumulative = 0u32;
    for (i, &v) in hist.iter().enumerate() {
        cumulative += v;
        if cumulative >= half {
            return Some(i as f64);
        }
    }
    None
}

/// Continuously fold one freshly saved edit into the learned profile (EMA).
/// Called after every sidecar save, once the edited thumbnail is in the cache.
/// Per-photo debounce: slider drags save every few hundred ms; sampling a photo
/// at most once a minute keeps intermediate states from polluting the profile.
pub async fn update_auto_profile_from_image(app_handle: tauri::AppHandle, image_path: String) {
    const EMA_ALPHA: f64 = 0.15;
    const MIN_SECS_BETWEEN_SAMPLES: u64 = 60;

    static LAST_SAMPLED: once_cell::sync::Lazy<
        std::sync::Mutex<std::collections::HashMap<String, u64>>,
    > = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    {
        let mut last = LAST_SAMPLED.lock().unwrap();
        if now.saturating_sub(last.get(&image_path).copied().unwrap_or(0))
            < MIN_SECS_BETWEEN_SAMPLES
        {
            return;
        }
        last.insert(image_path.clone(), now);
    }

    let state = tauri::Manager::state::<AppState>(&app_handle);
    let Ok(thumb_dir) = crate::file_management::get_thumb_cache_dir(&app_handle) else {
        return;
    };
    let Some(hash) = crate::file_management::get_cache_key_hash(&image_path) else {
        return;
    };
    let Ok(thumb) = image::open(thumb_dir.join(format!("{}.jpg", hash))) else {
        return;
    };
    let gray = thumb.to_luma8();

    let face_sample = match crate::ai_processing::get_or_init_face_model(
        &app_handle,
        &state.ai_state,
        &state.ai_init_lock,
    )
    .await
    {
        Ok(session) => crate::ai_processing::run_face_detection(&thumb, &session)
            .ok()
            .and_then(|faces| faces.first().copied())
            .and_then(|f| {
                luma_median(
                    &gray,
                    Some((
                        f.x1.max(0.0) as u32,
                        f.y1.max(0.0) as u32,
                        (f.x2 - f.x1).max(0.0) as u32,
                        (f.y2 - f.y1).max(0.0) as u32,
                    )),
                )
            }),
        Err(_) => None,
    };
    let global_sample = if face_sample.is_none() {
        luma_median(&gray, None)
    } else {
        None
    };

    let Ok(mut settings) = crate::app_settings::load_settings(app_handle.clone()) else {
        return;
    };
    let mut profile = settings
        .auto_profile
        .clone()
        .unwrap_or(crate::app_settings::AutoProfile {
            face_target_luma: 165.0,
            midtone_target_luma: 128.0,
            face_samples: 0,
            global_samples: 0,
            learned_at: 0,
        });

    if let Some(s) = face_sample {
        let s = s.clamp(140.0, 215.0);
        profile.face_target_luma = if profile.face_samples == 0 {
            s
        } else {
            profile.face_target_luma * (1.0 - EMA_ALPHA) + s * EMA_ALPHA
        };
        profile.face_samples += 1;
    } else if let Some(s) = global_sample {
        let s = s.clamp(105.0, 160.0);
        profile.midtone_target_luma = if profile.global_samples == 0 {
            s
        } else {
            profile.midtone_target_luma * (1.0 - EMA_ALPHA) + s * EMA_ALPHA
        };
        profile.global_samples += 1;
    } else {
        return;
    }
    profile.learned_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    log::info!(
        "[auto-profile] updated from edit: face {:.0} (n={}), midtone {:.0} (n={})",
        profile.face_target_luma,
        profile.face_samples,
        profile.midtone_target_luma,
        profile.global_samples
    );
    settings.auto_profile = Some(profile);
    let _ = crate::app_settings::save_settings(settings, app_handle);
}

#[tauri::command]
pub async fn calculate_auto_adjustments(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let (original_image, image_path) = {
        let guard = state.original_image.lock().unwrap();
        let loaded = guard
            .as_ref()
            .ok_or("No image loaded for auto adjustments")?;
        (loaded.image.clone(), loaded.path.clone())
    };

    let settings = crate::app_settings::load_settings(app_handle.clone()).unwrap_or_default();
    let profile = settings.auto_profile.clone();

    // Bootstrap: with no learned profile yet, scan the current folder's edited
    // thumbnails in the background so the NEXT auto benefits. Ongoing learning
    // happens incrementally on every edit save.
    if profile.is_none() {
        if let Some(folder) = std::path::Path::new(&image_path)
            .parent()
            .map(|p| p.to_path_buf())
        {
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = tauri::Manager::state::<AppState>(&handle);
                if let Some(p) = learn_auto_profile_from_folder(&handle, &state, &folder).await {
                    if let Ok(mut s) = crate::app_settings::load_settings(handle.clone()) {
                        log::info!(
                            "[auto-profile] bootstrapped from folder: face {:.0} (n={}), midtone {:.0} (n={})",
                            p.face_target_luma,
                            p.face_samples,
                            p.midtone_target_luma,
                            p.global_samples
                        );
                        s.auto_profile = Some(p);
                        let _ = crate::app_settings::save_settings(s, handle);
                    }
                }
            });
        }
    }

    let face_median = detect_face_median_luma(&app_handle, &state, &original_image).await;
    let results = perform_auto_analysis(&original_image, face_median, profile.as_ref());

    Ok(auto_results_to_json(&results))
}
