// A self-contained sRGB ICC profile for tagging exported images.
//
// Exports used to ship untagged, which leaves every downstream reader guessing
// what the numbers mean: browsers assume sRGB and usually land right, but
// editors, print shops and wide-gamut displays are guessing too. The pipeline
// renders in an Rgba16Float working space and resolves to Rgba8Unorm with the
// sRGB transfer applied in-shader, so tagging the file sRGB states what is
// already true of the bytes rather than converting anything.
//
// Built in code instead of shipped as a binary blob so it stays auditable: a
// v2.4 matrix/TRC display profile carrying the D50-adapted sRGB primaries and
// one 1024-point tone curve shared by all three channels.

use std::sync::OnceLock;

/// D50 is the PCS illuminant every ICC profile is defined against, so the sRGB
/// primaries below are the Bradford-adapted values, not the D65 ones.
const WHITE_D50: [f64; 3] = [0.9642, 1.0, 0.8249];
const PRIMARY_R: [f64; 3] = [0.43607, 0.22249, 0.01392];
const PRIMARY_G: [f64; 3] = [0.38515, 0.71687, 0.09708];
const PRIMARY_B: [f64; 3] = [0.14307, 0.06061, 0.71410];

const HEADER_LEN: usize = 128;
const TRC_POINTS: usize = 1024;

/// The profile bytes, built once and reused for every exported image.
pub fn srgb_profile() -> &'static [u8] {
    static PROFILE: OnceLock<Vec<u8>> = OnceLock::new();
    PROFILE.get_or_init(build_srgb_profile)
}

fn s15_fixed16(value: f64) -> [u8; 4] {
    ((value * 65536.0).round() as i32).to_be_bytes()
}

fn xyz_tag(xyz: [f64; 3]) -> Vec<u8> {
    let mut tag = Vec::with_capacity(20);
    tag.extend_from_slice(b"XYZ ");
    tag.extend_from_slice(&[0u8; 4]);
    for channel in xyz {
        tag.extend_from_slice(&s15_fixed16(channel));
    }
    tag
}

/// The sRGB transfer function, sampled as a `curv` table. An ICC TRC maps the
/// encoded device value to linear light, which is the inverse of what the
/// shader applies on the way out.
fn trc_tag() -> Vec<u8> {
    let mut tag = Vec::with_capacity(12 + TRC_POINTS * 2);
    tag.extend_from_slice(b"curv");
    tag.extend_from_slice(&[0u8; 4]);
    tag.extend_from_slice(&(TRC_POINTS as u32).to_be_bytes());
    for index in 0..TRC_POINTS {
        let encoded = index as f64 / (TRC_POINTS - 1) as f64;
        let linear = if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        };
        let sample = (linear * 65535.0).round().clamp(0.0, 65535.0) as u16;
        tag.extend_from_slice(&sample.to_be_bytes());
    }
    tag
}

/// `textDescriptionType` — a v2 tag that carries an ASCII string followed by
/// empty Unicode and ScriptCode blocks, which readers still expect to be there.
fn desc_tag(text: &str) -> Vec<u8> {
    let ascii = text.as_bytes();
    let mut tag = Vec::with_capacity(90 + ascii.len());
    tag.extend_from_slice(b"desc");
    tag.extend_from_slice(&[0u8; 4]);
    tag.extend_from_slice(&((ascii.len() + 1) as u32).to_be_bytes());
    tag.extend_from_slice(ascii);
    tag.push(0);
    tag.extend_from_slice(&[0u8; 4]); // Unicode language code
    tag.extend_from_slice(&[0u8; 4]); // Unicode count
    tag.extend_from_slice(&[0u8; 2]); // ScriptCode code
    tag.push(0); // ScriptCode count
    tag.extend_from_slice(&[0u8; 67]); // ScriptCode data
    tag
}

fn text_tag(text: &str) -> Vec<u8> {
    let mut tag = Vec::with_capacity(8 + text.len() + 1);
    tag.extend_from_slice(b"text");
    tag.extend_from_slice(&[0u8; 4]);
    tag.extend_from_slice(text.as_bytes());
    tag.push(0);
    tag
}

fn build_srgb_profile() -> Vec<u8> {
    let desc = desc_tag("sRGB IEC61966-2.1");
    let white = xyz_tag(WHITE_D50);
    let red = xyz_tag(PRIMARY_R);
    let green = xyz_tag(PRIMARY_G);
    let blue = xyz_tag(PRIMARY_B);
    let curve = trc_tag();
    let copyright = text_tag("Public Domain");

    // Each blob is stored once. The three TRC tags deliberately point at the
    // same offset — the tag table is allowed to alias, and sharing one curve
    // keeps the profile near 2 KB rather than 6 KB.
    let blobs = [&desc, &white, &red, &green, &blue, &curve, &copyright];
    let signatures: [&[u8; 4]; 9] = [
        b"desc", b"wtpt", b"rXYZ", b"gXYZ", b"bXYZ", b"rTRC", b"gTRC", b"bTRC", b"cprt",
    ];
    let blob_for_tag = [0usize, 1, 2, 3, 4, 5, 5, 5, 6];

    let table_len = 4 + signatures.len() * 12;
    let mut body = Vec::new();
    let mut placements = Vec::with_capacity(blobs.len());
    let mut offset = HEADER_LEN + table_len;
    for blob in blobs {
        // Tag data has to start on a 4-byte boundary.
        while offset % 4 != 0 {
            body.push(0);
            offset += 1;
        }
        placements.push((offset as u32, blob.len() as u32));
        body.extend_from_slice(blob);
        offset += blob.len();
    }

    // The tag table must be sorted by signature; some parsers binary-search it.
    let mut table_entries: Vec<(&[u8; 4], (u32, u32))> = signatures
        .iter()
        .zip(blob_for_tag)
        .map(|(signature, blob)| (*signature, placements[blob]))
        .collect();
    table_entries.sort_by_key(|(signature, _)| **signature);

    let mut table = Vec::with_capacity(table_len);
    table.extend_from_slice(&(signatures.len() as u32).to_be_bytes());
    for (signature, (tag_offset, length)) in table_entries {
        table.extend_from_slice(signature);
        table.extend_from_slice(&tag_offset.to_be_bytes());
        table.extend_from_slice(&length.to_be_bytes());
    }

    let total = HEADER_LEN + table.len() + body.len();
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(&(total as u32).to_be_bytes()); // profile size
    header.extend_from_slice(&[0u8; 4]); // preferred CMM: none
    header.extend_from_slice(&0x0240_0000u32.to_be_bytes()); // version 2.4
    header.extend_from_slice(b"mntr"); // device class: display
    header.extend_from_slice(b"RGB "); // data colour space
    header.extend_from_slice(b"XYZ "); // profile connection space
    header.extend_from_slice(&[0u8; 12]); // creation date/time
    header.extend_from_slice(b"acsp"); // file signature
    header.extend_from_slice(&[0u8; 4]); // primary platform
    header.extend_from_slice(&[0u8; 4]); // profile flags
    header.extend_from_slice(&[0u8; 4]); // device manufacturer
    header.extend_from_slice(&[0u8; 4]); // device model
    header.extend_from_slice(&[0u8; 8]); // device attributes
    header.extend_from_slice(&[0u8; 4]); // rendering intent: perceptual
    for channel in WHITE_D50 {
        header.extend_from_slice(&s15_fixed16(channel));
    }
    header.extend_from_slice(&[0u8; 4]); // profile creator
    header.extend_from_slice(&[0u8; 16]); // profile ID (unset)
    header.extend_from_slice(&[0u8; 28]); // reserved
    debug_assert_eq!(header.len(), HEADER_LEN);

    let mut profile = Vec::with_capacity(total);
    profile.extend_from_slice(&header);
    profile.extend_from_slice(&table);
    profile.extend_from_slice(&body);
    profile
}

// ===== Embedded input profiles =====
//
// The loader decodes JPEG/PNG/TIFF/WebP numbers as-is, but an embedded ICC
// profile changes what those numbers mean: an Adobe RGB or ProPhoto file fed
// straight into the sRGB-assuming pipeline renders desaturated (and, for
// ProPhoto's gamma 1.8, tonally wrong). The parser below handles the matrix/TRC
// profile class — which covers essentially every RGB working space a camera or
// editor embeds (sRGB, Adobe RGB, ProPhoto, Display P3, Rec.709 variants) — and
// produces a transform to sRGB. LUT-based (A2B) and non-RGB profiles parse to
// `None` and the file is treated as sRGB, which is the pre-existing behaviour.

/// A tone reproduction curve mapping encoded device values to linear light.
enum Trc {
    Linear,
    Gamma(f32),
    /// `curv` table samples, normalized to 0..=1, uniformly spaced over 0..=1.
    Table(Vec<f32>),
    /// `para` function type 0-4 with its parameters g, a, b, c, d, e, f.
    Parametric { kind: u16, p: [f64; 7] },
}

impl Trc {
    fn linearize(&self, encoded: f32) -> f32 {
        let x = encoded.clamp(0.0, 1.0);
        match self {
            Trc::Linear => x,
            Trc::Gamma(g) => x.powf(*g),
            Trc::Table(samples) => {
                let position = x as f64 * (samples.len() - 1) as f64;
                let low = position.floor() as usize;
                let high = (low + 1).min(samples.len() - 1);
                let fraction = (position - low as f64) as f32;
                samples[low] + (samples[high] - samples[low]) * fraction
            }
            Trc::Parametric { kind, p } => {
                let x = x as f64;
                let [g, a, b, c, d, e, f] = *p;
                let y = match kind {
                    0 => x.powf(g),
                    1 => {
                        if x >= -b / a { (a * x + b).powf(g) } else { 0.0 }
                    }
                    2 => {
                        if x >= -b / a { (a * x + b).powf(g) + c } else { c }
                    }
                    3 => {
                        if x >= d { (a * x + b).powf(g) } else { c * x }
                    }
                    4 => {
                        if x >= d { (a * x + b).powf(g) + e } else { c * x + f }
                    }
                    _ => x,
                };
                y.clamp(0.0, 1.0) as f32
            }
        }
    }
}

/// A decoded matrix/TRC input profile, reduced to the one operation the loader
/// needs: re-encode pixels from the tagged space into sRGB.
pub struct InputTransform {
    /// Linear device RGB -> linear sRGB. Both sides share the D50 PCS, so the
    /// adaptation cancels and no white-point term is needed.
    matrix: [[f32; 3]; 3],
    trc: [Trc; 3],
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn read_s15_fixed16(bytes: &[u8], at: usize) -> Option<f64> {
    let raw = i32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
    Some(raw as f64 / 65536.0)
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

fn parse_xyz_tag(tag: &[u8]) -> Option<[f64; 3]> {
    if tag.get(0..4)? != b"XYZ " {
        return None;
    }
    Some([
        read_s15_fixed16(tag, 8)?,
        read_s15_fixed16(tag, 12)?,
        read_s15_fixed16(tag, 16)?,
    ])
}

fn parse_trc_tag(tag: &[u8]) -> Option<Trc> {
    match tag.get(0..4)? {
        b"curv" => {
            let count = read_u32(tag, 8)? as usize;
            match count {
                0 => Some(Trc::Linear),
                1 => {
                    // A single entry is a gamma value in u8Fixed8 form.
                    let raw = u16::from_be_bytes(tag.get(12..14)?.try_into().ok()?);
                    Some(Trc::Gamma(raw as f32 / 256.0))
                }
                _ => {
                    let mut samples = Vec::with_capacity(count);
                    for index in 0..count {
                        let at = 12 + index * 2;
                        let raw = u16::from_be_bytes(tag.get(at..at + 2)?.try_into().ok()?);
                        samples.push(raw as f32 / 65535.0);
                    }
                    Some(Trc::Table(samples))
                }
            }
        }
        b"para" => {
            let kind = u16::from_be_bytes(tag.get(8..10)?.try_into().ok()?);
            let param_count = match kind {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => return None,
            };
            let mut p = [0.0f64; 7];
            for index in 0..param_count {
                p[index] = read_s15_fixed16(tag, 12 + index * 4)?;
            }
            Some(Trc::Parametric { kind, p })
        }
        _ => None,
    }
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

fn srgb_linearize(encoded: f32) -> f32 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_encode(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Parses an embedded profile into a to-sRGB transform. Returns `None` both
/// when the profile already is (close enough to) sRGB and when it is a shape
/// this parser doesn't cover — in either case the caller leaves the pixels
/// untouched, exactly as every version before input-profile support did.
pub fn parse_input_profile(profile: &[u8]) -> Option<InputTransform> {
    // Header sanity: an RGB profile against the XYZ connection space.
    if profile.len() < HEADER_LEN + 4
        || profile.get(36..40)? != b"acsp"
        || profile.get(16..20)? != b"RGB "
        || profile.get(20..24)? != b"XYZ "
    {
        return None;
    }

    let red = parse_xyz_tag(find_tag(profile, b"rXYZ")?)?;
    let green = parse_xyz_tag(find_tag(profile, b"gXYZ")?)?;
    let blue = parse_xyz_tag(find_tag(profile, b"bXYZ")?)?;
    let trc = [
        parse_trc_tag(find_tag(profile, b"rTRC")?)?,
        parse_trc_tag(find_tag(profile, b"gTRC")?)?,
        parse_trc_tag(find_tag(profile, b"bTRC")?)?,
    ];

    // Columns are the colorants; both matrices map linear RGB to D50 XYZ.
    let source = [
        [red[0], green[0], blue[0]],
        [red[1], green[1], blue[1]],
        [red[2], green[2], blue[2]],
    ];
    let srgb = [
        [PRIMARY_R[0], PRIMARY_G[0], PRIMARY_B[0]],
        [PRIMARY_R[1], PRIMARY_G[1], PRIMARY_B[1]],
        [PRIMARY_R[2], PRIMARY_G[2], PRIMARY_B[2]],
    ];
    let srgb_inverse = invert_3x3(srgb)?;

    let mut combined = [[0.0f64; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            for k in 0..3 {
                combined[row][col] += srgb_inverse[row][k] * source[k][col];
            }
        }
    }

    // Skip profiles that are sRGB in all but name: colorants within fixed-point
    // noise of the sRGB ones and a TRC tracking the sRGB curve.
    let identity_like = (0..3).all(|row| {
        (0..3).all(|col| {
            let expected = if row == col { 1.0 } else { 0.0 };
            (combined[row][col] - expected).abs() < 0.02
        })
    });
    if identity_like {
        let curve_like_srgb = trc.iter().all(|channel| {
            [0.05f32, 0.25, 0.5, 0.75, 0.95]
                .iter()
                .all(|&x| (channel.linearize(x) - srgb_linearize(x)).abs() < 0.01)
        });
        if curve_like_srgb {
            return None;
        }
    }

    let mut matrix = [[0.0f32; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            matrix[row][col] = combined[row][col] as f32;
        }
    }
    Some(InputTransform { matrix, trc })
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

/// Re-encodes `image` (non-linear values in the profile's space, 0..=1) into
/// non-linear sRGB in place. Out-of-gamut colors clip, matching what a
/// relative-colorimetric CMS does for matrix profiles.
pub fn transform_to_srgb(image: &mut image::Rgb32FImage, transform: &InputTransform) {
    use rayon::prelude::*;

    let matrix = transform.matrix;
    let samples: &mut [f32] = image.as_mut();
    samples.par_chunks_exact_mut(3).for_each(|pixel| {
        let r = transform.trc[0].linearize(pixel[0]);
        let g = transform.trc[1].linearize(pixel[1]);
        let b = transform.trc[2].linearize(pixel[2]);
        for channel in 0..3 {
            let linear =
                matrix[channel][0] * r + matrix[channel][1] * g + matrix[channel][2] * b;
            pixel[channel] = srgb_encode(linear.clamp(0.0, 1.0));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_declares_an_rgb_display_profile_sized_to_the_bytes() {
        let profile = srgb_profile();
        assert_eq!(
            u32::from_be_bytes(profile[0..4].try_into().unwrap()) as usize,
            profile.len(),
        );
        assert_eq!(&profile[12..16], b"mntr");
        assert_eq!(&profile[16..20], b"RGB ");
        assert_eq!(&profile[20..24], b"XYZ ");
        assert_eq!(&profile[36..40], b"acsp");
    }

    #[test]
    fn every_tag_is_aligned_sorted_and_inside_the_profile() {
        let profile = srgb_profile();
        let count = u32::from_be_bytes(profile[128..132].try_into().unwrap()) as usize;
        assert_eq!(count, 9);

        let mut previous = [0u8; 4];
        for index in 0..count {
            let entry = 132 + index * 12;
            let signature: [u8; 4] = profile[entry..entry + 4].try_into().unwrap();
            let offset = u32::from_be_bytes(profile[entry + 4..entry + 8].try_into().unwrap()) as usize;
            let length = u32::from_be_bytes(profile[entry + 8..entry + 12].try_into().unwrap()) as usize;

            assert!(signature > previous, "tag table must ascend by signature");
            previous = signature;
            assert_eq!(offset % 4, 0, "tag data must be 4-byte aligned");
            assert!(offset + length <= profile.len(), "tag runs past the profile");
        }
    }

    /// A minimal matrix/TRC test profile: given colorants plus one shared gamma.
    fn test_profile(red: [f64; 3], green: [f64; 3], blue: [f64; 3], gamma: f64) -> Vec<u8> {
        let mut curve = Vec::new();
        curve.extend_from_slice(b"curv");
        curve.extend_from_slice(&[0u8; 4]);
        curve.extend_from_slice(&1u32.to_be_bytes());
        curve.extend_from_slice(&((gamma * 256.0).round() as u16).to_be_bytes());
        curve.extend_from_slice(&[0u8; 2]); // pad to 4 bytes

        let tags: Vec<(&[u8; 4], Vec<u8>)> = vec![
            (b"rXYZ", xyz_tag(red)),
            (b"gXYZ", xyz_tag(green)),
            (b"bXYZ", xyz_tag(blue)),
            (b"rTRC", curve.clone()),
            (b"gTRC", curve.clone()),
            (b"bTRC", curve),
        ];

        let table_len = 4 + tags.len() * 12;
        let mut table = Vec::new();
        table.extend_from_slice(&(tags.len() as u32).to_be_bytes());
        let mut body = Vec::new();
        let mut offset = HEADER_LEN + table_len;
        for (signature, blob) in &tags {
            table.extend_from_slice(*signature);
            table.extend_from_slice(&(offset as u32).to_be_bytes());
            table.extend_from_slice(&(blob.len() as u32).to_be_bytes());
            body.extend_from_slice(blob);
            offset += blob.len();
        }

        let mut profile = vec![0u8; HEADER_LEN];
        profile[16..20].copy_from_slice(b"RGB ");
        profile[20..24].copy_from_slice(b"XYZ ");
        profile[36..40].copy_from_slice(b"acsp");
        profile.extend_from_slice(&table);
        profile.extend_from_slice(&body);
        profile
    }

    #[test]
    fn our_own_srgb_profile_parses_as_a_no_op() {
        assert!(parse_input_profile(srgb_profile()).is_none());
    }

    #[test]
    fn adobe_rgb_patches_convert_to_the_lcms_reference_values() {
        // Adobe RGB (1998) D50 colorants and gamma as Adobe ships them.
        let profile = test_profile(
            [0.60974, 0.31111, 0.01947],
            [0.20528, 0.62567, 0.06087],
            [0.14919, 0.06322, 0.74457],
            563.0 / 256.0,
        );
        let transform = parse_input_profile(&profile).expect("Adobe RGB must parse");

        let patches = [
            ([230u8, 40, 40], [255u8, 35, 35]),
            ([180, 70, 60], [207, 68, 57]),
            ([70, 150, 70], [0, 151, 61]),
            ([70, 90, 180], [57, 90, 184]),
            ([160, 160, 160], [161, 161, 161]),
        ];
        let mut image = image::Rgb32FImage::new(patches.len() as u32, 1);
        for (index, (source, _)) in patches.iter().enumerate() {
            image.put_pixel(
                index as u32,
                0,
                image::Rgb(source.map(|value| value as f32 / 255.0)),
            );
        }
        transform_to_srgb(&mut image, &transform);
        for (index, (_, expected)) in patches.iter().enumerate() {
            let pixel = image.get_pixel(index as u32, 0);
            for channel in 0..3 {
                let got = (pixel[channel] * 255.0).round();
                let want = expected[channel] as f32;
                assert!(
                    (got - want).abs() <= 2.0,
                    "patch {index} channel {channel}: got {got}, lcms says {want}"
                );
            }
        }
    }

    /// A minimal CMYK LUT test profile: Lab PCS, grid-2 mft2 A2B0, linear
    /// input/output tables, every CLUT node set by `node(c, m, y, k)`.
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
    fn cmyk_parser_rejects_rgb_profiles_and_vice_versa() {
        assert!(parse_cmyk_profile(srgb_profile()).is_none());
        let cmyk = test_cmyk_profile(|_| lab16(50.0, 0.0, 0.0));
        assert!(parse_input_profile(&cmyk).is_none());
    }

    #[test]
    fn non_rgb_and_lut_profiles_are_skipped() {
        let mut gray = srgb_profile().to_vec();
        gray[16..20].copy_from_slice(b"GRAY");
        assert!(parse_input_profile(&gray).is_none());
        assert!(parse_input_profile(&[]).is_none());
        assert!(parse_input_profile(&vec![0u8; 200]).is_none());
    }

    #[test]
    fn the_tone_curve_matches_the_srgb_transfer_function() {
        let profile = srgb_profile();
        let start = profile
            .windows(4)
            .position(|window| window == b"curv")
            .expect("curv tag present");
        let points = u32::from_be_bytes(profile[start + 8..start + 12].try_into().unwrap()) as usize;
        assert_eq!(points, TRC_POINTS);

        let sample_at = |index: usize| {
            let at = start + 12 + index * 2;
            u16::from_be_bytes(profile[at..at + 2].try_into().unwrap())
        };
        assert_eq!(sample_at(0), 0);
        assert_eq!(sample_at(TRC_POINTS - 1), 65535);
        // Mid-grey encodes to roughly 21.4% linear light under sRGB.
        let mid = sample_at(TRC_POINTS / 2) as f64 / 65535.0;
        assert!((mid - 0.2140).abs() < 0.005, "mid-grey was {mid}");
    }
}
