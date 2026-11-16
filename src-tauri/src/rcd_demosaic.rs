//! RCD ("Ratio Corrected Demosaicing", v2.3 by Luis Sanz Rodríguez) for 2x2
//! RGB Bayer sensors — the default demosaic in darktable and RawTherapee.
//!
//! Replaces rawler's PPG for full-quality decodes. PPG invents magenta/green
//! colour on fine diagonal textures (woven cloth: twill, sharkskin), which the
//! later chroma noise reduction only partly removes; shrunk to screen size the
//! residue reads as a purple cast. RCD estimates green from colour *ratios*
//! steered by directional statistics and interpolates red/blue on colour
//! differences, which keeps those textures neutral.
//!
//! Works on white-balanced data (each CFA sample is multiplied by its channel's
//! `scale` first, as RCD expects) and divides the scale back out, so the output
//! drops into the same place in the pipeline as rawler's PPG output.
//!
//! Processed in overlapping tiles (`TILE` + `BORDER` on each side) so each
//! worker needs only a few MB beyond the output image. Image edges are filled by
//! mirroring, which keeps the Bayer phase (x -> -x preserves parity).

use rawler::{CFA, imgop::Rect};
use rayon::prelude::*;

const TILE: usize = 256;
/// Deepest neighbourhood any step reads (R/B at green needs rows +-3 of data
/// computed from rows +-7) — the core of a padded tile is exactly valid.
const BORDER: usize = 10;
const EPS: f32 = 1e-5;
const EPS_SQ: f32 = 1e-10;

#[inline]
fn intp(a: f32, b: f32, c: f32) -> f32 {
    a * (b - c) + c
}

#[inline]
fn sq(v: f32) -> f32 {
    v * v
}

/// Mirror a coordinate into `0..len` without repeating the edge sample.
#[inline]
fn reflect(v: isize, len: usize) -> usize {
    let len = len as isize;
    if len == 1 {
        return 0;
    }
    let period = 2 * (len - 1);
    let mut v = v.rem_euclid(period);
    if v >= len {
        v = period - v;
    }
    v as usize
}

/// Demosaic the `roi` of a single-channel Bayer frame `raw` (`raw_width` wide).
/// `scale[c]` brings channel `c` (0 = R, 1 = G, 2 = B) to white-balanced, ~1.0
/// white; the output is in the input's units (scale divided back out).
pub fn demosaic(raw: &[f32], raw_width: usize, cfa: &CFA, roi: Rect, scale: [f32; 3]) -> Vec<[f32; 3]> {
    let (w, h) = (roi.width(), roi.height());
    let (rx, ry) = (roi.x(), roi.y());
    let cfa = cfa.shift(rx, ry);
    let pattern: [[usize; 2]; 2] = [
        [cfa.color_at(0, 0), cfa.color_at(0, 1)],
        [cfa.color_at(1, 0), cfa.color_at(1, 1)],
    ];
    let inv_scale = scale.map(|s| if s > 0.0 { 1.0 / s } else { 1.0 });

    let mut out = vec![[0.0f32; 3]; w * h];
    out.par_chunks_mut(TILE * w).enumerate().for_each(|(band, rows)| {
        let y0 = band * TILE;
        let th = rows.len() / w;
        let mut scratch = Scratch::default();
        let mut x0 = 0;
        while x0 < w {
            let tw = TILE.min(w - x0);
            scratch.run(raw, raw_width, (rx, ry), (w, h), (x0, y0), (tw, th), &pattern, scale);
            let pw = tw + 2 * BORDER;
            for y in 0..th {
                let src = (y + BORDER) * pw + BORDER;
                let dst = &mut rows[y * w + x0..y * w + x0 + tw];
                for (x, px) in dst.iter_mut().enumerate() {
                    let i = src + x;
                    *px = [
                        scratch.rgb[0][i].max(0.0) * inv_scale[0],
                        scratch.rgb[1][i].max(0.0) * inv_scale[1],
                        scratch.rgb[2][i].max(0.0) * inv_scale[2],
                    ];
                }
            }
            x0 += tw;
        }
    });
    out
}

/// Per-worker buffers for one padded tile, reused across tiles.
#[derive(Default)]
struct Scratch {
    color: Vec<u8>,
    cfa: Vec<f32>,
    rgb: [Vec<f32>; 3],
    vh_dir: Vec<f32>,
    lpf: Vec<f32>,
    hpf_a: Vec<f32>,
    hpf_b: Vec<f32>,
    pq_dir: Vec<f32>,
}

impl Scratch {
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        raw: &[f32],
        raw_width: usize,
        (rx, ry): (usize, usize),
        (w, h): (usize, usize),
        (x0, y0): (usize, usize),
        (tw, th): (usize, usize),
        pattern: &[[usize; 2]; 2],
        scale: [f32; 3],
    ) {
        let pw = tw + 2 * BORDER;
        let ph = th + 2 * BORDER;
        let n = pw * ph;
        let [r, g, b] = &mut self.rgb;
        for buf in [
            &mut self.cfa,
            r,
            g,
            b,
            &mut self.vh_dir,
            &mut self.lpf,
            &mut self.hpf_a,
            &mut self.hpf_b,
            &mut self.pq_dir,
        ] {
            buf.clear();
            buf.resize(n, 0.0);
        }
        self.color.clear();
        self.color.resize(n, 0);

        // Load the padded tile, white balanced; seed each pixel's own channel.
        for py in 0..ph {
            let gy = y0 as isize + py as isize - BORDER as isize;
            let sy = reflect(gy, h);
            let row = &raw[(ry + sy) * raw_width + rx..];
            for px in 0..pw {
                let gx = x0 as isize + px as isize - BORDER as isize;
                let sx = reflect(gx, w);
                let c = pattern[sy & 1][sx & 1];
                let v = (row[sx] * scale[c]).max(0.0);
                let i = py * pw + px;
                self.color[i] = c as u8;
                self.cfa[i] = v;
                self.rgb[c][i] = v;
            }
        }

        let (w1, w2, w3, w4) = (pw, 2 * pw, 3 * pw, 4 * pw);
        let cfa = &self.cfa;
        let color = &self.color;
        let is_green = |i: usize| color[i] == 1;

        // Step 1: vertical / horizontal discrimination from squared colour-difference high-pass.
        for row in 3..ph - 3 {
            for col in 3..pw - 3 {
                let i = row * pw + col;
                self.hpf_a[i] = sq((cfa[i - w3] - cfa[i - w1] - cfa[i + w1] + cfa[i + w3])
                    - 3.0 * (cfa[i - w2] + cfa[i + w2])
                    + 6.0 * cfa[i]);
                self.hpf_b[i] = sq((cfa[i - 3] - cfa[i - 1] - cfa[i + 1] + cfa[i + 3])
                    - 3.0 * (cfa[i - 2] + cfa[i + 2])
                    + 6.0 * cfa[i]);
            }
        }
        for row in 4..ph - 4 {
            for col in 4..pw - 4 {
                let i = row * pw + col;
                let v = (self.hpf_a[i - w1] + self.hpf_a[i] + self.hpf_a[i + w1]).max(EPS_SQ);
                let hz = (self.hpf_b[i - 1] + self.hpf_b[i] + self.hpf_b[i + 1]).max(EPS_SQ);
                self.vh_dir[i] = v / (v + hz);
            }
        }

        // Step 2: low-pass of the local R/G/B samples at red and blue sites.
        for row in 1..ph - 1 {
            for col in 1..pw - 1 {
                let i = row * pw + col;
                if is_green(i) {
                    continue;
                }
                self.lpf[i] = cfa[i]
                    + 0.5 * (cfa[i - w1] + cfa[i + w1] + cfa[i - 1] + cfa[i + 1])
                    + 0.25 * (cfa[i - w1 - 1] + cfa[i - w1 + 1] + cfa[i + w1 - 1] + cfa[i + w1 + 1]);
            }
        }

        // Step 3: green at red and blue sites, from ratio-corrected cardinal estimates.
        for row in 5..ph - 5 {
            for col in 5..pw - 5 {
                let i = row * pw + col;
                if is_green(i) {
                    continue;
                }
                let ci = cfa[i];
                let n_grad = EPS
                    + (cfa[i - w1] - cfa[i + w1]).abs()
                    + (ci - cfa[i - w2]).abs()
                    + (cfa[i - w1] - cfa[i - w3]).abs()
                    + (cfa[i - w2] - cfa[i - w4]).abs();
                let s_grad = EPS
                    + (cfa[i - w1] - cfa[i + w1]).abs()
                    + (ci - cfa[i + w2]).abs()
                    + (cfa[i + w1] - cfa[i + w3]).abs()
                    + (cfa[i + w2] - cfa[i + w4]).abs();
                let w_grad = EPS
                    + (cfa[i - 1] - cfa[i + 1]).abs()
                    + (ci - cfa[i - 2]).abs()
                    + (cfa[i - 1] - cfa[i - 3]).abs()
                    + (cfa[i - 2] - cfa[i - 4]).abs();
                let e_grad = EPS
                    + (cfa[i - 1] - cfa[i + 1]).abs()
                    + (ci - cfa[i + 2]).abs()
                    + (cfa[i + 1] - cfa[i + 3]).abs()
                    + (cfa[i + 2] - cfa[i + 4]).abs();

                let lpfi = self.lpf[i];
                let n_est = cfa[i - w1] * (lpfi + lpfi) / (EPS + lpfi + self.lpf[i - w2]);
                let s_est = cfa[i + w1] * (lpfi + lpfi) / (EPS + lpfi + self.lpf[i + w2]);
                let w_est = cfa[i - 1] * (lpfi + lpfi) / (EPS + lpfi + self.lpf[i - 2]);
                let e_est = cfa[i + 1] * (lpfi + lpfi) / (EPS + lpfi + self.lpf[i + 2]);

                let v_est = (s_grad * n_est + n_grad * s_est) / (n_grad + s_grad);
                let h_est = (w_grad * e_est + e_grad * w_est) / (e_grad + w_grad);

                let central = self.vh_dir[i];
                let around = 0.25
                    * (self.vh_dir[i - w1 - 1] + self.vh_dir[i - w1 + 1] + self.vh_dir[i + w1 - 1] + self.vh_dir[i + w1 + 1]);
                let disc = if (0.5 - central).abs() < (0.5 - around).abs() { around } else { central };
                self.rgb[1][i] = intp(disc, h_est, v_est);
            }
        }

        // Step 4.0/4.1: diagonal (P = NW-SE, Q = NE-SW) discrimination at red and blue sites.
        for row in 3..ph - 3 {
            for col in 3..pw - 3 {
                let i = row * pw + col;
                if is_green(i) {
                    continue;
                }
                self.hpf_a[i] = sq((cfa[i - w3 - 3] - cfa[i - w1 - 1] - cfa[i + w1 + 1] + cfa[i + w3 + 3])
                    - 3.0 * (cfa[i - w2 - 2] + cfa[i + w2 + 2])
                    + 6.0 * cfa[i]);
                self.hpf_b[i] = sq((cfa[i - w3 + 3] - cfa[i - w1 + 1] - cfa[i + w1 - 1] + cfa[i + w3 - 3])
                    - 3.0 * (cfa[i - w2 + 2] + cfa[i + w2 - 2])
                    + 6.0 * cfa[i]);
            }
        }
        for row in 4..ph - 4 {
            for col in 4..pw - 4 {
                let i = row * pw + col;
                if is_green(i) {
                    continue;
                }
                let p = (self.hpf_a[i - w1 - 1] + self.hpf_a[i] + self.hpf_a[i + w1 + 1]).max(EPS_SQ);
                let q = (self.hpf_b[i - w1 + 1] + self.hpf_b[i] + self.hpf_b[i + w1 - 1]).max(EPS_SQ);
                self.pq_dir[i] = p / (p + q);
            }
        }

        // Step 4.2: red at blue sites and blue at red sites, on diagonal colour differences.
        for row in 7..ph - 7 {
            for col in 7..pw - 7 {
                let i = row * pw + col;
                if is_green(i) {
                    continue;
                }
                let c = 2 - color[i] as usize;
                let central = self.pq_dir[i];
                let around = 0.25
                    * (self.pq_dir[i - w1 - 1] + self.pq_dir[i - w1 + 1] + self.pq_dir[i + w1 - 1] + self.pq_dir[i + w1 + 1]);
                let disc = if (0.5 - central).abs() < (0.5 - around).abs() { around } else { central };

                let (rc, g) = (&self.rgb[c], &self.rgb[1]);
                let nw_grad = EPS
                    + (rc[i - w1 - 1] - rc[i + w1 + 1]).abs()
                    + (rc[i - w1 - 1] - rc[i - w3 - 3]).abs()
                    + (g[i] - g[i - w2 - 2]).abs();
                let ne_grad = EPS
                    + (rc[i - w1 + 1] - rc[i + w1 - 1]).abs()
                    + (rc[i - w1 + 1] - rc[i - w3 + 3]).abs()
                    + (g[i] - g[i - w2 + 2]).abs();
                let sw_grad = EPS
                    + (rc[i - w1 + 1] - rc[i + w1 - 1]).abs()
                    + (rc[i + w1 - 1] - rc[i + w3 - 3]).abs()
                    + (g[i] - g[i + w2 - 2]).abs();
                let se_grad = EPS
                    + (rc[i - w1 - 1] - rc[i + w1 + 1]).abs()
                    + (rc[i + w1 + 1] - rc[i + w3 + 3]).abs()
                    + (g[i] - g[i + w2 + 2]).abs();

                let nw_est = rc[i - w1 - 1] - g[i - w1 - 1];
                let ne_est = rc[i - w1 + 1] - g[i - w1 + 1];
                let sw_est = rc[i + w1 - 1] - g[i + w1 - 1];
                let se_est = rc[i + w1 + 1] - g[i + w1 + 1];

                let p_est = (nw_grad * se_est + se_grad * nw_est) / (nw_grad + se_grad);
                let q_est = (ne_grad * sw_est + sw_grad * ne_est) / (ne_grad + sw_grad);
                let value = g[i] + intp(disc, q_est, p_est);
                self.rgb[c][i] = value;
            }
        }

        // Step 4.3: red and blue at green sites, on cardinal colour differences.
        for row in 10..ph - 10 {
            for col in 10..pw - 10 {
                let i = row * pw + col;
                if !is_green(i) {
                    continue;
                }
                let central = self.vh_dir[i];
                let around = 0.25
                    * (self.vh_dir[i - w1 - 1] + self.vh_dir[i - w1 + 1] + self.vh_dir[i + w1 - 1] + self.vh_dir[i + w1 + 1]);
                let disc = if (0.5 - central).abs() < (0.5 - around).abs() { around } else { central };

                let g = &self.rgb[1];
                let gi = g[i];
                let n1 = EPS + (gi - g[i - w2]).abs();
                let s1 = EPS + (gi - g[i + w2]).abs();
                let w1g = EPS + (gi - g[i - 2]).abs();
                let e1 = EPS + (gi - g[i + 2]).abs();
                let (g_n, g_s, g_w, g_e) = (g[i - w1], g[i + w1], g[i - 1], g[i + 1]);

                for c in [0usize, 2] {
                    let ch = &self.rgb[c];
                    let sn_abs = (ch[i - w1] - ch[i + w1]).abs();
                    let ew_abs = (ch[i - 1] - ch[i + 1]).abs();
                    let n_grad = n1 + sn_abs + (ch[i - w1] - ch[i - w3]).abs();
                    let s_grad = s1 + sn_abs + (ch[i + w1] - ch[i + w3]).abs();
                    let w_grad = w1g + ew_abs + (ch[i - 1] - ch[i - 3]).abs();
                    let e_grad = e1 + ew_abs + (ch[i + 1] - ch[i + 3]).abs();

                    let n_est = ch[i - w1] - g_n;
                    let s_est = ch[i + w1] - g_s;
                    let w_est = ch[i - 1] - g_w;
                    let e_est = ch[i + 1] - g_e;

                    let v_est = (n_grad * s_est + s_grad * n_est) / (n_grad + s_grad);
                    let h_est = (e_grad * w_est + w_grad * e_est) / (e_grad + w_grad);
                    let value = gi + intp(disc, h_est, v_est);
                    self.rgb[c][i] = value;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawler::imgop::{Dim2, Point};

    fn mosaic(w: usize, h: usize, f: impl Fn(usize, usize) -> [f32; 3]) -> (Vec<f32>, CFA) {
        let cfa = CFA::new("RGGB");
        let raw = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y)[cfa.color_at(y, x)])
            .collect();
        (raw, cfa)
    }

    #[test]
    fn flat_grey_stays_grey() {
        let (w, h) = (300, 270);
        let (raw, cfa) = mosaic(w, h, |_, _| [0.4, 0.4, 0.4]);
        let roi = Rect::new(Point::zero(), Dim2::new(w, h));
        let out = demosaic(&raw, w, &cfa, roi, [1.0; 3]);
        for p in &out {
            for c in p {
                assert!((c - 0.4).abs() < 1e-3, "{p:?}");
            }
        }
    }

    #[test]
    fn white_balance_scale_round_trips() {
        let (w, h) = (64, 64);
        // A flat patch in camera units that is grey once balanced by [2, 1, 1.5].
        let (raw, cfa) = mosaic(w, h, |_, _| [0.2, 0.4, 0.4 / 1.5]);
        let roi = Rect::new(Point::zero(), Dim2::new(w, h));
        let out = demosaic(&raw, w, &cfa, roi, [2.0, 1.0, 1.5]);
        let p = out[32 * w + 32];
        assert!((p[0] - 0.2).abs() < 1e-3 && (p[1] - 0.4).abs() < 1e-3 && (p[2] - 0.4 / 1.5).abs() < 1e-3, "{p:?}");
    }

    fn mean_error(out: &[[f32; 3]], w: usize, h: usize, truth: impl Fn(usize, usize) -> [f32; 3]) -> f64 {
        let mut sum = 0.0f64;
        let mut count = 0;
        for y in 16..h - 16 {
            for x in 16..w - 16 {
                let (p, t) = (out[y * w + x], truth(x, y));
                sum += (0..3).map(|c| (p[c] - t[c]).abs() as f64).sum::<f64>() / 3.0;
                count += 1;
            }
        }
        sum / count as f64
    }

    #[test]
    fn smooth_colour_gradient_is_reproduced() {
        let (w, h) = (200, 180);
        let truth = |x: usize, y: usize| [0.2 + 0.002 * x as f32, 0.4, 0.3 + 0.001 * y as f32];
        let (raw, cfa) = mosaic(w, h, truth);
        let roi = Rect::new(Point::zero(), Dim2::new(w, h));
        let out = demosaic(&raw, w, &cfa, roi, [1.0; 3]);
        let err = mean_error(&out, w, h, truth);
        assert!(err < 2e-3, "mean error {err}");
    }

    /// A fine neutral diagonal texture (~5 px period, like twill cloth but still
    /// resolvable by a Bayer sensor) must not come out coloured.
    #[test]
    fn fine_neutral_texture_stays_neutral() {
        let (w, h) = (256, 256);
        let truth = |x: usize, y: usize| {
            let v = 0.35 + 0.2 * (((x + y) as f32) * 1.25).sin();
            [v, v, v]
        };
        let (raw, cfa) = mosaic(w, h, truth);
        let roi = Rect::new(Point::zero(), Dim2::new(w, h));
        let out = demosaic(&raw, w, &cfa, roi, [1.0; 3]);
        let mut chroma = 0.0f64;
        let mut count = 0;
        for y in 16..h - 16 {
            for x in 16..w - 16 {
                let [r, g, b] = out[y * w + x];
                chroma += ((r - g).abs() + (b - g).abs()) as f64;
                count += 1;
            }
        }
        let rcd = chroma / count as f64;

        // The same mosaic through rawler's PPG, which RCD replaces.
        use rawler::imgop::sensor::bayer::{Demosaic, ppg::PPGDemosaic};
        use rawler::{cfa::PlaneColor, pixarray::PixF32};
        let pixels = PixF32::new_with(raw.clone(), w, h);
        let ppg = PPGDemosaic::new().demosaic(&pixels, &cfa, &PlaneColor::new("RGB"), roi);
        let mut chroma = 0.0f64;
        for y in 16..h - 16 {
            for x in 16..w - 16 {
                let [r, g, b] = ppg.data[y * w + x];
                chroma += ((r - g).abs() + (b - g).abs()) as f64;
            }
        }
        let ppg = chroma / count as f64;
        eprintln!("mean false colour on a 5px diagonal texture: rcd {rcd:.4}, ppg {ppg:.4}");
        assert!(rcd < ppg * 0.7, "rcd {rcd} not clearly better than ppg {ppg}");
    }

    /// Side-by-side PPG vs RCD crops from a real RAW, for eyeballing:
    /// `RCD_SAMPLE=<file.CR2> RCD_CROP=x,y,w,h cargo test --lib rcd_demosaic::tests::compare_on_real_raw -- --ignored`
    /// writes `<file>.ppg.png` and `<file>.rcd.png` (white balanced, camera RGB, sRGB gamma).
    #[test]
    #[ignore]
    fn compare_on_real_raw() {
        use rawler::decoders::RawDecodeParams;
        use rawler::imgop::sensor::bayer::{Demosaic, ppg::PPGDemosaic};
        use rawler::{pixarray::PixF32, rawimage::RawPhotometricInterpretation, rawsource::RawSource};

        let path = std::env::var("RCD_SAMPLE").expect("set RCD_SAMPLE");
        let crop: Vec<usize> = std::env::var("RCD_CROP")
            .unwrap_or_else(|_| "1000,1500,800,600".into())
            .split(',')
            .map(|v| v.trim().parse().unwrap())
            .collect();
        let source = RawSource::new(std::path::Path::new(&path)).unwrap();
        let decoder = rawler::get_decoder(&source).unwrap();
        let mut raw = decoder.raw_image(&source, &RawDecodeParams::default(), false).unwrap();
        raw.apply_scaling().unwrap();
        let RawPhotometricInterpretation::Cfa(config) = &raw.photometric else { panic!("not a CFA raw") };
        let wb = [raw.wb_coeffs[0], raw.wb_coeffs[1], raw.wb_coeffs[2]];
        let mosaic = PixF32::new_with(raw.data.as_f32().into_owned(), raw.width, raw.height);
        let active = raw.active_area.unwrap_or(mosaic.rect());
        // Demosaic just the crop (plus margin) so both run on identical input.
        let margin = 16;
        let roi = Rect::new(
            Point::new(active.x() + crop[0] - margin, active.y() + crop[1] - margin),
            Dim2::new(crop[2] + 2 * margin, crop[3] + 2 * margin),
        );
        let peak = {
            let mut v: Vec<f32> = mosaic.data.iter().step_by(97).copied().collect();
            v.sort_by(f32::total_cmp);
            v[v.len() * 999 / 1000].max(1e-9)
        };
        let rcd = demosaic(&mosaic.data, mosaic.width, &config.cfa, roi, wb.map(|c| c / peak));
        let ppg = PPGDemosaic::new().demosaic(&mosaic, &config.cfa, &config.colors, roi);

        let save = |pixels: &[[f32; 3]], suffix: &str| {
            let (w, h) = (crop[2], crop[3]);
            let mut img = image::RgbImage::new(w as u32, h as u32);
            for y in 0..h {
                for x in 0..w {
                    let p = pixels[(y + margin) * roi.width() + x + margin];
                    let px = std::array::from_fn(|c| {
                        let v = (p[c] * wb[c] / peak * 0.8).clamp(0.0, 1.0);
                        let s = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
                        (s * 255.0).round() as u8
                    });
                    img.put_pixel(x as u32, y as u32, image::Rgb(px));
                }
            }
            img.save(format!("{path}.{suffix}.png")).unwrap();
        };
        save(&ppg.data, "ppg");
        save(&rcd, "rcd");
    }

    #[test]
    fn honours_roi_offset_and_odd_sizes() {
        let (w, h) = (101, 77);
        let (raw, cfa) = mosaic(w, h, |_, _| [0.3, 0.3, 0.3]);
        let roi = Rect::new(Point::new(3, 1), Dim2::new(95, 70));
        let out = demosaic(&raw, w, &cfa, roi, [1.0; 3]);
        assert_eq!(out.len(), 95 * 70);
        assert!(out.iter().all(|p| p.iter().all(|c| (c - 0.3).abs() < 1e-3)));
    }
}
