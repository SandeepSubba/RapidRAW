// Finds the printed label (a bright, roughly rectangular card) in a photo of a
// fabric swatch, so the assistant can be handed a tight native-resolution crop of
// the text instead of guessing coordinates off a downscaled overview.
//
// The naive approach — bounding box of everything bright — fails on these shots:
// the frame also contains a steel ruler (long and thin) and loose white threads
// (small and ragged). So candidates are found as connected components and scored
// on how card-like they are: area, aspect, and how completely they fill their own
// bounding box.

use image::{DynamicImage, GenericImageView};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Longest edge the search runs at. Detection needs shape, not detail, and this
/// keeps a 22MP frame's scan in the low milliseconds.
const WORK_DIM: u32 = 720;

/// Fraction of the frame a label may occupy. Below this it is a thread or a
/// speck; above it, the crop is no better than the overview already sent.
const MIN_AREA_FRAC: f32 = 0.002;
const MAX_AREA_FRAC: f32 = 0.25;

/// A card is roughly as wide as it is tall, within reason. A ruler is ~1:20 and
/// must not win.
const MIN_ASPECT: f32 = 0.4;
const MAX_ASPECT: f32 = 3.5;

/// A rectangle fills its own bounding box; a ragged thread or a hand does not.
const MIN_FILL: f32 = 0.55;

/// Grow the winning box so descenders and the border are not clipped.
const PAD_FRAC: f32 = 0.06;

struct Component {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    pixels: u32,
    lum_sum: u64,
}

/// Locate the most label-like bright region, in the coordinate space of `img`.
pub fn detect_label_region(img: &DynamicImage) -> Option<Rect> {
    let (full_w, full_h) = img.dimensions();
    if full_w == 0 || full_h == 0 {
        return None;
    }

    let work = img.thumbnail(WORK_DIM, WORK_DIM).to_luma8();
    let (w, h) = (work.width(), work.height());
    if w < 8 || h < 8 {
        return None;
    }

    // Threshold high on the histogram rather than at a fixed value: exposure of
    // these swatch shots varies, but the label is reliably among the brightest
    // few percent of a dark-fabric frame.
    let mut hist = [0u32; 256];
    for p in work.pixels() {
        hist[p.0[0] as usize] += 1;
    }
    let total = w * h;
    let target = (total as f32 * 0.94) as u32;
    let mut acc = 0u32;
    let mut threshold = 255u8;
    for (v, count) in hist.iter().enumerate() {
        acc += count;
        if acc >= target {
            threshold = v as u8;
            break;
        }
    }
    // A flat frame can put the percentile in the midtones; refuse to treat half
    // the image as "label".
    let threshold = threshold.max(110);

    // Flood-fill each bright blob once, iteratively — recursion would blow the
    // stack on a large connected region.
    let mut seen = vec![false; (w * h) as usize];
    let mut stack: Vec<(u32, u32)> = Vec::new();
    let mut best: Option<(f32, Rect)> = None;

    for sy in 0..h {
        for sx in 0..w {
            let idx = (sy * w + sx) as usize;
            if seen[idx] || work.get_pixel(sx, sy).0[0] < threshold {
                continue;
            }
            let mut c = Component {
                min_x: sx,
                min_y: sy,
                max_x: sx,
                max_y: sy,
                pixels: 0,
                lum_sum: 0,
            };
            stack.push((sx, sy));
            seen[idx] = true;

            while let Some((x, y)) = stack.pop() {
                let v = work.get_pixel(x, y).0[0];
                c.pixels += 1;
                c.lum_sum += v as u64;
                c.min_x = c.min_x.min(x);
                c.min_y = c.min_y.min(y);
                c.max_x = c.max_x.max(x);
                c.max_y = c.max_y.max(y);

                let mut neighbours: [Option<(u32, u32)>; 4] = [None; 4];
                if x > 0 {
                    neighbours[0] = Some((x - 1, y));
                }
                if y > 0 {
                    neighbours[1] = Some((x, y - 1));
                }
                if x + 1 < w {
                    neighbours[2] = Some((x + 1, y));
                }
                if y + 1 < h {
                    neighbours[3] = Some((x, y + 1));
                }
                for n in neighbours.into_iter().flatten() {
                    let ni = (n.1 * w + n.0) as usize;
                    if !seen[ni] && work.get_pixel(n.0, n.1).0[0] >= threshold {
                        seen[ni] = true;
                        stack.push(n);
                    }
                }
            }

            if let Some(scored) = score(&c, w, h)
                && best.as_ref().map(|(s, _)| scored.0 > *s).unwrap_or(true)
            {
                best = Some(scored);
            }
        }
    }

    let (_, rect) = best?;
    Some(scale_and_pad(rect, w, h, full_w, full_h))
}

/// Reject anything un-card-like; score the rest so the brightest, most solidly
/// filled candidate wins.
fn score(c: &Component, w: u32, h: u32) -> Option<(f32, Rect)> {
    let bw = c.max_x - c.min_x + 1;
    let bh = c.max_y - c.min_y + 1;
    let box_area = (bw * bh) as f32;
    let frame_area = (w * h) as f32;

    let area_frac = box_area / frame_area;
    if !(MIN_AREA_FRAC..=MAX_AREA_FRAC).contains(&area_frac) {
        return None;
    }
    let aspect = bw as f32 / bh as f32;
    if !(MIN_ASPECT..=MAX_ASPECT).contains(&aspect) {
        return None;
    }
    let fill = c.pixels as f32 / box_area;
    if fill < MIN_FILL {
        return None;
    }
    let mean_lum = (c.lum_sum as f32 / c.pixels as f32) / 255.0;

    // Solidity and brightness identify the card; the mild area term breaks ties
    // toward the larger of two equally card-like regions.
    Some((
        fill * mean_lum * (1.0 + area_frac),
        Rect {
            x: c.min_x,
            y: c.min_y,
            width: bw,
            height: bh,
        },
    ))
}

fn scale_and_pad(r: Rect, w: u32, h: u32, full_w: u32, full_h: u32) -> Rect {
    let sx = full_w as f32 / w as f32;
    let sy = full_h as f32 / h as f32;
    let pad_x = r.width as f32 * sx * PAD_FRAC;
    let pad_y = r.height as f32 * sy * PAD_FRAC;

    let x0 = ((r.x as f32 * sx) - pad_x).max(0.0);
    let y0 = ((r.y as f32 * sy) - pad_y).max(0.0);
    let x1 = (((r.x + r.width) as f32 * sx) + pad_x).min(full_w as f32);
    let y1 = (((r.y + r.height) as f32 * sy) + pad_y).min(full_h as f32);

    Rect {
        x: x0 as u32,
        y: y0 as u32,
        width: (x1 - x0).max(1.0) as u32,
        height: (y1 - y0).max(1.0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn frame(w: u32, h: u32) -> RgbImage {
        RgbImage::from_pixel(w, h, Rgb([40, 38, 42]))
    }

    fn fill_rect(img: &mut RgbImage, x: u32, y: u32, w: u32, h: u32, v: u8) {
        for yy in y..(y + h).min(img.height()) {
            for xx in x..(x + w).min(img.width()) {
                img.put_pixel(xx, yy, Rgb([v, v, v]));
            }
        }
    }

    #[test]
    fn finds_a_bright_card_on_dark_fabric() {
        let mut img = frame(800, 600);
        fill_rect(&mut img, 500, 80, 180, 110, 245);
        let r = detect_label_region(&DynamicImage::ImageRgb8(img)).expect("label found");

        // Padded, so assert containment rather than exact equality.
        assert!(r.x <= 500 && r.y <= 80, "must cover the card origin, got {r:?}");
        assert!(r.x + r.width >= 680, "must span the card width, got {r:?}");
        assert!(r.y + r.height >= 190, "must span the card height, got {r:?}");
    }

    #[test]
    fn ignores_the_steel_ruler_and_picks_the_card() {
        let mut img = frame(800, 600);
        fill_rect(&mut img, 745, 20, 20, 560, 250); // ruler: bright but ~1:28
        fill_rect(&mut img, 200, 300, 170, 100, 240); // the label
        let r = detect_label_region(&DynamicImage::ImageRgb8(img)).expect("label found");

        assert!(r.x < 400, "should pick the card, not the ruler, got {r:?}");
        assert!(r.height < 300, "ruler-shaped box leaked in: {r:?}");
    }

    #[test]
    fn ignores_a_small_stray_thread() {
        let mut img = frame(800, 600);
        fill_rect(&mut img, 30, 560, 10, 8, 255); // thread fleck
        fill_rect(&mut img, 300, 150, 200, 130, 235);
        let r = detect_label_region(&DynamicImage::ImageRgb8(img)).expect("label found");
        assert!(r.x > 200 && r.y > 100, "picked the fleck: {r:?}");
    }

    #[test]
    fn returns_none_when_nothing_is_card_like() {
        let img = frame(400, 300);
        assert!(detect_label_region(&DynamicImage::ImageRgb8(img)).is_none());
    }

    #[test]
    fn never_reports_a_box_outside_the_image() {
        let mut img = frame(300, 200);
        fill_rect(&mut img, 0, 0, 90, 70, 250); // flush against the corner
        let r = detect_label_region(&DynamicImage::ImageRgb8(img)).expect("label found");
        assert!(
            r.x + r.width <= 300 && r.y + r.height <= 200,
            "out of bounds: {r:?}"
        );
    }
}
