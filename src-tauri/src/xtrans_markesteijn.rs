//! Markesteijn demosaicing for Fujifilm X-Trans (6x6 CFA), 1-pass variant —
//! ported from Frank Markesteijn's algorithm as published in dcraw's
//! `xtrans_interpolate` and used by darktable / RawTherapee.
//!
//! Replaces rawler's simple four-pass X-Trans interpolation for full-quality
//! decodes. That one puts purple/green mottling and a crosshatch on fine woven
//! cloth; Markesteijn interpolates green along four directions over the green
//! hexagon around each red/blue site (clamped to its neighbours' range), fills
//! red/blue on colour differences per direction, then keeps, per pixel, the
//! directions whose result is most homogeneous in CIELab.
//!
//! Like `rcd_demosaic`, it works on white-balanced samples (`scale`) and divides
//! the scale back out, and runs in overlapping tiles with a mirrored border.
//! The mirror is shifted to keep the 6x6 phase, since X-Trans is not symmetric.

use rawler::{CFA, imgop::Rect};
use rayon::prelude::*;

/// Output pixels per tile edge.
const CORE: usize = 240;
/// Padding around each tile: the final step reads up to `TOP + 8` px in from the
/// patch edge (and green interpolation up to 6 rows/cols from `TOP`).
const BORDER: usize = 16;
const TOP: usize = 6;
const NDIR: usize = 4;

/// Mirror `v` into `0..len`, then shift by under one period so the X-Trans phase
/// (mod 6) is kept.
fn reflect6(v: isize, len: usize) -> usize {
    let n = len as isize;
    if (0..n).contains(&v) {
        return v as usize;
    }
    let m = if v < 0 { -v } else { 2 * (n - 1) - v };
    let mut x = m + (v - m).rem_euclid(6);
    while x >= n {
        x -= 6;
    }
    while x < 0 {
        x += 6;
    }
    x as usize
}

/// Demosaic the `roi` of a single-channel X-Trans frame `raw` (`raw_width` wide).
/// `scale[c]` brings channel `c` (0 = R, 1 = G, 2 = B) to white-balanced, ~1.0
/// white; the output is in the input's units.
pub fn demosaic(raw: &[f32], raw_width: usize, cfa: &CFA, roi: Rect, scale: [f32; 3]) -> Vec<[f32; 3]> {
    let (w, h) = (roi.width(), roi.height());
    let (rx, ry) = (roi.x(), roi.y());
    let cfa = cfa.shift(rx, ry);
    let pattern: [[u8; 6]; 6] = std::array::from_fn(|r| std::array::from_fn(|c| cfa.color_at(r, c) as u8));
    let inv_scale = scale.map(|s| if s > 0.0 { 1.0 / s } else { 1.0 });

    let mut out = vec![[0.0f32; 3]; w * h];
    if w < 12 || h < 12 {
        // Too small to mirror with the 6x6 phase; not a real photo anyway.
        for y in 0..h {
            for x in 0..w {
                let v = raw[(ry + y) * raw_width + rx + x];
                out[y * w + x] = [v; 3];
            }
        }
        return out;
    }

    out.par_chunks_mut(CORE * w).enumerate().for_each(|(band, rows)| {
        let y0 = band * CORE;
        let th = rows.len() / w;
        let mut tile = Tile::default();
        let mut x0 = 0;
        while x0 < w {
            let tw = CORE.min(w - x0);
            tile.run(raw, raw_width, (rx, ry), (w, h), (x0, y0), (tw, th), &pattern, scale);
            let pw = tw + 2 * BORDER;
            for y in 0..th {
                for x in 0..tw {
                    let p = tile.result[(y + BORDER) * pw + x + BORDER];
                    rows[y * w + x0 + x] = std::array::from_fn(|c| p[c].max(0.0) * inv_scale[c]);
                }
            }
            x0 += tw;
        }
    });
    out
}

/// sRGB→XYZ (D65) with rows normalised by the D65 white, as dcraw's `cielab`
/// does with an identity camera matrix — only used to judge homogeneity.
const XYZ_RGB: [[f32; 3]; 3] = {
    const M: [[f32; 3]; 3] = [
        [0.412453, 0.357580, 0.180423],
        [0.212671, 0.715160, 0.072169],
        [0.019334, 0.119193, 0.950227],
    ];
    const W: [f32; 3] = [0.950456, 1.0, 1.088754];
    [
        [M[0][0] / W[0], M[0][1] / W[0], M[0][2] / W[0]],
        [M[1][0] / W[1], M[1][1] / W[1], M[1][2] / W[1]],
        [M[2][0] / W[2], M[2][1] / W[2], M[2][2] / W[2]],
    ]
};

#[inline]
fn lab_f(t: f32) -> f32 {
    let t = t.max(0.0);
    if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 }
}

/// dcraw's CIELab scaling (x64), so its derivative weights apply unchanged.
#[inline]
fn cielab(rgb: [f32; 3]) -> [f32; 3] {
    let xyz: [f32; 3] =
        std::array::from_fn(|i| lab_f(XYZ_RGB[i][0] * rgb[0] + XYZ_RGB[i][1] * rgb[1] + XYZ_RGB[i][2] * rgb[2]));
    [
        64.0 * (116.0 * xyz[1] - 16.0),
        64.0 * 500.0 * (xyz[0] - xyz[1]),
        64.0 * 200.0 * (xyz[1] - xyz[2]),
    ]
}

#[inline]
fn sq(v: f32) -> f32 {
    v * v
}

#[derive(Default)]
struct Tile {
    /// Per pixel: [R, G, B, Gmax]. Own colour holds the sample; at red/blue sites
    /// [1] holds the hexagon's green minimum and [3] its maximum.
    img: Vec<[f32; 4]>,
    rgb: [Vec<[f32; 3]>; NDIR],
    lab: Vec<[f32; 3]>,
    drv: [Vec<f32>; NDIR],
    homo: [Vec<u8>; NDIR],
    result: Vec<[f32; 3]>,
}

impl Tile {
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        raw: &[f32],
        raw_width: usize,
        (rx, ry): (usize, usize),
        (w, h): (usize, usize),
        (x0, y0): (usize, usize),
        (tw, th): (usize, usize),
        pattern: &[[u8; 6]; 6],
        scale: [f32; 3],
    ) {
        let pw = tw + 2 * BORDER;
        let ph = th + 2 * BORDER;
        let n = pw * ph;
        let (oy, ox) = (y0 as isize - BORDER as isize, x0 as isize - BORDER as isize);
        let fcol = |r: isize, c: isize| pattern[(oy + r).rem_euclid(6) as usize][(ox + c).rem_euclid(6) as usize] as usize;
        let spw = pw as isize;

        // Load the padded tile, white balanced.
        self.img.clear();
        self.img.resize(n, [0.0; 4]);
        for py in 0..ph {
            let sy = reflect6(oy + py as isize, h);
            let row = &raw[(ry + sy) * raw_width + rx..];
            for px in 0..pw {
                let sx = reflect6(ox + px as isize, w);
                let c = fcol(py as isize, px as isize);
                debug_assert_eq!(c, pattern[sy % 6][sx % 6] as usize);
                self.img[py * pw + px][c] = (row[sx] * scale[c]).max(0.0);
            }
        }

        // Map a green hexagon around each non-green pixel and vice versa.
        const ORTH: [isize; 12] = [1, 0, 0, 1, -1, 0, 0, -1, 1, 0, 0, 1];
        const PATT: [[isize; 16]; 2] = [
            [0, 1, 0, -1, 2, 0, -1, 0, 1, 1, 1, -1, 0, 0, 0, 0],
            [0, 1, 0, -2, 1, 0, -2, 0, 1, 1, -2, -2, 1, -1, -1, 1],
        ];
        let mut allhex = [[[0isize; 8]; 3]; 3];
        let (mut sgrow, mut sgcol) = (0isize, 0isize);
        for row in 0..3isize {
            for col in 0..3isize {
                let g = (fcol(row, col) == 1) as usize;
                let mut ng = 0usize;
                for d in (0..10).step_by(2) {
                    if fcol(row + ORTH[d], col + ORTH[d + 2]) == 1 {
                        ng = 0;
                    } else {
                        ng += 1;
                    }
                    if ng == 4 {
                        sgrow = row;
                        sgcol = col;
                    }
                    if ng == g + 1 {
                        for c in 0..8 {
                            let v = ORTH[d] * PATT[g][c * 2] + ORTH[d + 1] * PATT[g][c * 2 + 1];
                            let hh = ORTH[d + 2] * PATT[g][c * 2] + ORTH[d + 3] * PATT[g][c * 2 + 1];
                            allhex[row as usize][col as usize][c ^ (g * 2 & d)] = hh + v * spw;
                        }
                    }
                }
            }
        }
        let hex_at = |row: isize, col: isize| &allhex[row.rem_euclid(3) as usize][col.rem_euclid(3) as usize];

        // Green minimum / maximum of each red/blue site's hexagon.
        for row in 2..ph as isize - 2 {
            for col in 2..pw as isize - 2 {
                if fcol(row, col) == 1 {
                    continue;
                }
                let p = row * spw + col;
                let hex = hex_at(row, col);
                let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                for &o in &hex[..6] {
                    let v = self.img[(p + o) as usize][1];
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
                self.img[p as usize][1] = lo;
                self.img[p as usize][3] = hi;
            }
        }

        let top = TOP as isize;
        let (mrow, mcol) = (ph as isize - top, pw as isize - top);
        for buf in &mut self.rgb {
            buf.clear();
            buf.resize(n, [0.0; 3]);
        }
        for row in top..mrow {
            for col in top..mcol {
                let p = (row * spw + col) as usize;
                let s = self.img[p];
                self.rgb[0][p] = [s[0], s[1], s[2]];
            }
        }
        let first = self.rgb[0].clone();
        for buf in &mut self.rgb[1..] {
            buf.copy_from_slice(&first);
        }

        // Interpolate green horizontally, vertically and along both diagonals.
        for row in top..mrow {
            let flip = ((row - sgrow).rem_euclid(3) == 0) as usize;
            for col in top..mcol {
                let f = fcol(row, col);
                if f == 1 {
                    continue;
                }
                let p = row * spw + col;
                let hex = hex_at(row, col);
                let img = &self.img;
                let g = |o: isize| img[(p + o) as usize][1];
                let v = |o: isize| img[(p + o) as usize][f];
                let color = [
                    174.0 * (g(hex[1]) + g(hex[0])) - 46.0 * (g(2 * hex[1]) + g(2 * hex[0])),
                    223.0 * g(hex[3]) + 33.0 * g(hex[2]) + 92.0 * (v(0) - v(-hex[2])),
                    164.0 * g(hex[4]) + 92.0 * g(-2 * hex[4]) + 33.0 * (2.0 * v(0) - v(3 * hex[4]) - v(-3 * hex[4])),
                    164.0 * g(hex[5]) + 92.0 * g(-2 * hex[5]) + 33.0 * (2.0 * v(0) - v(3 * hex[5]) - v(-3 * hex[5])),
                ];
                let (lo, hi) = (img[p as usize][1], img[p as usize][3]);
                for (c, value) in color.iter().enumerate() {
                    self.rgb[c ^ flip][p as usize][1] = (value / 256.0).min(hi).max(lo);
                }
            }
        }

        // Red and blue for solitary green pixels.
        let mut row = (top - sgrow + 4) / 3 * 3 + sgrow;
        while row < mrow - 2 {
            let mut col = (top - sgcol + 4) / 3 * 3 + sgcol;
            while col < mcol - 2 {
                let p = row * spw + col;
                let mut hc = fcol(row, col + 1);
                let mut diff = [0.0f32; 6];
                let mut color = [[0.0f32; 6]; 3];
                let mut step = 1isize;
                let mut buf = 0usize;
                for d in 0..6 {
                    let rb = &self.rgb[buf];
                    let at = |o: isize| rb[(p + o) as usize];
                    for c in 0..2 {
                        let s = step << c;
                        let g = 2.0 * at(0)[1] - at(s)[1] - at(-s)[1];
                        color[hc][d] = g + at(s)[hc] + at(-s)[hc];
                        if d > 1 {
                            diff[d] += sq(at(s)[1] - at(-s)[1] - at(s)[hc] + at(-s)[hc]) + sq(g);
                        }
                        hc ^= 2;
                    }
                    if d > 1 && d & 1 == 1 && diff[d - 1] < diff[d] {
                        for c in 0..2 {
                            color[c * 2][d] = color[c * 2][d - 1];
                        }
                    }
                    if d < 2 || d & 1 == 1 {
                        for c in 0..2 {
                            self.rgb[buf][p as usize][c * 2] = (color[c * 2][d] / 2.0).max(0.0);
                        }
                        buf += 1;
                    }
                    step = if step == 1 { spw } else { 1 };
                    hc ^= 2;
                }
                col += 3;
            }
            row += 3;
        }

        // Red at blue sites and blue at red sites.
        for row in top + 3..mrow - 3 {
            let vertical = (row - sgrow).rem_euclid(3) != 0;
            let (c, hh) = if vertical { (spw, 3) } else { (1, 3 * spw) };
            for col in top + 3..mcol - 3 {
                let fc = fcol(row, col);
                if fc == 1 {
                    continue;
                }
                let f = 2 - fc;
                let p = row * spw + col;
                for d in 0..NDIR {
                    let rb = &self.rgb[d];
                    let at = |o: isize| rb[(p + o) as usize];
                    let g0 = at(0)[1];
                    let parity = if vertical { d & 1 == 1 } else { d & 1 == 0 };
                    let i = if d > 1
                        || parity
                        || ((g0 - at(c)[1]).abs() + (g0 - at(-c)[1]).abs())
                            < 2.0 * ((g0 - at(hh)[1]).abs() + (g0 - at(-hh)[1]).abs())
                    {
                        c
                    } else {
                        hh
                    };
                    let value = (at(i)[f] + at(-i)[f] + 2.0 * g0 - at(i)[1] - at(-i)[1]) / 2.0;
                    self.rgb[d][p as usize][f] = value.max(0.0);
                }
            }
        }

        // Red and blue for 2x2 blocks of green. dcraw's 1-pass fills only the
        // first two buffers here; the diagonal buffers reuse the same neighbour
        // pairs (with their own greens) so every direction stays a valid
        // candidate for the homogeneity vote.
        for row in top + 2..mrow - 2 {
            if (row - sgrow).rem_euclid(3) == 0 {
                continue;
            }
            for col in top + 2..mcol - 2 {
                if (col - sgcol).rem_euclid(3) == 0 {
                    continue;
                }
                let p = row * spw + col;
                let hex = hex_at(row, col);
                for (k, rb) in self.rgb.iter_mut().enumerate() {
                    let d = (k % 2) * 2;
                    let (a, b) = (hex[d], hex[d + 1]);
                    let at = |o: isize| rb[(p + o) as usize];
                    let (ra, rbb) = (at(a), at(b));
                    let here = at(0);
                    let mut px = here;
                    if a + b != 0 {
                        let g = 3.0 * here[1] - 2.0 * ra[1] - rbb[1];
                        for c in [0usize, 2] {
                            px[c] = ((g + 2.0 * ra[c] + rbb[c]) / 3.0).max(0.0);
                        }
                    } else {
                        let g = 2.0 * here[1] - ra[1] - rbb[1];
                        for c in [0usize, 2] {
                            px[c] = ((g + ra[c] + rbb[c]) / 2.0).max(0.0);
                        }
                    }
                    rb[p as usize] = px;
                }
            }
        }

        // Convert to CIELab and differentiate in each direction.
        let dirs = [1isize, spw, spw + 1, spw - 1];
        self.lab.clear();
        self.lab.resize(n, [0.0; 3]);
        for d in 0..NDIR {
            for row in top + 2..mrow - 2 {
                for col in top + 2..mcol - 2 {
                    let p = (row * spw + col) as usize;
                    self.lab[p] = cielab(self.rgb[d][p]);
                }
            }
            let drv = &mut self.drv[d];
            drv.clear();
            drv.resize(n, 0.0);
            let f = dirs[d];
            for row in top + 3..mrow - 3 {
                for col in top + 3..mcol - 3 {
                    let p = row * spw + col;
                    let l = |o: isize| self.lab[(p + o) as usize];
                    let g = 2.0 * l(0)[0] - l(f)[0] - l(-f)[0];
                    drv[p as usize] = sq(g)
                        + sq(2.0 * l(0)[1] - l(f)[1] - l(-f)[1] + g * 500.0 / 232.0)
                        + sq(2.0 * l(0)[2] - l(f)[2] - l(-f)[2] - g * 500.0 / 580.0);
                }
            }
        }

        // Homogeneity maps from the derivatives.
        for homo in &mut self.homo {
            homo.clear();
            homo.resize(n, 0);
        }
        for row in top + 4..mrow - 4 {
            for col in top + 4..mcol - 4 {
                let p = row * spw + col;
                let tr = 8.0 * (0..NDIR).map(|d| self.drv[d][p as usize]).fold(f32::MAX, f32::min);
                for d in 0..NDIR {
                    let mut count = 0u8;
                    for v in -1..=1 {
                        for hz in -1..=1 {
                            if self.drv[d][(p + v * spw + hz) as usize] <= tr {
                                count += 1;
                            }
                        }
                    }
                    self.homo[d][p as usize] = count;
                }
            }
        }

        // Average the most homogeneous directions (only the core is kept).
        self.result.clear();
        self.result.resize(n, [0.0; 3]);
        for row in BORDER as isize..(BORDER + th) as isize {
            for col in BORDER as isize..(BORDER + tw) as isize {
                let p = row * spw + col;
                let mut hm = [0u32; NDIR];
                for (d, sum) in hm.iter_mut().enumerate() {
                    for v in -2..=2 {
                        for hz in -2..=2 {
                            *sum += self.homo[d][(p + v * spw + hz) as usize] as u32;
                        }
                    }
                }
                let mut best = *hm.iter().max().unwrap();
                best -= best >> 3;
                let (mut sum, mut count) = ([0.0f32; 3], 0.0f32);
                for d in 0..NDIR {
                    if hm[d] >= best {
                        let px = self.rgb[d][p as usize];
                        for c in 0..3 {
                            sum[c] += px[c];
                        }
                        count += 1.0;
                    }
                }
                self.result[p as usize] = sum.map(|s| s / count);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rawler::imgop::{Dim2, Point};

    const XTRANS: &str = "GGRGGBGGBGGRBRGRBGGGBGGRGGRGGBRBGBRG";

    fn mosaic(w: usize, h: usize, f: impl Fn(usize, usize) -> [f32; 3]) -> (Vec<f32>, CFA) {
        let cfa = CFA::new(XTRANS);
        let raw = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y)[cfa.color_at(y, x)])
            .collect();
        (raw, cfa)
    }

    fn mean_abs(out: &[[f32; 3]], w: usize, h: usize, m: usize, e: impl Fn(usize, usize, [f32; 3]) -> f32) -> f64 {
        let mut sum = 0.0f64;
        let mut count = 0;
        for y in m..h - m {
            for x in m..w - m {
                sum += e(x, y, out[y * w + x]) as f64;
                count += 1;
            }
        }
        sum / count as f64
    }

    #[test]
    fn reflect_keeps_phase_and_bounds() {
        for len in [12usize, 13, 17, 100] {
            for v in -20isize..(len as isize + 20) {
                let r = reflect6(v, len);
                assert!(r < len, "{v} -> {r} (len {len})");
                assert_eq!((r as isize).rem_euclid(6), v.rem_euclid(6), "{v} -> {r} (len {len})");
            }
        }
    }

    #[test]
    fn flat_grey_stays_grey() {
        let (w, h) = (300, 260);
        let (raw, cfa) = mosaic(w, h, |_, _| [0.4; 3]);
        let out = demosaic(&raw, w, &cfa, Rect::new(Point::zero(), Dim2::new(w, h)), [1.0; 3]);
        assert!(out.iter().all(|p| p.iter().all(|c| (c - 0.4).abs() < 1e-3)), "{:?}", out[1000]);
    }

    #[test]
    fn white_balance_scale_round_trips_and_roi_offset() {
        let (w, h) = (101, 89);
        let (raw, cfa) = mosaic(w, h, |_, _| [0.2, 0.4, 0.4 / 1.5]);
        let roi = Rect::new(Point::new(5, 3), Dim2::new(90, 80));
        let out = demosaic(&raw, w, &cfa, roi, [2.0, 1.0, 1.5]);
        assert_eq!(out.len(), 90 * 80);
        let err = mean_abs(&out, 90, 80, 0, |_, _, p| (p[0] - 0.2).abs() + (p[1] - 0.4).abs() + (p[2] - 0.4 / 1.5).abs());
        assert!(err < 1e-3, "mean error {err}");
    }

    #[test]
    fn smooth_colour_gradient_is_reproduced() {
        let (w, h) = (240, 200);
        let truth = |x: usize, y: usize| [0.2 + 0.002 * x as f32, 0.4, 0.3 + 0.001 * y as f32];
        let (raw, cfa) = mosaic(w, h, truth);
        let out = demosaic(&raw, w, &cfa, Rect::new(Point::zero(), Dim2::new(w, h)), [1.0; 3]);
        let err = mean_abs(&out, w, h, 12, |x, y, p| {
            let t = truth(x, y);
            ((p[0] - t[0]).abs() + (p[1] - t[1]).abs() + (p[2] - t[2]).abs()) / 3.0
        });
        assert!(err < 3e-3, "mean error {err}");
    }

    /// Fine neutral diagonal texture: false colour must be well below rawler's
    /// X-Trans interpolation, which this replaces.
    #[test]
    fn fine_neutral_texture_beats_rawler_xtrans() {
        use rawler::imgop::sensor::bayer::Demosaic;
        use rawler::imgop::sensor::xtrans::demosaic::XTransDemosaic;
        use rawler::{cfa::PlaneColor, pixarray::PixF32};

        let (w, h) = (264, 264);
        let roi = Rect::new(Point::zero(), Dim2::new(w, h));
        let chroma = |_: usize, _: usize, p: [f32; 3]| (p[0] - p[1]).abs() + (p[2] - p[1]).abs();
        // A 45-degree weave (one of Markesteijn's own directions) and an
        // off-angle one that no direction lines up with.
        for (label, fx, fy) in [("45-degree", 1.25f32, 1.25f32), ("off-angle", 1.05, 0.55)] {
            let truth = |x: usize, y: usize| {
                let v = 0.35 + 0.2 * (x as f32 * fx + y as f32 * fy).sin();
                [v, v, v]
            };
            let (raw, cfa) = mosaic(w, h, truth);
            let ours = demosaic(&raw, w, &cfa, roi, [1.0; 3]);
            let ours = mean_abs(&ours, w, h, 16, chroma);
            let theirs =
                XTransDemosaic::new().demosaic(&PixF32::new_with(raw.clone(), w, h), &cfa, &PlaneColor::new("RGB"), roi);
            let theirs = mean_abs(&theirs.data, w, h, 16, chroma);
            eprintln!("false colour, {label} texture: markesteijn {ours:.4}, rawler x-trans {theirs:.4}");
            assert!(ours < theirs * 0.7, "{label}: markesteijn {ours} not clearly better than rawler {theirs}");
        }
    }
}
