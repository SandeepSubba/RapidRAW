use crate::app::settings::AppSettings;
use rawler::bits::Endian;
use rawler::decoders::{Decoder, FormatHint, WellKnownIFD};
use rawler::formats::bmff::Bmff;
use rawler::formats::tiff::reader::TiffReader;
use rawler::formats::tiff::{IFD, Value, ifd::OffsetMode};
use rawler::tags::{DngTag, ExifTag};
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

const HIGHLIGHT_BIAS_MIN: f32 = -1.0;
const HIGHLIGHT_BIAS_MAX: f32 = 4.0;

const CANON_LIGHTING_OPT: u16 = 0x4018;
const FUJI_DEVELOPMENT_DYNAMIC_RANGE: u16 = 0x1403;
const FUJI_AUTO_DYNAMIC_RANGE: u16 = 0x140b;
const NIKON_COLOR_SPACE: u16 = 0x001e;
const NIKON_ACTIVE_D_LIGHTING: u16 = 0x0022;
const OLYMPUS_CAMERA_SETTINGS: u16 = 0x2020;
const OLYMPUS_GRADATION: u16 = 0x050f;
const PENTAX_DYNAMIC_RANGE_EXPANSION: u16 = 0x0069;

const RAF_JPEG_OFFSET_POSITION: usize = 84;
const RAF_JPEG_TIFF_HEADER: u32 = 12;
const FUJI_MAKERNOTE_HEADER: &[u8] = b"FUJIFILM";
const FUJI_MAKERNOTE_IFD_OFFSET: u32 = 12;

#[derive(Clone, Copy, Debug, Default)]
pub struct RawExposure {
    pub scene_referred: bool,
    pub dng_baseline: Option<f32>,
    pub highlight_preservation: f32,
}

impl RawExposure {
    pub fn compensation(&self, options: BaselineOptions) -> f32 {
        let baseline = options
            .baseline_exposure
            .map_or(0.0, |amount| match self.dng_baseline {
                Some(dng_baseline) => dng_baseline,
                None if self.scene_referred => amount,
                None => 0.0,
            });
        let highlight_preservation = if options.compensate_highlight_preservation {
            self.highlight_preservation
        } else {
            0.0
        };
        baseline + highlight_preservation
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BaselineOptions {
    pub baseline_exposure: Option<f32>,
    pub compensate_highlight_preservation: bool,
}

impl BaselineOptions {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            baseline_exposure: settings
                .enable_baseline_exposure
                .then_some(settings.baseline_exposure),
            compensate_highlight_preservation: settings.compensate_highlight_preservation,
        }
    }
}

pub fn read(decoder: &dyn Decoder, file_bytes: &[u8], is_monochrome: bool) -> RawExposure {
    let highlight_preservation = catch_unwind(AssertUnwindSafe(|| {
        highlight_preservation(decoder.format_hint(), file_bytes)
    }))
    .unwrap_or(0.0);
    RawExposure {
        scene_referred: !is_monochrome,
        dng_baseline: dng_baseline(decoder).or_else(|| {
            // Fuji's RawExposureBias is the camera's own baseline: it replaces
            // the generic scene-referred amount (like a DNG tag), never adds to it.
            matches!(decoder.format_hint(), FormatHint::RAF)
                .then(|| fuji_raw_exposure_bias(file_bytes).map(|bias| -bias))
                .flatten()
        }),
        highlight_preservation: highlight_preservation
            .clamp(HIGHLIGHT_BIAS_MIN, HIGHLIGHT_BIAS_MAX),
    }
}

fn dng_baseline(decoder: &dyn Decoder) -> Option<f32> {
    let tags = decoder.ifd(WellKnownIFD::VirtualDngRootTags).ok()??;
    [DngTag::BaselineExposure, DngTag::BaselineExposureOffset]
        .into_iter()
        .filter_map(|tag| tags.get_entry(tag)?.value.get_f32(0).ok().flatten())
        .reduce(|total, value| total + value)
}

fn highlight_preservation(format: FormatHint, file_bytes: &[u8]) -> f32 {
    match format {
        FormatHint::CR2 => tiff_makernote(file_bytes).map(|makernote| canon(&makernote)),
        FormatHint::CR3 => Bmff::new_buf(file_bytes)
            .ok()
            .and_then(|bmff| bmff.filebox.moov.cr3desc)
            .map(|desc| canon(desc.cmt3.tiff.root_ifd())),
        FormatHint::NEF | FormatHint::NRW => {
            tiff_makernote(file_bytes).map(|makernote| nikon(&makernote))
        }
        FormatHint::PEF => tiff_makernote(file_bytes).map(|makernote| pentax(&makernote)),
        FormatHint::ORF => olympus_camera_settings(file_bytes).map(|settings| olympus(&settings)),
        FormatHint::RAF => fuji_makernote(file_bytes).map(|makernote| fuji(&makernote)),
        _ => None,
    }
    .unwrap_or(0.0)
}

const RAF_META_OFFSET_POSITION: usize = 92;
const RAF_RAW_EXPOSURE_BIAS: u16 = 0x9650;

/// Fuji's RawExposureBias (RAF metadata tag 0x9650): how much darker the camera
/// recorded the RAW than its own JPEG rendering — e.g. -0.72 EV on an X-T5 at
/// DR100. Adobe's Fuji profiles use it as the baseline exposure, so RAFs use it
/// in place of the generic scene-referred baseline (adding both over-brightens
/// by ~0.7 EV). The DR200/DR400 boost stays in highlight preservation.
/// The RAF metadata block is big-endian: a record count, then tag/size/data.
fn fuji_raw_exposure_bias(file_bytes: &[u8]) -> Option<f32> {
    let be_u32 = |at: usize| Some(u32::from_be_bytes(file_bytes.get(at..at + 4)?.try_into().ok()?));
    let be_u16 = |at: usize| Some(u16::from_be_bytes(file_bytes.get(at..at + 2)?.try_into().ok()?));
    let meta = be_u32(RAF_META_OFFSET_POSITION)? as usize;
    let count = be_u32(meta)?;
    let mut at = meta + 4;
    for _ in 0..count.min(256) {
        let (tag, size) = (be_u16(at)?, be_u16(at + 2)? as usize);
        if tag == RAF_RAW_EXPOSURE_BIAS && size >= 4 {
            let numerator = be_u16(at + 4)? as i16 as f32;
            let denominator = be_u16(at + 6)? as f32;
            return (denominator > 0.0).then(|| numerator / denominator);
        }
        at += 4 + size;
    }
    None
}

fn exif_with_makernote(cursor: &mut Cursor<&[u8]>, tiff_offset: u32) -> Option<IFD> {
    IFD::new_root(cursor, tiff_offset)
        .ok()?
        .find_first_ifd_with_tag(ExifTag::MakerNotes)
        .cloned()
}

fn makernote_start(exif: &IFD) -> Option<u32> {
    Some(exif.base + exif.get_entry(ExifTag::MakerNotes)?.offset()? as u32)
}

fn tiff_makernote(file_bytes: &[u8]) -> Option<IFD> {
    let mut cursor = Cursor::new(file_bytes);
    exif_with_makernote(&mut cursor, 0)?
        .parse_makernote(&mut cursor, OffsetMode::Absolute, &[])
        .ok()?
}

fn fuji_makernote(file_bytes: &[u8]) -> Option<IFD> {
    let jpeg_offset = u32::from_be_bytes(
        file_bytes
            .get(RAF_JPEG_OFFSET_POSITION..RAF_JPEG_OFFSET_POSITION + 4)?
            .try_into()
            .ok()?,
    );
    let mut cursor = Cursor::new(file_bytes);
    let exif = exif_with_makernote(&mut cursor, jpeg_offset + RAF_JPEG_TIFF_HEADER)?;
    if !matches!(
        exif.get_entry(ExifTag::MakerNotes).map(|entry| &entry.value),
        Some(Value::Undefined(data)) if data.starts_with(FUJI_MAKERNOTE_HEADER)
    ) {
        return None;
    }
    IFD::new(
        &mut cursor,
        FUJI_MAKERNOTE_IFD_OFFSET,
        makernote_start(&exif)?,
        0,
        Endian::Little,
        &[],
    )
    .ok()
}

fn olympus_camera_settings(file_bytes: &[u8]) -> Option<IFD> {
    let mut cursor = Cursor::new(file_bytes);
    let exif = exif_with_makernote(&mut cursor, 0)?;
    let makernote = rawler::decoders::orf::parse_makernote(&mut cursor, &exif).ok()??;
    let settings_offset = makernote
        .get_entry_raw_with_len(OLYMPUS_CAMERA_SETTINGS, &mut cursor, 4)
        .ok()??
        .get_force_u32(0);
    IFD::new(
        &mut cursor,
        settings_offset,
        makernote_start(&exif)?,
        0,
        makernote.endian,
        &[],
    )
    .ok()
}

fn integers(ifd: &IFD, tag: u16) -> Vec<i32> {
    ifd.get_entry(tag)
        .map(|entry| {
            (0..entry.value.count())
                .map_while(|index| entry.value.get_i32(index).ok().flatten())
                .collect()
        })
        .unwrap_or_default()
}

fn first_integer(ifd: &IFD, tag: u16) -> Option<i32> {
    ifd.get_entry(tag)?.value.get_i32(0).ok().flatten()
}

fn canon(makernote: &IFD) -> f32 {
    let lighting_opt = integers(makernote, CANON_LIGHTING_OPT);
    let auto_lighting_optimizer = match lighting_opt.get(2) {
        Some(0) => 0.5,
        Some(1) => 0.33,
        Some(2) => 0.66,
        _ => 0.0,
    };
    let highlight_tone_priority = match lighting_opt.get(3) {
        Some(&state) if state > 0 => 1.0,
        _ => 0.0,
    };
    auto_lighting_optimizer + highlight_tone_priority
}

fn fuji(makernote: &IFD) -> f32 {
    match [FUJI_DEVELOPMENT_DYNAMIC_RANGE, FUJI_AUTO_DYNAMIC_RANGE]
        .into_iter()
        .find_map(|tag| first_integer(makernote, tag))
    {
        Some(200) => 1.0,
        Some(400) => 2.0,
        _ => 0.0,
    }
}

fn nikon(makernote: &IFD) -> f32 {
    if first_integer(makernote, NIKON_COLOR_SPACE) == Some(4) {
        return 2.0;
    }
    match first_integer(makernote, NIKON_ACTIVE_D_LIGHTING) {
        Some(3) => 0.33,
        Some(5) => 0.66,
        Some(7) => 1.0,
        Some(8) => 1.1,
        Some(9) => 1.2,
        Some(10) => 1.3,
        Some(11) => 1.33,
        _ => 0.0,
    }
}

fn olympus(camera_settings: &IFD) -> f32 {
    match integers(camera_settings, OLYMPUS_GRADATION).as_slice() {
        [_, _, _, 1, ..] => 0.33,
        [-1, -1, 1, ..] => 0.66,
        _ => 0.0,
    }
}

fn pentax(makernote: &IFD) -> f32 {
    match makernote
        .get_entry(PENTAX_DYNAMIC_RANGE_EXPANSION)
        .map(|entry| &entry.value)
    {
        Some(Value::Undefined(data)) if data.first() == Some(&1) => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raf_with_meta(records: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = vec![0u8; 100];
        let meta = bytes.len() as u32;
        bytes[RAF_META_OFFSET_POSITION..RAF_META_OFFSET_POSITION + 4].copy_from_slice(&meta.to_be_bytes());
        bytes.extend((records.len() as u32).to_be_bytes());
        for (tag, data) in records {
            bytes.extend(tag.to_be_bytes());
            bytes.extend((data.len() as u16).to_be_bytes());
            bytes.extend(data);
        }
        bytes
    }

    #[test]
    fn reads_fuji_raw_exposure_bias_after_other_records() {
        let bias = [(-72i16).to_be_bytes(), 100u16.to_be_bytes()].concat();
        let bytes = raf_with_meta(&[(0x0100, vec![0, 1, 2, 3, 4, 5]), (RAF_RAW_EXPOSURE_BIAS, bias)]);
        assert!((fuji_raw_exposure_bias(&bytes).unwrap() + 0.72).abs() < 1e-6);
    }

    #[test]
    fn missing_or_truncated_bias_is_none() {
        assert_eq!(fuji_raw_exposure_bias(&raf_with_meta(&[(0x0100, vec![1, 2])])), None);
        assert_eq!(fuji_raw_exposure_bias(&[0u8; 40]), None);
        let mut cut = raf_with_meta(&[(RAF_RAW_EXPOSURE_BIAS, vec![0xff, 0xb8, 0, 100])]);
        cut.truncate(cut.len() - 2);
        assert_eq!(fuji_raw_exposure_bias(&cut), None);
    }

    /// `RAF_SAMPLE=<file.RAF> cargo test --lib baseline_exposure -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_raf() {
        let bytes = std::fs::read(std::env::var("RAF_SAMPLE").unwrap()).unwrap();
        println!("RawExposureBias {:?}", fuji_raw_exposure_bias(&bytes));
        println!("highlight preservation {:+.2} EV", highlight_preservation(FormatHint::RAF, &bytes));
    }

    #[test]
    fn vendor_baseline_replaces_generic_amount() {
        let options = BaselineOptions { baseline_exposure: Some(0.7), compensate_highlight_preservation: true };
        let raf = RawExposure { scene_referred: true, dng_baseline: Some(0.72), highlight_preservation: 1.0 };
        assert!((raf.compensation(options) - 1.72).abs() < 1e-6);
        let off = BaselineOptions { baseline_exposure: None, ..options };
        assert!((raf.compensation(off) - 1.0).abs() < 1e-6);
    }
}
