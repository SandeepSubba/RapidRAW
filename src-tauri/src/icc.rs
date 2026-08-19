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
