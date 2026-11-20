// Fork: colour-managed decoding of profiled CMYK JPEG/TIFF (press files). The
// generic decoders only expose a naive, unmanaged ink inversion; this parses the
// embedded press profile's A2B LUT and converts to sRGB. RGB input profiles go
// through upstream's moxcms path in image_loader.rs, and export profiles come
// from `render::color_space` (upstream v1.7).

/// D50 is the PCS illuminant every ICC profile is defined against, so the sRGB
/// primaries below are the Bradford-adapted values, not the D65 ones.
const WHITE_D50: [f64; 3] = [0.9642, 1.0, 0.8249];
const PRIMARY_R: [f64; 3] = [0.43607, 0.22249, 0.01392];
const PRIMARY_G: [f64; 3] = [0.38515, 0.71687, 0.09708];
const PRIMARY_B: [f64; 3] = [0.14307, 0.06061, 0.71410];

const HEADER_LEN: usize = 128;

// Big-endian readers and helpers shared by the CMYK parser below. RGB input
// profiles are converted by upstream's moxcms path in image_loader.rs.

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn find_tag<'a>(profile: &'a [u8], signature: &[u8; 4]) -> Option<&'a [u8]> {
    let count = read_u32(profile, HEADER_LEN)? as usize;
    for index in 0..count {
        let entry = HEADER_LEN + 4 + index * 12;
        if profile.get(entry..entry + 4)? == signature {
            let offset = read_u32(profile, entry + 4)? as usize;
            let length = read_u32(profile, entry + 8)? as usize;
            return profile.get(offset..offset + length);
        }
    }
    None
}

fn invert_3x3(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;
    let mut out = [[0.0f64; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            // Cofactor of the transposed position (adjugate).
            let r1 = (col + 1) % 3;
            let r2 = (col + 2) % 3;
            let c1 = (row + 1) % 3;
            let c2 = (row + 2) % 3;
            out[row][col] = (m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]) * inv_det;
        }
    }
    Some(out)
}

fn srgb_encode(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

// ===== CMYK profiles =====
//
// CMYK profiles are never matrix/TRC — they are LUT profiles, where A2B tags
// carry per-channel input curves, a 4-D grid into the PCS (almost always Lab)
// and per-channel output curves. The `mft1`/`mft2` parser below covers the v2
// LUT layout, which is what press profiles (SWOP, FOGRA, GRACoL and the
// Ghostscript defaults) ship; a profile carrying only a v4 `mAB ` tag parses
// to `None` and the caller keeps the decoder's naive ink inversion.

/// A CMYK -> sRGB conversion built from a profile's A2B LUT.
pub struct CmykTransform {
    /// Per-ink input curves, normalized samples over 0..=1.
    input_tables: [Vec<f32>; 4],
    grid_points: usize,
    /// `grid_points^4` nodes × 3 PCS channels, C-major in channel order CMYK.
    clut: Vec<f32>,
    output_tables: [Vec<f32>; 3],
    /// Lab PCS (v2 16-bit encoding) when true, XYZ when false.
    pcs_is_lab: bool,
    xyz_to_linear_srgb: [[f32; 3]; 3],
}

fn sample_table(table: &[f32], x: f32) -> f32 {
    let position = x.clamp(0.0, 1.0) as f64 * (table.len() - 1) as f64;
    let low = position.floor() as usize;
    let high = (low + 1).min(table.len() - 1);
    let fraction = (position - low as f64) as f32;
    table[low] + (table[high] - table[low]) * fraction
}

impl CmykTransform {
    /// Ink coverages 0..=1 in, non-linear sRGB 0..=1 out.
    pub fn to_srgb(&self, cmyk: [f32; 4]) -> [f32; 3] {
        // Input curves, then the position of each channel inside the grid.
        let mut base = [0usize; 4];
        let mut fraction = [0f32; 4];
        for channel in 0..4 {
            let curved = sample_table(&self.input_tables[channel], cmyk[channel]);
            let position = curved as f64 * (self.grid_points - 1) as f64;
            let low = (position.floor() as usize).min(self.grid_points - 2);
            base[channel] = low;
            fraction[channel] = (position - low as f64) as f32;
        }

        // Quadrilinear interpolation over the 16 surrounding grid nodes.
        let mut pcs = [0f32; 3];
        for corner in 0..16usize {
            let mut weight = 1f32;
            let mut index = 0usize;
            for channel in 0..4 {
                let stride = self.grid_points.pow(3 - channel as u32) * 3;
                if corner & (8 >> channel) != 0 {
                    weight *= fraction[channel];
                    index += (base[channel] + 1) * stride;
                } else {
                    weight *= 1.0 - fraction[channel];
                    index += base[channel] * stride;
                }
            }
            if weight > 0.0 {
                for out in 0..3 {
                    pcs[out] += weight * self.clut[index + out];
                }
            }
        }
        for out in 0..3 {
            pcs[out] = sample_table(&self.output_tables[out], pcs[out]);
        }

        let xyz = if self.pcs_is_lab {
            // v2 LUTs encode Lab with the legacy 0xFF00 scale.
            let l = pcs[0] * 65535.0 / 65280.0 * 100.0;
            let a = pcs[1] * 65535.0 / 256.0 - 128.0;
            let b = pcs[2] * 65535.0 / 256.0 - 128.0;
            lab_to_xyz_d50(l, a, b)
        } else {
            // XYZ PCS: u16 range spans 0..=(2 - 2^-15).
            [pcs[0] * 1.999_969_5, pcs[1] * 1.999_969_5, pcs[2] * 1.999_969_5]
        };

        let mut rgb = [0f32; 3];
        for channel in 0..3 {
            let linear = self.xyz_to_linear_srgb[channel][0] * xyz[0]
                + self.xyz_to_linear_srgb[channel][1] * xyz[1]
                + self.xyz_to_linear_srgb[channel][2] * xyz[2];
            rgb[channel] = srgb_encode(linear.clamp(0.0, 1.0));
        }
        rgb
    }
}

fn lab_to_xyz_d50(l: f32, a: f32, b: f32) -> [f32; 3] {
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let f_inverse = |f: f32| {
        if f > 6.0 / 29.0 {
            f * f * f
        } else {
            3.0 * (6.0f32 / 29.0).powi(2) * (f - 4.0 / 29.0)
        }
    };
    [
        f_inverse(fx) * WHITE_D50[0] as f32,
        f_inverse(fy) * WHITE_D50[1] as f32,
        f_inverse(fz) * WHITE_D50[2] as f32,
    ]
}

/// Parses one `mft1`/`mft2` LUT with 4 input and 3 output channels.
fn parse_mft_lut(tag: &[u8]) -> Option<(usize, [Vec<f32>; 4], Vec<f32>, [Vec<f32>; 3])> {
    let wide = match tag.get(0..4)? {
        b"mft2" => true,
        b"mft1" => false,
        _ => return None,
    };
    if *tag.get(8)? != 4 || *tag.get(9)? != 3 {
        return None;
    }
    let grid_points = *tag.get(10)? as usize;
    if grid_points < 2 {
        return None;
    }
    // The 3x3 matrix at 12..48 only applies to an XYZ input side, never CMYK.
    let (input_entries, output_entries, mut at) = if wide {
        let input = u16::from_be_bytes(tag.get(48..50)?.try_into().ok()?) as usize;
        let output = u16::from_be_bytes(tag.get(50..52)?.try_into().ok()?) as usize;
        if input < 2 || output < 2 {
            return None;
        }
        (input, output, 52usize)
    } else {
        (256, 256, 48usize)
    };

    let mut read_table = |entries: usize| -> Option<Vec<f32>> {
        let mut table = Vec::with_capacity(entries);
        for _ in 0..entries {
            let value = if wide {
                let raw = u16::from_be_bytes(tag.get(at..at + 2)?.try_into().ok()?);
                at += 2;
                raw as f32 / 65535.0
            } else {
                let raw = *tag.get(at)?;
                at += 1;
                raw as f32 / 255.0
            };
            table.push(value);
        }
        Some(table)
    };

    let input_tables = [
        read_table(input_entries)?,
        read_table(input_entries)?,
        read_table(input_entries)?,
        read_table(input_entries)?,
    ];
    let clut = read_table(grid_points.checked_pow(4)?.checked_mul(3)?)?;
    let output_tables = [
        read_table(output_entries)?,
        read_table(output_entries)?,
        read_table(output_entries)?,
    ];
    Some((grid_points, input_tables, clut, output_tables))
}

/// Parses an embedded CMYK profile into a to-sRGB transform, or `None` when
/// the profile isn't a CMYK LUT profile this parser covers — the caller then
/// keeps the decoder's naive conversion, the pre-existing behaviour.
pub fn parse_cmyk_profile(profile: &[u8]) -> Option<CmykTransform> {
    if profile.len() < HEADER_LEN + 4
        || profile.get(36..40)? != b"acsp"
        || profile.get(16..20)? != b"CMYK"
    {
        return None;
    }
    let pcs_is_lab = match profile.get(20..24)? {
        b"Lab " => true,
        b"XYZ " => false,
        _ => return None,
    };

    // Relative colorimetric when present (matching how a CMS renders into a
    // working space), the perceptual table otherwise.
    let tag = find_tag(profile, b"A2B1").or_else(|| find_tag(profile, b"A2B0"))?;
    let (grid_points, input_tables, clut, output_tables) = parse_mft_lut(tag)?;

    let srgb = [
        [PRIMARY_R[0], PRIMARY_G[0], PRIMARY_B[0]],
        [PRIMARY_R[1], PRIMARY_G[1], PRIMARY_B[1]],
        [PRIMARY_R[2], PRIMARY_G[2], PRIMARY_B[2]],
    ];
    let inverse = invert_3x3(srgb)?;
    let mut xyz_to_linear_srgb = [[0f32; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            xyz_to_linear_srgb[row][col] = inverse[row][col] as f32;
        }
    }

    Some(CmykTransform {
        input_tables,
        grid_points,
        clut,
        output_tables,
        pcs_is_lab,
        xyz_to_linear_srgb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s15_fixed16(value: f64) -> [u8; 4] {
        ((value * 65536.0).round() as i32).to_be_bytes()
    }

    fn test_cmyk_profile(node: impl Fn([usize; 4]) -> [u16; 3]) -> Vec<u8> {
        let mut lut = Vec::new();
        lut.extend_from_slice(b"mft2");
        lut.extend_from_slice(&[0u8; 4]);
        lut.extend_from_slice(&[4, 3, 2, 0]); // in, out, grid, pad
        for value in [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
            lut.extend_from_slice(&s15_fixed16(value));
        }
        lut.extend_from_slice(&2u16.to_be_bytes()); // input entries
        lut.extend_from_slice(&2u16.to_be_bytes()); // output entries
        for _ in 0..4 {
            lut.extend_from_slice(&0u16.to_be_bytes());
            lut.extend_from_slice(&65535u16.to_be_bytes());
        }
        for index in 0..16usize {
            let corner = [index >> 3 & 1, index >> 2 & 1, index >> 1 & 1, index & 1];
            for channel in node(corner) {
                lut.extend_from_slice(&channel.to_be_bytes());
            }
        }
        for _ in 0..3 {
            lut.extend_from_slice(&0u16.to_be_bytes());
            lut.extend_from_slice(&65535u16.to_be_bytes());
        }

        let mut profile = vec![0u8; HEADER_LEN];
        profile[16..20].copy_from_slice(b"CMYK");
        profile[20..24].copy_from_slice(b"Lab ");
        profile[36..40].copy_from_slice(b"acsp");
        profile.extend_from_slice(&1u32.to_be_bytes());
        profile.extend_from_slice(b"A2B0");
        profile.extend_from_slice(&((HEADER_LEN + 16) as u32).to_be_bytes());
        profile.extend_from_slice(&(lut.len() as u32).to_be_bytes());
        profile.extend_from_slice(&lut);
        profile
    }

    /// v2 legacy 16-bit Lab encoding.
    fn lab16(l: f64, a: f64, b: f64) -> [u16; 3] {
        [
            (l / 100.0 * 65280.0).round() as u16,
            ((a + 128.0) * 256.0).round() as u16,
            ((b + 128.0) * 256.0).round() as u16,
        ]
    }

    #[test]
    fn cmyk_lut_maps_paper_white_and_full_ink_through_lab() {
        // No ink -> Lab white, full ink -> Lab black, in between interpolated.
        let profile = test_cmyk_profile(|corner| {
            let coverage = corner.iter().sum::<usize>() as f64 / 4.0;
            lab16(100.0 * (1.0 - coverage), 0.0, 0.0)
        });
        let transform = parse_cmyk_profile(&profile).expect("LUT profile must parse");

        let white = transform.to_srgb([0.0; 4]);
        assert!(white.iter().all(|&v| v > 0.99), "paper white was {white:?}");
        let black = transform.to_srgb([1.0; 4]);
        assert!(black.iter().all(|&v| v < 0.01), "full ink was {black:?}");

        // Lab 50 is mid lightness; sRGB encodes it near 119/255, neutral.
        let mid = transform.to_srgb([0.5, 0.5, 0.5, 0.5]);
        assert!((mid[0] - mid[1]).abs() < 0.01 && (mid[1] - mid[2]).abs() < 0.01);
        assert!(
            (mid[0] - 0.466).abs() < 0.02,
            "Lab 50 should encode near 0.466, was {}",
            mid[0]
        );
    }

    #[test]
    fn cmyk_parser_rejects_rgb_profiles() {
        assert!(
            parse_cmyk_profile(crate::render::color_space::OutputColorSpace::Srgb.icc_profile())
                .is_none()
        );
    }

}
