use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use crate::formats::is_raw_file;
use crate::image_processing::ImageMetadata;
use chrono::{DateTime, NaiveDateTime, Utc};
use exif::{Exif, In, Value};
use little_exif::exif_tag::ExifTag;
use little_exif::filetype::FileExtension;
use little_exif::metadata::Metadata;
use little_exif::rational::{iR64, uR64};
use rawler::decoders::RawMetadata;

/// Decode an EXIF `UserComment` (tag 0x9286) into displayable text.
///
/// Per the EXIF spec the value is an 8-byte character-code header
/// (`ASCII\0\0\0`, `UNICODE\0`, `JIS\0\0\0\0\0`, or all-zero = undefined)
/// followed by the comment. Cameras such as the Canon 5D Mark III allocate the
/// buffer but leave it padded with NUL bytes; returning `None` for those keeps
/// the "Comments" field empty instead of showing a hex dump of zero bytes.
fn decode_user_comment(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    let (code, payload) = if bytes.len() >= 8 {
        bytes.split_at(8)
    } else {
        (&[][..], bytes)
    };

    let text: String = if code == b"UNICODE\0" {
        let decode = |big_endian: bool| -> String {
            let units = payload.chunks_exact(2).map(|c| {
                if big_endian {
                    u16::from_be_bytes([c[0], c[1]])
                } else {
                    u16::from_le_bytes([c[0], c[1]])
                }
            });
            char::decode_utf16(units)
                .map(|r| r.unwrap_or('\u{FFFD}'))
                .collect()
        };
        // EXIF doesn't record the UTF-16 byte order, so keep whichever decode
        // produced fewer replacement characters.
        let le = decode(false);
        let be = decode(true);
        if be.matches('\u{FFFD}').count() < le.matches('\u{FFFD}').count() {
            be
        } else {
            le
        }
    } else {
        // ASCII / JIS / undefined: treat the payload as Latin-1 bytes.
        payload.iter().map(|&b| b as char).collect()
    };

    let trimmed = text.trim_matches(|c: char| c == '\u{0}' || c.is_whitespace());
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Repair a `UserComment` value cached by an older import that stored the raw
/// EXIF bytes as a hex dump (e.g. `0x000000…` for the Canon empty-comment
/// buffer, or `0x415343494900…` for a real comment). The hex is parsed back
/// into bytes and decoded properly: real text replaces the dump, empty buffers
/// are dropped. Returns `true` if `map` was modified.
fn heal_cached_user_comment(map: &mut HashMap<String, String>) -> bool {
    let Some(raw) = map.get("UserComment") else {
        return false;
    };

    // Only touch the hex-dump artifact `0x<hex>`. Large buffers are stored
    // truncated by `truncate_large_exif` (`0x000…000`), so drop the `...` marker.
    let Some(body) = raw.strip_prefix("0x") else {
        return false;
    };
    let truncated = body.contains("...");
    let hex = body.replace("...", "");
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }

    // An all-zero dump is Canon's empty padding buffer regardless of truncation.
    if hex.bytes().all(|b| b == b'0') {
        map.remove("UserComment");
        return true;
    }

    // A truncated non-zero dump can't be reconstructed exactly; leave it rather
    // than risk corrupting a genuine comment.
    if truncated || hex.len() % 2 != 0 {
        return false;
    }

    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect();

    match decode_user_comment(&bytes) {
        Some(comment) => {
            map.insert("UserComment".to_string(), comment);
        }
        None => {
            map.remove("UserComment");
        }
    }
    true
}

pub fn truncate_large_exif(value: &str) -> String {
    if value.len() <= 500 {
        return value.to_string();
    }

    let mut start_idx = 200;
    while !value.is_char_boundary(start_idx) {
        start_idx -= 1;
    }

    let mut end_idx = value.len() - 200;
    while !value.is_char_boundary(end_idx) {
        end_idx += 1;
    }

    if start_idx < end_idx {
        let start_str = &value[..start_idx];
        let end_str = &value[end_idx..];
        return format!("{}...{}", start_str, end_str);
    }

    value.to_string()
}

pub fn load_sidecar(sidecar_path: &Path) -> ImageMetadata {
    if !sidecar_path.exists() {
        return ImageMetadata::default();
    }

    let Ok(content) = fs::read_to_string(sidecar_path) else {
        return ImageMetadata::default();
    };

    let mut meta = serde_json::from_str::<ImageMetadata>(&content).unwrap_or_default();
    let mut healed = false;

    if let Some(ref mut exif_map) = meta.exif {
        for val in exif_map.values_mut() {
            if val.len() > 500 {
                *val = truncate_large_exif(val);
                healed = true;
            }
        }
    }

    if healed && let Ok(json) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(sidecar_path, json);
        log::info!(
            "Auto-healed bloated sidecar for: {}",
            sidecar_path.display()
        );
    }

    meta
}

fn to_ur64(val: &exif::Rational) -> uR64 {
    uR64 {
        nominator: val.num,
        denominator: val.denom,
    }
}

fn to_ir64(val: &exif::SRational) -> iR64 {
    iR64 {
        nominator: val.num,
        denominator: val.denom,
    }
}

fn clean_creation_datetime_str(s: &str) -> &str {
    s.trim().trim_matches('"').trim_matches('\'').trim()
}

fn fmt_date_str(s: String) -> String {
    if let Some(dt) = parse_creation_datetime(&s) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    clean_creation_datetime_str(&s).to_string()
}

fn normalize_creation_datetime(s: &str) -> Option<String> {
    let normalized = s.replace('T', " ");
    let (date, time) = normalized.split_once(' ')?;
    Some(format!("{} {}", date.replace(':', "-"), time))
}

fn parse_creation_datetime(s: &str) -> Option<NaiveDateTime> {
    let clean = clean_creation_datetime_str(s);
    if clean.is_empty() {
        return None;
    }

    let normalized = normalize_creation_datetime(clean);
    for candidate in std::iter::once(clean).chain(normalized.as_deref()) {
        for format in [
            "%Y:%m:%d %H:%M:%S",
            "%Y:%m:%d %H:%M:%S%.f",
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M:%S%.f",
        ] {
            if let Ok(dt) = NaiveDateTime::parse_from_str(candidate, format) {
                return Some(dt);
            }
        }
    }

    None
}

fn parse_creation_field(field: &exif::Field) -> Option<DateTime<Utc>> {
    parse_creation_datetime(&field.display_value().to_string())
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

fn parse_raw_creation_date(date_str: Option<&str>) -> Option<DateTime<Utc>> {
    parse_creation_datetime(date_str?).map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

fn clean_ascii_value(value: &exif::Value) -> Option<String> {
    let exif::Value::Ascii(ref components) = *value else {
        return None;
    };

    let cleaned: Vec<String> = components
        .iter()
        .map(|c| {
            String::from_utf8_lossy(c)
                .trim_matches(char::from(0))
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.join(" "))
    }
}

fn rational_to_f32_checked(r: &exif::Rational) -> Option<f32> {
    if r.denom == 0 {
        None
    } else {
        Some(r.num as f32 / r.denom as f32)
    }
}

fn rawler_rational_to_f32_checked(r: &rawler::formats::tiff::Rational) -> Option<f32> {
    if r.d == 0 {
        None
    } else {
        Some(r.n as f32 / r.d as f32)
    }
}

fn format_min_max(min: f32, max: f32, tolerance: f32) -> String {
    if (min - max).abs() < tolerance {
        format!("{min}")
    } else {
        format!("{min}-{max}")
    }
}

fn format_lens_specification(components: &[exif::Rational]) -> Option<String> {
    if components.len() < 4 {
        return None;
    }

    let focal_min = rational_to_f32_checked(&components[0]);
    let focal_max = rational_to_f32_checked(&components[1]);
    let (focal_min, focal_max) = match (focal_min, focal_max) {
        (Some(min), Some(max)) => (min, max),
        _ => return None,
    };

    let mut spec = format!("{} mm", format_min_max(focal_min, focal_max, 0.01));

    let aperture_min = rational_to_f32_checked(&components[2]);
    let aperture_max = rational_to_f32_checked(&components[3]);
    if let (Some(amin), Some(amax)) = (aperture_min, aperture_max) {
        spec.push_str(&format!(", f/{}", format_min_max(amin, amax, 0.01)));
    }

    Some(spec)
}

pub fn read_exif(file_bytes: &[u8]) -> Option<Exif> {
    let exifreader = exif::Reader::new();
    exifreader
        .read_from_container(&mut Cursor::new(file_bytes))
        .ok()
}

pub fn read_raw_metadata(file_bytes: &[u8]) -> Option<RawMetadata> {
    let loader = rawler::RawLoader::new();
    let raw_source = rawler::rawsource::RawSource::new_from_slice(file_bytes);
    let decoder = loader.get_decoder(&raw_source).ok()?;
    decoder.raw_metadata(&raw_source, &Default::default()).ok()
}

pub fn read_exposure_time_secs(path: &str, file_bytes: &[u8]) -> Option<f32> {
    if let Some(map) = read_rrexif_sidecar(Path::new(path))
        && let Some(val_str) = map.get("ExposureTime").or(map.get("ShutterSpeedValue"))
    {
        let cleaned = val_str.replace(" s", "");
        if cleaned.contains('/') {
            let parts: Vec<&str> = cleaned.split('/').collect();
            if parts.len() == 2
                && let (Ok(num), Ok(den)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>())
                && den != 0.0
            {
                return Some(num / den);
            }
        } else if let Ok(val) = cleaned.parse::<f32>() {
            return Some(val);
        }
    }

    if is_raw_file(path)
        && let Some(meta) = read_raw_metadata(file_bytes)
    {
        if let Some(r) = meta.exif.exposure_time {
            return if r.d == 0 {
                None
            } else {
                Some(r.n as f32 / r.d as f32)
            };
        } else if let Some(r) = meta.exif.shutter_speed_value {
            return if r.d == 0 {
                None
            } else {
                Some(r.n as f32 / r.d as f32)
            };
        }
    }

    if let Some(exif) = read_exif(file_bytes) {
        if let Some(exposure) = exif.get_field(exif::Tag::ExposureTime, In::PRIMARY) {
            if let Value::Rational(ref r) = exposure.value {
                if r.is_empty() {
                    return None;
                }

                let val = r.first()?;

                return if val.denom == 0 {
                    None
                } else {
                    Some(val.num as f32 / val.denom as f32)
                };
            }
        } else if let Some(shutter_speed) =
            exif.get_field(exif::Tag::ShutterSpeedValue, In::PRIMARY)
            && let Value::Rational(ref r) = shutter_speed.value
        {
            if r.is_empty() {
                return None;
            }

            let val = r.first()?;

            return if val.denom == 0 {
                None
            } else {
                Some(val.num as f32 / val.denom as f32)
            };
        }
    }
    None
}

pub fn read_iso(path: &str, file_bytes: &[u8]) -> Option<u32> {
    if let Some(map) = read_rrexif_sidecar(Path::new(path))
        && let Some(val_str) = map
            .get("ISOSpeed")
            .or(map.get("PhotographicSensitivity"))
            .or(map.get("ISOSpeedRatings"))
        && let Ok(val) = val_str.parse::<u32>()
    {
        return Some(val);
    }

    if is_raw_file(path)
        && let Some(meta) = read_raw_metadata(file_bytes)
    {
        if let Some(r) = meta.exif.iso_speed {
            return Some(r);
        } else if let Some(r) = meta.exif.iso_speed_ratings {
            return Some(r as u32);
        }
    }

    if let Some(exif) = read_exif(file_bytes) {
        if let Some(r) = exif.get_field(exif::Tag::ISOSpeed, In::PRIMARY) {
            return r.value.get_uint(0);
        } else if let Some(r) = exif.get_field(exif::Tag::PhotographicSensitivity, In::PRIMARY) {
            return r.value.get_uint(0);
        }
    }
    None
}

pub fn extract_metadata(file_bytes: &[u8]) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();

    if let Some(exif_obj) = read_exif(file_bytes) {
        for field in exif_obj.fields() {
            match field.tag {
                exif::Tag::ExposureTime => {
                    if let exif::Value::Rational(ref v) = field.value
                        && !v.is_empty()
                    {
                        let r = &v[0];
                        if r.num == 1 && r.denom > 1 {
                            map.insert("ExposureTime".to_string(), format!("1/{} s", r.denom));
                        } else {
                            let val = r.num as f32 / r.denom as f32;
                            if val < 1.0 && val > 0.0 {
                                map.insert(
                                    "ExposureTime".to_string(),
                                    format!("1/{} s", (1.0 / val).round()),
                                );
                            } else {
                                map.insert("ExposureTime".to_string(), format!("{} s", val));
                            }
                        }
                    }
                }
                exif::Tag::ShutterSpeedValue => {
                    if let exif::Value::SRational(ref v) = field.value
                        && !v.is_empty()
                    {
                        let val = v[0].num as f32 / v[0].denom as f32;
                        map.insert("ShutterSpeedValue".to_string(), val.to_string());
                    }
                }
                exif::Tag::FNumber => {
                    if let exif::Value::Rational(ref v) = field.value
                        && !v.is_empty()
                    {
                        let val = v[0].num as f32 / v[0].denom as f32;
                        map.insert("FNumber".to_string(), format!("f/{}", val));
                    }
                }
                exif::Tag::ApertureValue => {
                    if let exif::Value::Rational(ref v) = field.value
                        && !v.is_empty()
                    {
                        let val = v[0].num as f32 / v[0].denom as f32;
                        map.insert("ApertureValue".to_string(), format!("f/{}", val));
                    }
                }
                exif::Tag::FocalLength => {
                    if let exif::Value::Rational(ref v) = field.value
                        && !v.is_empty()
                    {
                        let val = v[0].num as f32 / v[0].denom as f32;
                        map.insert("FocalLength".to_string(), val.to_string());
                        map.insert("FocalLengthIn35mmFilm".to_string(), val.to_string());
                    }
                }
                exif::Tag::PhotographicSensitivity | exif::Tag::ISOSpeed => {
                    map.insert(
                        "PhotographicSensitivity".to_string(),
                        field.display_value().to_string(),
                    );
                    map.insert("ISOSpeed".to_string(), field.display_value().to_string());
                }
                exif::Tag::DateTimeOriginal => {
                    map.insert(
                        "DateTimeOriginal".to_string(),
                        fmt_date_str(field.display_value().to_string()),
                    );
                }
                exif::Tag::DateTime => {
                    map.insert(
                        "CreateDate".to_string(),
                        fmt_date_str(field.display_value().to_string()),
                    );
                }
                exif::Tag::DateTimeDigitized => {
                    map.insert(
                        "ModifyDate".to_string(),
                        fmt_date_str(field.display_value().to_string()),
                    );
                }
                exif::Tag::UserComment => {
                    let decoded = match field.value {
                        // Spec-compliant: UNDEFINED with an 8-byte charset header.
                        exif::Value::Undefined(ref v, _) => decode_user_comment(v),
                        // Non-standard ASCII writers store the text directly (no header).
                        exif::Value::Ascii(ref v) => {
                            let s: String = v.iter().flatten().map(|&b| b as char).collect();
                            let t = s.trim_matches(|c: char| c == '\u{0}' || c.is_whitespace());
                            if t.is_empty() {
                                None
                            } else {
                                Some(t.to_string())
                            }
                        }
                        _ => None,
                    };
                    if let Some(comment) = decoded {
                        map.insert("UserComment".to_string(), comment);
                    }
                }
                exif::Tag::LensSpecification => {
                    if let exif::Value::Rational(ref v) = field.value
                        && v.len() >= 4
                        && let (Some(focal_min), Some(focal_max)) = (
                            rational_to_f32_checked(&v[0]),
                            rational_to_f32_checked(&v[1]),
                        )
                    {
                        let mut spec = format!("{} mm", format_min_max(focal_min, focal_max, 0.01));

                        let aperture = match (
                            rational_to_f32_checked(&v[2]),
                            rational_to_f32_checked(&v[3]),
                        ) {
                            (Some(amin), Some(amax)) => Some((amin, amax)),
                            _ => read_raw_metadata(file_bytes).and_then(|meta| {
                                let lens_desc = meta.lens?;
                                let amin =
                                    rawler_rational_to_f32_checked(&lens_desc.aperture_range[0])?;
                                let amax =
                                    rawler_rational_to_f32_checked(&lens_desc.aperture_range[1])?;
                                Some((amin, amax))
                            }),
                        };

                        if let Some((amin, amax)) = aperture
                            && (amin > 0.0 || amax > 0.0)
                        {
                            spec.push_str(&format!(", f/{}", format_min_max(amin, amax, 0.01)));
                        }

                        map.insert("LensSpecification".to_string(), spec);
                    }
                }
                _ => match &field.value {
                    exif::Value::Ascii(_) => {
                        if let Some(val) = clean_ascii_value(&field.value) {
                            map.insert(field.tag.to_string(), val);
                        }
                    }
                    _ => {
                        let val = field.display_value().with_unit(&exif_obj).to_string();
                        if !val.trim().is_empty() {
                            map.insert(field.tag.to_string(), val);
                        }
                    }
                },
            }
        }
    }

    if !map.is_empty() {
        return Some(map);
    }

    let metadata = read_raw_metadata(file_bytes)?;

    let exif = metadata.exif;

    let fmt_rat = |r: &rawler::formats::tiff::Rational| -> f32 {
        if r.d == 0 {
            0.0
        } else {
            r.n as f32 / r.d as f32
        }
    };

    let fmt_srat = |r: &rawler::formats::tiff::SRational| -> f32 {
        if r.d == 0 {
            0.0
        } else {
            r.n as f32 / r.d as f32
        }
    };

    let mut insert_if_present = |key: &str, val: String| {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            map.insert(key.to_string(), truncate_large_exif(trimmed));
        }
    };

    insert_if_present("Make", metadata.make);
    insert_if_present("Model", metadata.model);

    if let Some(v) = exif.artist {
        insert_if_present("Artist", v);
    }
    if let Some(v) = exif.copyright {
        insert_if_present("Copyright", v);
    }
    if let Some(v) = exif.owner_name {
        insert_if_present("OwnerName", v);
    }
    if let Some(v) = exif.serial_number {
        insert_if_present("SerialNumber", v);
    }
    if let Some(v) = exif.image_number {
        insert_if_present("ImageNumber", v.to_string());
    }
    if let Some(v) = exif.user_comment {
        let cleaned = v.trim_matches(|c: char| c == '\u{0}' || c.is_whitespace());
        if !cleaned.is_empty() {
            insert_if_present("UserComment", cleaned.to_string());
        }
    }

    if let Some(v) = exif.date_time_original {
        insert_if_present("DateTimeOriginal", fmt_date_str(v));
    }
    if let Some(v) = exif.create_date {
        insert_if_present("CreateDate", fmt_date_str(v));
    }
    if let Some(v) = exif.modify_date {
        insert_if_present("ModifyDate", fmt_date_str(v));
    }

    if let Some(v) = exif.offset_time {
        insert_if_present("OffsetTime", v);
    }
    if let Some(v) = exif.offset_time_original {
        insert_if_present("OffsetTimeOriginal", v);
    }
    if let Some(v) = exif.offset_time_digitized {
        insert_if_present("OffsetTimeDigitized", v);
    }
    if let Some(v) = exif.sub_sec_time {
        insert_if_present("SubSecTime", v);
    }
    if let Some(v) = exif.sub_sec_time_original {
        insert_if_present("SubSecTimeOriginal", v);
    }
    if let Some(v) = exif.sub_sec_time_digitized {
        insert_if_present("SubSecTimeDigitized", v);
    }

    if let Some(v) = exif.lens_model {
        insert_if_present("LensModel", v);
    } else if let Some(lens_desc) = &metadata.lens {
        insert_if_present("LensModel", lens_desc.lens_model.clone());
    }

    if let Some(v) = exif.lens_make {
        insert_if_present("LensMake", v);
    } else if let Some(lens_desc) = &metadata.lens {
        insert_if_present("LensMake", lens_desc.lens_make.clone());
    }

    if let Some(v) = exif.lens_serial_number {
        insert_if_present("LensSerialNumber", v);
    }

    if let Some(lens_desc) = &metadata.lens {
        let focal_min = fmt_rat(&lens_desc.focal_range[0]);
        let focal_max = fmt_rat(&lens_desc.focal_range[1]);
        let mut spec = format!("{} mm", format_min_max(focal_min, focal_max, 0.01));

        let aperture_min = fmt_rat(&lens_desc.aperture_range[0]);
        let aperture_max = fmt_rat(&lens_desc.aperture_range[1]);
        if aperture_min > 0.0 || aperture_max > 0.0 {
            spec.push_str(&format!(
                ", f/{}",
                format_min_max(aperture_min, aperture_max, 0.01)
            ));
        }

        insert_if_present("LensSpecification", spec);
    }

    if let Some(v) = exif.orientation {
        insert_if_present("Orientation", v.to_string());
    }

    if let Some(r) = exif.fnumber {
        let val = fmt_rat(&r);
        insert_if_present("FNumber", format!("f/{}", val));
    }

    if let Some(r) = exif.aperture_value {
        let val = fmt_rat(&r);
        insert_if_present("ApertureValue", format!("f/{}", val));
    }

    if let Some(r) = exif.max_aperture_value {
        insert_if_present("MaxApertureValue", fmt_rat(&r).to_string());
    }

    if let Some(r) = exif.exposure_time {
        if r.n == 1 && r.d > 1 {
            insert_if_present("ExposureTime", format!("1/{} s", r.d));
        } else {
            let val = fmt_rat(&r);
            if val < 1.0 && val > 0.0 {
                insert_if_present("ExposureTime", format!("1/{} s", (1.0 / val).round()));
            } else {
                insert_if_present("ExposureTime", format!("{} s", val));
            }
        }
    }

    if let Some(r) = exif.shutter_speed_value {
        insert_if_present("ShutterSpeedValue", fmt_srat(&r).to_string());
    }

    if let Some(v) = exif.iso_speed {
        insert_if_present("PhotographicSensitivity", v.to_string());
        insert_if_present("ISOSpeed", v.to_string());
    } else if let Some(v) = exif.iso_speed_ratings {
        insert_if_present("PhotographicSensitivity", v.to_string());
        insert_if_present("ISOSpeedRatings", v.to_string());
    }

    if let Some(v) = exif.recommended_exposure_index {
        insert_if_present("RecommendedExposureIndex", v.to_string());
    }
    if let Some(v) = exif.sensitivity_type {
        insert_if_present("SensitivityType", v.to_string());
    }

    if let Some(r) = exif.focal_length {
        let val = fmt_rat(&r);
        insert_if_present("FocalLength", val.to_string());
        insert_if_present("FocalLengthIn35mmFilm", val.to_string());
    }

    if let Some(r) = exif.exposure_bias {
        insert_if_present("ExposureBiasValue", fmt_srat(&r).to_string());
    }

    if let Some(v) = exif.metering_mode {
        insert_if_present("MeteringMode", v.to_string());
    }
    if let Some(v) = exif.light_source {
        insert_if_present("LightSource", v.to_string());
    }
    if let Some(v) = exif.flash {
        insert_if_present("Flash", v.to_string());
    }
    if let Some(v) = exif.white_balance {
        insert_if_present("WhiteBalance", v.to_string());
    }
    if let Some(v) = exif.exposure_program {
        insert_if_present("ExposureProgram", v.to_string());
    }
    if let Some(v) = exif.exposure_mode {
        insert_if_present("ExposureMode", v.to_string());
    }
    if let Some(v) = exif.scene_capture_type {
        insert_if_present("SceneCaptureType", v.to_string());
    }
    if let Some(v) = exif.color_space {
        insert_if_present("ColorSpace", v.to_string());
    }
    if let Some(r) = exif.flash_energy {
        insert_if_present("FlashEnergy", fmt_rat(&r).to_string());
    }
    if let Some(r) = exif.brightness_value {
        insert_if_present("BrightnessValue", fmt_srat(&r).to_string());
    }

    if let Some(r) = exif.subject_distance {
        insert_if_present("SubjectDistance", fmt_rat(&r).to_string());
    }
    if let Some(v) = exif.subject_distance_range {
        insert_if_present("SubjectDistanceRange", v.to_string());
    }

    if let Some(gps) = exif.gps {
        let fmt_gps_coord = |coords: &[rawler::formats::tiff::Rational; 3]| -> String {
            format!(
                "{} deg {} min {} sec",
                fmt_rat(&coords[0]),
                fmt_rat(&coords[1]),
                fmt_rat(&coords[2])
            )
        };

        if let Some(lat) = gps.gps_latitude {
            insert_if_present("GPSLatitude", fmt_gps_coord(&lat));
        }
        if let Some(lat_ref) = gps.gps_latitude_ref {
            insert_if_present("GPSLatitudeRef", lat_ref);
        }
        if let Some(lon) = gps.gps_longitude {
            insert_if_present("GPSLongitude", fmt_gps_coord(&lon));
        }
        if let Some(lon_ref) = gps.gps_longitude_ref {
            insert_if_present("GPSLongitudeRef", lon_ref);
        }
        if let Some(alt) = gps.gps_altitude {
            insert_if_present("GPSAltitude", fmt_rat(&alt).to_string());
        }
        if let Some(alt_ref) = gps.gps_altitude_ref {
            insert_if_present("GPSAltitudeRef", alt_ref.to_string());
        }
        if let Some(v) = gps.gps_img_direction {
            insert_if_present("GPSImgDirection", fmt_rat(&v).to_string());
        }
        if let Some(v) = gps.gps_img_direction_ref {
            insert_if_present("GPSImgDirectionRef", v);
        }
        if let Some(v) = gps.gps_speed {
            insert_if_present("GPSSpeed", fmt_rat(&v).to_string());
        }
        if let Some(v) = gps.gps_speed_ref {
            insert_if_present("GPSSpeedRef", v);
        }
        if let Some(v) = gps.gps_status {
            insert_if_present("GPSStatus", v);
        }
        if let Some(v) = gps.gps_measure_mode {
            insert_if_present("GPSMeasureMode", v);
        }
        if let Some(v) = gps.gps_dop {
            insert_if_present("GPSDOP", fmt_rat(&v).to_string());
        }
        if let Some(v) = gps.gps_map_datum {
            insert_if_present("GPSMapDatum", v);
        }
    }

    Some(map)
}

pub fn get_creation_date_from_path(path: &Path) -> DateTime<Utc> {
    if let Some(dt) = try_get_exif_creation_date(path) {
        return dt;
    }

    fs::metadata(path)
        .ok()
        .and_then(|m| m.created().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
}

pub fn try_get_exif_creation_date(path: &Path) -> Option<DateTime<Utc>> {
    if let Some(map) = read_rrexif_sidecar(path)
        && let Some(dt_str) = map.get("DateTimeOriginal").or(map.get("CreateDate"))
        && let Some(dt) = parse_creation_datetime(dt_str)
    {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }

    if let Ok(file) = std::fs::File::open(path) {
        let mut bufreader = BufReader::new(&file);
        let exifreader = exif::Reader::new();

        if let Ok(exif_obj) = exifreader.read_from_container(&mut bufreader) {
            for tag in [exif::Tag::DateTimeOriginal, exif::Tag::DateTime] {
                if let Some(field) = exif_obj.get_field(tag, exif::In::PRIMARY)
                    && let Some(dt) = parse_creation_field(field)
                {
                    return Some(dt);
                }
            }
        }
    }

    if is_raw_file(path) {
        let loader = rawler::RawLoader::new();
        if let Ok(raw_source) = rawler::rawsource::RawSource::new(path)
            && let Ok(decoder) = loader.get_decoder(&raw_source)
            && let Ok(metadata) = decoder.raw_metadata(&raw_source, &Default::default())
        {
            if let Some(dt) = parse_raw_creation_date(metadata.exif.date_time_original.as_deref()) {
                return Some(dt);
            }
            if let Some(dt) = parse_raw_creation_date(metadata.exif.create_date.as_deref()) {
                return Some(dt);
            }
        }
    }

    None
}

#[cfg(target_os = "android")]
pub fn get_creation_date_from_bytes(path_hint: &str, file_bytes: &[u8]) -> DateTime<Utc> {
    if let Some(exif_obj) = read_exif(file_bytes) {
        for tag in [exif::Tag::DateTimeOriginal, exif::Tag::DateTime] {
            if let Some(field) = exif_obj.get_field(tag, exif::In::PRIMARY)
                && let Some(dt) = parse_creation_field(field)
            {
                return dt;
            }
        }
    }

    if is_raw_file(path_hint)
        && let Some(metadata) = read_raw_metadata(file_bytes)
    {
        if let Some(dt) = parse_raw_creation_date(metadata.exif.date_time_original.as_deref()) {
            return dt;
        }
        if let Some(dt) = parse_raw_creation_date(metadata.exif.create_date.as_deref()) {
            return dt;
        }
    }

    Utc::now()
}

pub fn write_image_with_metadata(
    image_bytes: &mut Vec<u8>,
    original_path_str: &str,
    output_format: &str,
    keep_metadata: bool,
    strip_gps: bool,
) -> Result<(), String> {
    if !keep_metadata {
        return Ok(());
    }

    let original_path = Path::new(original_path_str);
    if !original_path.exists() {
        return Ok(());
    }

    // FIXME: EXIF still can't be written to TIFF — little_exif corrupts tags on
    // this path. Keywords are separate: they go in as an XMP IFD entry below,
    // which touches nothing little_exif would have written.
    if output_format.to_lowercase() == "tiff" {
        write_xmp_only(image_bytes, original_path, "tiff");
        return Ok(());
    }

    // Skip TIFF sources to avoid potential tag corruption issues
    let original_ext = original_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if original_ext == "tiff" || original_ext == "tif" {
        return Ok(());
    }

    let file_type = match output_format.to_lowercase().as_str() {
        "jpg" | "jpeg" => FileExtension::JPEG,
        "png" => FileExtension::PNG {
            as_zTXt_chunk: true,
        },
        "tiff" => FileExtension::TIFF,
        _ => return Ok(()),
    };

    let mut metadata = Metadata::new();
    let mut source_read_success = false;

    if let Some(map) = read_rrexif_sidecar(original_path) {
        source_read_success = true;

        let clean_s = |s: &String| s.replace('"', "").trim().to_string();

        let parse_ur64 = |s: &str| -> Option<uR64> {
            let cleaned_string = s
                .replace("f/", "")
                .replace(" s", "")
                .replace(" mm", "")
                .replace("\"", "");

            let val = cleaned_string.trim();

            if val.contains('/') {
                let parts: Vec<&str> = val.split('/').collect();
                if parts.len() == 2
                    && let (Ok(n), Ok(d)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                {
                    return Some(uR64 {
                        nominator: n,
                        denominator: d,
                    });
                }
            } else if let Ok(f) = val.parse::<f32>() {
                return Some(uR64 {
                    nominator: (f * 1000.0) as u32,
                    denominator: 1000,
                });
            }
            None
        };
        if let Some(val) = map.get("Make") {
            metadata.set_tag(ExifTag::Make(clean_s(val)));
        }
        if let Some(val) = map.get("Model") {
            metadata.set_tag(ExifTag::Model(clean_s(val)));
        }
        if let Some(val) = map.get("LensMake") {
            metadata.set_tag(ExifTag::LensMake(clean_s(val)));
        }
        if let Some(val) = map.get("LensModel") {
            metadata.set_tag(ExifTag::LensModel(clean_s(val)));
        }
        if let Some(val) = map.get("Artist") {
            metadata.set_tag(ExifTag::Artist(clean_s(val)));
        }
        if let Some(val) = map.get("Copyright") {
            metadata.set_tag(ExifTag::Copyright(clean_s(val)));
        }
        if let Some(val) = map.get("UserComment") {
            metadata.set_tag(ExifTag::UserComment(clean_s(val).into_bytes()));
        }
        if let Some(val) = map.get("ImageDescription") {
            metadata.set_tag(ExifTag::ImageDescription(clean_s(val)));
        }
        if let Some(val) = map.get("DateTimeOriginal") {
            metadata.set_tag(ExifTag::DateTimeOriginal(clean_s(val)));
        }
        if let Some(val) = map.get("CreateDate") {
            metadata.set_tag(ExifTag::CreateDate(clean_s(val)));
        }
        if let Some(val) = map.get("FNumber")
            && let Some(ur) = parse_ur64(val)
        {
            metadata.set_tag(ExifTag::FNumber(vec![ur]));
        }
        if let Some(val) = map.get("ExposureTime")
            && let Some(ur) = parse_ur64(val)
        {
            metadata.set_tag(ExifTag::ExposureTime(vec![ur]));
        }
        if let Some(val) = map.get("FocalLength")
            && let Some(ur) = parse_ur64(val)
        {
            metadata.set_tag(ExifTag::FocalLength(vec![ur]));
        }
        if let Some(val) = map.get("FocalLengthIn35mmFilm") {
            let cleaned = val.replace(" mm", "").replace("\"", "");
            let trimmed = cleaned.trim();
            if let Ok(f_val) = trimmed.parse::<f32>() {
                metadata.set_tag(ExifTag::FocalLengthIn35mmFormat(vec![f_val.round() as u16]));
            }
        }
        if let Some(val) = map.get("ISOSpeed").or(map.get("PhotographicSensitivity"))
            && let Ok(iso) = val.replace('"', "").trim().parse::<u16>()
        {
            metadata.set_tag(ExifTag::ISO(vec![iso]));
        }
    }

    if !source_read_success && let Ok(file) = std::fs::File::open(original_path) {
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();

        if let Ok(exif_obj) = exifreader.read_from_container(&mut bufreader) {
            source_read_success = true;

            let get_string_val = |field: &exif::Field| -> String {
                match &field.value {
                    exif::Value::Ascii(vec) => vec
                        .iter()
                        .map(|v| {
                            String::from_utf8_lossy(v)
                                .trim_matches(char::from(0))
                                .to_string()
                        })
                        .collect::<Vec<String>>()
                        .join(" "),
                    _ => field
                        .display_value()
                        .to_string()
                        .replace("\"", "")
                        .trim()
                        .to_string(),
                }
            };

            if let Some(f) = exif_obj.get_field(exif::Tag::Make, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Make(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::Model, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Model(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::LensMake, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::LensMake(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::LensModel(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::Artist, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Artist(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::Copyright, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Copyright(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::DateTimeOriginal(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::DateTime, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::CreateDate(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::FNumber, exif::In::PRIMARY)
                && let exif::Value::Rational(v) = &f.value
                && !v.is_empty()
            {
                metadata.set_tag(ExifTag::FNumber(vec![to_ur64(&v[0])]));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY)
                && let exif::Value::Rational(v) = &f.value
                && !v.is_empty()
            {
                metadata.set_tag(ExifTag::ExposureTime(vec![to_ur64(&v[0])]));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::FocalLength, exif::In::PRIMARY)
                && let exif::Value::Rational(v) = &f.value
                && !v.is_empty()
            {
                metadata.set_tag(ExifTag::FocalLength(vec![to_ur64(&v[0])]));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::ExposureBiasValue, exif::In::PRIMARY) {
                match &f.value {
                    exif::Value::SRational(v) if !v.is_empty() => {
                        metadata.set_tag(ExifTag::ExposureCompensation(vec![to_ir64(&v[0])]));
                    }
                    exif::Value::Rational(v) if !v.is_empty() => {
                        metadata.set_tag(ExifTag::ExposureCompensation(vec![iR64 {
                            nominator: v[0].num as i32,
                            denominator: v[0].denom as i32,
                        }]));
                    }
                    _ => {}
                }
            }
            if let Some(f) =
                exif_obj.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
            {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::ISO(vec![val as u16]));
                }
            } else if let Some(f) = exif_obj.get_field(exif::Tag::ISOSpeed, exif::In::PRIMARY)
                && let Some(val) = f.value.get_uint(0)
            {
                metadata.set_tag(ExifTag::ISO(vec![val as u16]));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::FocalLengthIn35mmFilm, exif::In::PRIMARY)
                && let Some(val) = f.value.get_uint(0)
            {
                metadata.set_tag(ExifTag::FocalLengthIn35mmFormat(vec![val as u16]));
            }
            if !strip_gps {
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
                    && let exif::Value::Rational(v) = &f.value
                    && v.len() >= 3
                {
                    metadata.set_tag(ExifTag::GPSLatitude(vec![
                        to_ur64(&v[0]),
                        to_ur64(&v[1]),
                        to_ur64(&v[2]),
                    ]));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY) {
                    metadata.set_tag(ExifTag::GPSLatitudeRef(get_string_val(f)));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
                    && let exif::Value::Rational(v) = &f.value
                    && v.len() >= 3
                {
                    metadata.set_tag(ExifTag::GPSLongitude(vec![
                        to_ur64(&v[0]),
                        to_ur64(&v[1]),
                        to_ur64(&v[2]),
                    ]));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY) {
                    metadata.set_tag(ExifTag::GPSLongitudeRef(get_string_val(f)));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY)
                    && let exif::Value::Rational(v) = &f.value
                    && !v.is_empty()
                {
                    metadata.set_tag(ExifTag::GPSAltitude(vec![to_ur64(&v[0])]));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSAltitudeRef, exif::In::PRIMARY) {
                    let alt_ref = f.value.get_uint(0).unwrap_or(0) as u8;
                    metadata.set_tag(ExifTag::GPSAltitudeRef(vec![alt_ref]));
                }
            }
        }
    }

    if !source_read_success && is_raw_file(original_path_str) {
        let loader = rawler::RawLoader::new();
        if let Ok(raw_source) = rawler::rawsource::RawSource::new(Path::new(original_path_str))
            && let Ok(decoder) = loader.get_decoder(&raw_source)
            && let Ok(meta) = decoder.raw_metadata(&raw_source, &Default::default())
        {
            if !meta.make.is_empty() {
                metadata.set_tag(ExifTag::Make(meta.make.clone()));
            }
            if !meta.model.is_empty() {
                metadata.set_tag(ExifTag::Model(meta.model.clone()));
            }
            let exif = meta.exif;
            if let Some(artist) = exif.artist {
                metadata.set_tag(ExifTag::Artist(artist));
            }
            if let Some(copyright) = exif.copyright {
                metadata.set_tag(ExifTag::Copyright(copyright));
            }
            if let Some(dt) = exif.date_time_original {
                metadata.set_tag(ExifTag::DateTimeOriginal(dt));
            }
            if let Some(dt) = exif.create_date {
                metadata.set_tag(ExifTag::CreateDate(dt));
            }
            if let Some(lens_make) = exif.lens_make {
                metadata.set_tag(ExifTag::LensMake(lens_make));
            }
            if let Some(lens_model) = exif.lens_model {
                metadata.set_tag(ExifTag::LensModel(lens_model));
            }
            if let Some(f) = exif.fnumber {
                metadata.set_tag(ExifTag::FNumber(vec![uR64 {
                    nominator: f.n,
                    denominator: f.d,
                }]));
            }
            if let Some(t) = exif.exposure_time {
                metadata.set_tag(ExifTag::ExposureTime(vec![uR64 {
                    nominator: t.n,
                    denominator: t.d,
                }]));
            }
            if let Some(fl) = exif.focal_length {
                metadata.set_tag(ExifTag::FocalLength(vec![uR64 {
                    nominator: fl.n,
                    denominator: fl.d,
                }]));
            }
            if let Some(iso) = exif.iso_speed {
                metadata.set_tag(ExifTag::ISO(vec![iso as u16]));
            } else if let Some(iso) = exif.iso_speed_ratings {
                metadata.set_tag(ExifTag::ISO(vec![iso]));
            }
            if let Some(ev) = exif.exposure_bias {
                metadata.set_tag(ExifTag::ExposureCompensation(vec![iR64 {
                    nominator: ev.n,
                    denominator: ev.d,
                }]));
            }
            if let Some(flash) = exif.flash {
                metadata.set_tag(ExifTag::Flash(vec![flash]));
            }
            if let Some(metering) = exif.metering_mode {
                metadata.set_tag(ExifTag::MeteringMode(vec![metering]));
            }
            if let Some(wb) = exif.white_balance {
                metadata.set_tag(ExifTag::WhiteBalance(vec![wb]));
            }
            if let Some(prog) = exif.exposure_program {
                metadata.set_tag(ExifTag::ExposureProgram(vec![prog]));
            }
            if !strip_gps && let Some(gps) = exif.gps {
                if let Some(lat) = gps.gps_latitude {
                    metadata.set_tag(ExifTag::GPSLatitude(vec![
                        uR64 {
                            nominator: lat[0].n,
                            denominator: lat[0].d,
                        },
                        uR64 {
                            nominator: lat[1].n,
                            denominator: lat[1].d,
                        },
                        uR64 {
                            nominator: lat[2].n,
                            denominator: lat[2].d,
                        },
                    ]));
                }
                if let Some(lat_ref) = gps.gps_latitude_ref {
                    metadata.set_tag(ExifTag::GPSLatitudeRef(lat_ref));
                }
                if let Some(lon) = gps.gps_longitude {
                    metadata.set_tag(ExifTag::GPSLongitude(vec![
                        uR64 {
                            nominator: lon[0].n,
                            denominator: lon[0].d,
                        },
                        uR64 {
                            nominator: lon[1].n,
                            denominator: lon[1].d,
                        },
                        uR64 {
                            nominator: lon[2].n,
                            denominator: lon[2].d,
                        },
                    ]));
                }
                if let Some(lon_ref) = gps.gps_longitude_ref {
                    metadata.set_tag(ExifTag::GPSLongitudeRef(lon_ref));
                }
                if let Some(alt) = gps.gps_altitude {
                    metadata.set_tag(ExifTag::GPSAltitude(vec![uR64 {
                        nominator: alt.n,
                        denominator: alt.d,
                    }]));
                }
                if let Some(alt_ref) = gps.gps_altitude_ref {
                    metadata.set_tag(ExifTag::GPSAltitudeRef(vec![alt_ref]));
                }
            }
        }
    }

    metadata.set_tag(ExifTag::Software("RapidRAW".to_string()));
    metadata.set_tag(ExifTag::Orientation(vec![1u16]));
    metadata.set_tag(ExifTag::ColorSpace(vec![1u16]));

    if let Err(e) = metadata.write_to_vec(image_bytes, file_type) {
        log::warn!("Failed to write metadata: {}", e);
    }

    // Keyword tags live in the sidecar, not in the EXIF map read above, and have
    // no EXIF tag to carry them — emit them as an XMP dc:subject packet. Formats
    // whose containers we can't splice get a .xmp sidecar instead, written by the
    // caller (which is the side that knows the output path).
    if let Some(xmp) = xmp_packet_for_source(original_path) {
        let format = output_format.to_lowercase();
        let result = match format.as_str() {
            "jpg" | "jpeg" => insert_xmp_into_jpeg(image_bytes, &xmp),
            "png" => insert_xmp_into_png(image_bytes, &xmp),
            _ => Ok(()),
        };
        // A failed splice must not lose the export — the pixels are fine, only
        // the keywords are missing.
        if let Err(e) = result {
            log::warn!("Failed to write XMP keywords: {}", e);
        }
    }

    Ok(())
}

pub fn get_primary_sidecar_path(image_path: &Path) -> PathBuf {
    let mut filename = image_path.file_name().unwrap_or_default().to_os_string();
    filename.push(".rrdata");
    image_path.with_file_name(filename)
}

pub fn get_rrexif_path(image_path: &Path) -> PathBuf {
    let mut filename = image_path.file_name().unwrap_or_default().to_os_string();
    filename.push(".rrexif");
    image_path.with_file_name(filename)
}

fn load_primary_metadata(image_path: &Path) -> ImageMetadata {
    let primary = get_primary_sidecar_path(image_path);
    load_sidecar(&primary)
}

fn save_primary_metadata(image_path: &Path, metadata: &ImageMetadata) -> std::io::Result<()> {
    let primary = get_primary_sidecar_path(image_path);
    let json = serde_json::to_string_pretty(metadata).map_err(std::io::Error::other)?;
    fs::write(&primary, json)
}

pub fn read_rrexif_sidecar(image_path: &Path) -> Option<HashMap<String, String>> {
    let metadata = load_primary_metadata(image_path);
    if let Some(mut exif) = metadata.exif {
        if heal_cached_user_comment(&mut exif) {
            let mut healed = load_primary_metadata(image_path);
            healed.exif = Some(exif.clone());
            let _ = save_primary_metadata(image_path, &healed);
        }
        return Some(exif);
    }

    let legacy = get_rrexif_path(image_path);
    if legacy.exists()
        && let Ok(content) = fs::read_to_string(&legacy)
        && let Ok(mut map) = serde_json::from_str::<HashMap<String, String>>(&content)
    {
        heal_cached_user_comment(&mut map);
        let mut migrated = load_primary_metadata(image_path);
        migrated.exif = Some(map.clone());
        if save_primary_metadata(image_path, &migrated).is_ok() {
            let _ = fs::remove_file(&legacy);
        }
        return Some(map);
    }

    None
}

pub fn read_exif_data_from_bytes(path: &str, file_bytes: &[u8]) -> HashMap<String, String> {
    if is_raw_file(path)
        && let Some(map) = extract_metadata(file_bytes)
    {
        return map;
    }

    let mut exif_data = HashMap::new();
    if let Some(exif) = read_exif(file_bytes) {
        for field in exif.fields() {
            let raw_val = match &field.value {
                exif::Value::Ascii(_) => match clean_ascii_value(&field.value) {
                    Some(v) => v,
                    None => continue,
                },
                exif::Value::Rational(v) if field.tag == exif::Tag::LensSpecification => {
                    match format_lens_specification(v) {
                        Some(s) => s,
                        None => continue,
                    }
                }
                _ => field.display_value().with_unit(&exif).to_string(),
            };
            exif_data.insert(field.tag.to_string(), truncate_large_exif(&raw_val));
        }
    }
    exif_data
}

pub fn read_exif_data(path: &str, file_bytes: &[u8]) -> HashMap<String, String> {
    let source_path = Path::new(path);
    if let Some(sidecar_exif) = read_rrexif_sidecar(source_path) {
        return sidecar_exif;
    }

    let exif_map = read_exif_data_from_bytes(path, file_bytes);
    if !exif_map.is_empty() {
        let mut metadata = load_primary_metadata(source_path);
        metadata.exif = Some(exif_map.clone());
        let _ = save_primary_metadata(source_path, &metadata);
    }
    exif_map
}

pub fn persist_exif_if_missing(source_path: &Path, source_path_str: &str, file_bytes: &[u8]) {
    {
        let metadata = load_primary_metadata(source_path);
        if metadata.exif.is_some() {
            return;
        }
    }

    let legacy = get_rrexif_path(source_path);
    if legacy.exists()
        && let Ok(content) = fs::read_to_string(&legacy)
        && let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content)
    {
        let mut metadata = load_primary_metadata(source_path);
        metadata.exif = Some(map);
        if save_primary_metadata(source_path, &metadata).is_ok() {
            let _ = fs::remove_file(&legacy);
        }
        return;
    }

    let exif_map = read_exif_data_from_bytes(source_path_str, file_bytes);
    if exif_map.is_empty() {
        return;
    }

    let mut metadata = load_primary_metadata(source_path);

    if metadata.exif.is_none() {
        metadata.exif = Some(exif_map);
        let _ = save_primary_metadata(source_path, &metadata);
    }
}

pub fn write_rrexif_sidecar(source_path_str: &str, target_image_path: &Path) -> Result<(), String> {
    let source_path = Path::new(source_path_str);

    let exif_data = if let Some(existing) = read_rrexif_sidecar(source_path) {
        existing
    } else if let Ok(bytes) = fs::read(source_path) {
        read_exif_data_from_bytes(source_path_str, &bytes)
    } else {
        return Ok(());
    };

    if exif_data.is_empty() {
        return Ok(());
    }

    let mut metadata = load_primary_metadata(target_image_path);
    metadata.exif = Some(exif_data);
    save_primary_metadata(target_image_path, &metadata)
        .map_err(|e| format!("Failed to write sidecar: {}", e))
}

/// Keywords have no home in EXIF as little_exif exposes it — there is no
/// XPKeywords variant and no raw-tag escape hatch — so they go out as an XMP
/// `dc:subject` bag instead. That is the format every DAM reads (Lightroom,
/// Bridge, digiKam), and it is the one this app already parses on the way in,
/// via `file_management::extract_xmp_tags`, so a round-trip preserves them.
fn build_xmp_packet(tags: &[String]) -> String {
    // XML-escape: a tag is free text and may legitimately contain & or <.
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let items = tags
        .iter()
        .map(|t| format!("<rdf:li>{}</rdf:li>", esc(t)))
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
<dc:subject><rdf:Bag>{items}</rdf:Bag></dc:subject>\
</rdf:Description></rdf:RDF></x:xmpmeta>\
<?xpacket end=\"w\"?>"
    )
}

/// Formats whose containers we can splice an XMP packet into.
///
/// The rest (TIFF, WebP, AVIF, JXL) each need real container surgery — a new
/// IFD entry, a VP8X upgrade, an ISOBMFF meta item — so they get a `.xmp`
/// sidecar written next to the export instead. Every DAM reads those, and an
/// unparsed sidecar cannot corrupt the image the way a bad chunk can.
pub fn supports_embedded_xmp(output_format: &str) -> bool {
    matches!(output_format.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "tiff" | "tif")
}

/// Did an XMP packet actually make it into these bytes?
///
/// The caller uses this to decide whether a `.xmp` sidecar is still needed: a
/// splice that failed (odd container variant, size limit) logs a warning but
/// otherwise looks identical to success, and silently dropping keywords is
/// worse than an extra file.
pub fn has_embedded_xmp(image_bytes: &[u8]) -> bool {
    image_bytes
        .windows(11)
        .any(|w| w == b"dc:subject>")
}

/// Build the XMP packet for an image's sidecar tags, if it has any.
pub fn xmp_packet_for_source(original_path: &Path) -> Option<String> {
    let tags = load_primary_metadata(original_path)
        .tags
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>();
    if tags.is_empty() {
        None
    } else {
        Some(build_xmp_packet(&tags))
    }
}

/// Splice an XMP packet into an encoded PNG as an uncompressed `iTXt` chunk.
///
/// The keyword `XML:com.adobe.xmp` is what the XMP spec mandates for PNG, and
/// what readers look for. The chunk goes immediately before `IEND`, which is
/// legal for ancillary chunks and avoids disturbing `IHDR`/`PLTE` ordering.
fn insert_xmp_into_png(png: &mut Vec<u8>, xmp: &str) -> Result<(), String> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if png.len() < 8 || png[0..8] != SIGNATURE {
        return Err("not a PNG (bad signature)".to_string());
    }

    // Walk the chunk list to locate IEND rather than searching for the bytes —
    // "IEND" could otherwise appear inside compressed image data.
    let mut pos = 8usize;
    let mut iend_start = None;
    while pos + 8 <= png.len() {
        let len = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
        let ctype = &png[pos + 4..pos + 8];
        if ctype == b"IEND" {
            iend_start = Some(pos);
            break;
        }
        // length + type + data + crc
        pos = pos
            .checked_add(12)
            .and_then(|p| p.checked_add(len))
            .ok_or("malformed PNG chunk length")?;
    }
    let iend_start = iend_start.ok_or("no IEND chunk found")?;

    // iTXt payload: keyword\0 compression_flag compression_method
    //               language\0 translated_keyword\0 text
    let mut data = Vec::new();
    data.extend_from_slice(b"XML:com.adobe.xmp");
    data.push(0); // keyword terminator
    data.push(0); // compression flag: uncompressed
    data.push(0); // compression method (ignored when uncompressed)
    data.push(0); // empty language tag
    data.push(0); // empty translated keyword
    data.extend_from_slice(xmp.as_bytes());

    let len: u32 = data
        .len()
        .try_into()
        .map_err(|_| "XMP packet too large for a PNG chunk".to_string())?;

    let mut hasher = crc32fast::Hasher::new();
    hasher.update(b"iTXt");
    hasher.update(&data);
    let crc = hasher.finalize();

    let mut chunk = Vec::with_capacity(data.len() + 12);
    chunk.extend_from_slice(&len.to_be_bytes());
    chunk.extend_from_slice(b"iTXt");
    chunk.extend_from_slice(&data);
    chunk.extend_from_slice(&crc.to_be_bytes());

    png.splice(iend_start..iend_start, chunk);
    Ok(())
}

/// Embed an XMP packet into an encoded TIFF as IFD entry 700.
///
/// Rather than widening IFD0 in place — which would push every value offset
/// after it and require rewriting the whole file — this appends the packet and a
/// fresh copy of the IFD at the end, then repoints the header at the new IFD.
/// Existing entries keep their original offsets verbatim, so nothing that worked
/// before can break. The old IFD is left orphaned, which readers never visit.
fn insert_xmp_into_tiff(tiff: &mut Vec<u8>, xmp: &str) -> Result<(), String> {
    const TAG_XMP: u16 = 700;
    const TYPE_BYTE: u16 = 1;

    if tiff.len() < 8 {
        return Err("not a TIFF (too short)".to_string());
    }
    let le = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err("not a TIFF (bad byte-order mark)".to_string()),
    };
    let u16_at = |b: &[u8], o: usize| -> u16 {
        if le { u16::from_le_bytes([b[o], b[o + 1]]) } else { u16::from_be_bytes([b[o], b[o + 1]]) }
    };
    let u32_at = |b: &[u8], o: usize| -> u32 {
        if le {
            u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        } else {
            u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        }
    };
    let put_u16 = |v: u16| -> [u8; 2] { if le { v.to_le_bytes() } else { v.to_be_bytes() } };
    let put_u32 = |v: u32| -> [u8; 4] { if le { v.to_le_bytes() } else { v.to_be_bytes() } };

    // 42 is classic TIFF; 43 is BigTIFF, whose 8-byte offsets this doesn't handle.
    if u16_at(tiff, 2) != 42 {
        return Err("unsupported TIFF variant (BigTIFF?)".to_string());
    }

    let ifd_offset = u32_at(tiff, 4) as usize;
    if ifd_offset + 2 > tiff.len() {
        return Err("TIFF IFD offset out of range".to_string());
    }
    let count = u16_at(tiff, ifd_offset) as usize;
    let entries_start = ifd_offset + 2;
    let entries_end = entries_start + count * 12;
    if entries_end + 4 > tiff.len() {
        return Err("TIFF IFD truncated".to_string());
    }

    let entries: Vec<&[u8]> = (0..count)
        .map(|i| &tiff[entries_start + i * 12..entries_start + (i + 1) * 12])
        .collect();

    // Already tagged (re-export of an XMP-bearing file): leave it alone rather
    // than write a second, conflicting entry.
    if entries.iter().any(|e| u16_at(e, 0) == TAG_XMP) {
        return Ok(());
    }

    let next_ifd = u32_at(tiff, entries_end);
    let entries: Vec<Vec<u8>> = entries.iter().map(|e| e.to_vec()).collect();

    // Values must sit at an even offset per the spec.
    if tiff.len() % 2 != 0 {
        tiff.push(0);
    }
    let xmp_offset = tiff.len();
    let xmp_len: u32 = xmp
        .len()
        .try_into()
        .map_err(|_| "XMP packet too large for a TIFF entry".to_string())?;
    tiff.extend_from_slice(xmp.as_bytes());

    if tiff.len() % 2 != 0 {
        tiff.push(0);
    }
    let new_ifd_offset = tiff.len();

    let mut xmp_entry = Vec::with_capacity(12);
    xmp_entry.extend_from_slice(&put_u16(TAG_XMP));
    xmp_entry.extend_from_slice(&put_u16(TYPE_BYTE));
    xmp_entry.extend_from_slice(&put_u32(xmp_len));
    xmp_entry.extend_from_slice(&put_u32(xmp_offset as u32));

    // The spec requires entries in ascending tag order.
    let mut all: Vec<Vec<u8>> = entries;
    let insert_at = all
        .iter()
        .position(|e| u16_at(e, 0) > TAG_XMP)
        .unwrap_or(all.len());
    all.insert(insert_at, xmp_entry);

    tiff.extend_from_slice(&put_u16(all.len() as u16));
    for e in &all {
        tiff.extend_from_slice(e);
    }
    tiff.extend_from_slice(&put_u32(next_ifd));

    // Repoint the header at the IFD we just wrote.
    let ptr = put_u32(new_ifd_offset as u32);
    tiff[4..8].copy_from_slice(&ptr);
    Ok(())
}

/// Write only the keyword packet for formats where the EXIF path is skipped.
fn write_xmp_only(image_bytes: &mut Vec<u8>, original_path: &Path, format: &str) {
    let Some(xmp) = xmp_packet_for_source(original_path) else {
        return;
    };
    let result = match format {
        "tiff" => insert_xmp_into_tiff(image_bytes, &xmp),
        _ => Ok(()),
    };
    if let Err(e) = result {
        log::warn!("Failed to write XMP keywords: {}", e);
    }
}

/// Splice an XMP APP1 segment into an already-encoded JPEG.
///
/// Runs *after* little_exif has written its own segments, so it walks past the
/// existing APPn block and inserts at the end of it rather than at SOI — putting
/// it ahead of the EXIF APP1 makes some readers stop before finding the EXIF.
fn insert_xmp_into_jpeg(jpeg: &mut Vec<u8>, xmp: &str) -> Result<(), String> {
    const XMP_NS: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

    if jpeg.len() < 2 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err("not a JPEG (missing SOI)".to_string());
    }

    // Walk the APPn/marker segments to find where the header block ends.
    let mut pos = 2usize;
    loop {
        if pos + 4 > jpeg.len() || jpeg[pos] != 0xFF {
            break;
        }
        let marker = jpeg[pos + 1];
        // SOS/SOF and friends mean the header block is over.
        if !(0xE0..=0xEF).contains(&marker) && marker != 0xFE {
            break;
        }
        let len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        if len < 2 || pos + 2 + len > jpeg.len() {
            break;
        }
        pos += 2 + len;
    }

    let payload_len = XMP_NS.len() + xmp.len();
    // A segment's length field is 16-bit and counts itself, so the packet has a
    // hard ceiling. Extended XMP exists for larger payloads; keywords never come
    // close, so bail rather than emit a corrupt file.
    if payload_len + 2 > u16::MAX as usize {
        return Err("XMP packet too large for a single APP1 segment".to_string());
    }

    let mut segment = Vec::with_capacity(payload_len + 4);
    segment.extend_from_slice(&[0xFF, 0xE1]);
    segment.extend_from_slice(&((payload_len + 2) as u16).to_be_bytes());
    segment.extend_from_slice(XMP_NS);
    segment.extend_from_slice(xmp.as_bytes());

    jpeg.splice(pos..pos, segment);
    Ok(())
}

#[cfg(test)]
mod xmp_tests {
    use super::*;

    /// Minimal but structurally real JPEG: SOI, an APP0/JFIF segment, then EOI.
    fn tiny_jpeg() -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8];
        let jfif: &[u8] = b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0";
        v.extend_from_slice(&[0xFF, 0xE0]);
        v.extend_from_slice(&((jfif.len() + 2) as u16).to_be_bytes());
        v.extend_from_slice(jfif);
        v.extend_from_slice(&[0xFF, 0xD9]);
        v
    }

    #[test]
    fn inserts_after_existing_app_segments_and_keeps_the_file_intact() {
        let mut jpeg = tiny_jpeg();
        let original_len = jpeg.len();
        insert_xmp_into_jpeg(&mut jpeg, &build_xmp_packet(&["forest".to_string()])).unwrap();

        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8], "SOI must stay first");
        assert_eq!(&jpeg[jpeg.len() - 2..], &[0xFF, 0xD9], "EOI must stay last");
        assert!(jpeg.len() > original_len);

        // The APP0 must still precede our APP1.
        let app0 = jpeg.windows(2).position(|w| w == [0xFF, 0xE0]).unwrap();
        let app1 = jpeg.windows(2).position(|w| w == [0xFF, 0xE1]).unwrap();
        assert!(app0 < app1, "XMP segment must come after the existing APP0");
    }

    #[test]
    fn declared_segment_length_matches_the_bytes_written() {
        let mut jpeg = tiny_jpeg();
        let xmp = build_xmp_packet(&["a".to_string(), "b".to_string()]);
        insert_xmp_into_jpeg(&mut jpeg, &xmp).unwrap();

        let app1 = jpeg.windows(2).position(|w| w == [0xFF, 0xE1]).unwrap();
        let declared = u16::from_be_bytes([jpeg[app1 + 2], jpeg[app1 + 3]]) as usize;
        let expected = b"http://ns.adobe.com/xap/1.0/\0".len() + xmp.len() + 2;
        assert_eq!(declared, expected, "length field must count itself + payload");
    }

    #[test]
    fn round_trips_through_the_apps_own_tag_parser() {
        let tags = vec!["landscape".to_string(), "black & white".to_string()];
        let xmp = build_xmp_packet(&tags);
        // The importer is the real consumer — what we write it must read back.
        let parsed = crate::file_management::extract_xmp_tags(&xmp);
        assert_eq!(parsed, tags, "ampersand must survive escape + parse");
    }

    #[test]
    fn rejects_non_jpeg_input_instead_of_corrupting_it() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(insert_xmp_into_jpeg(&mut png, "<x/>").is_err());
        assert_eq!(png.len(), 8, "input must be left untouched on error");
    }

    /// End-to-end: a real `.rrdata` sidecar on disk -> exported JPEG -> tags
    /// parsed back out. Covers the wiring (`load_primary_metadata().tags`) that
    /// the unit tests above stub past.
    #[test]
    fn sidecar_tags_reach_the_exported_jpeg() {
        use image::{DynamicImage, RgbImage};

        let dir = std::env::temp_dir().join(format!("rr-xmp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("frame.jpg");

        // A real source file must exist: the writer bails early otherwise.
        let img = DynamicImage::ImageRgb8(RgbImage::new(8, 8));
        img.save(&source).unwrap();

        // Sidecar in the app's own on-disk shape (`<file>.rrdata`).
        let meta = ImageMetadata {
            tags: Some(vec!["pizza".to_string(), "black & white".to_string()]),
            ..Default::default()
        };
        std::fs::write(
            get_primary_sidecar_path(&source),
            serde_json::to_string(&meta).unwrap(),
        )
        .unwrap();

        let mut bytes = std::fs::read(&source).unwrap();
        write_image_with_metadata(&mut bytes, source.to_str().unwrap(), "jpg", true, false).unwrap();

        let text = String::from_utf8_lossy(&bytes);
        let parsed = crate::file_management::extract_xmp_tags(&text);
        assert_eq!(parsed, vec!["pizza".to_string(), "black & white".to_string()]);

        // Still a valid JPEG after the splice.
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
        assert!(image::load_from_memory(&bytes).is_ok(), "must still decode");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keep_metadata_off_writes_no_keywords() {
        let dir = std::env::temp_dir().join(format!("rr-xmp-off-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("frame.jpg");
        image::DynamicImage::ImageRgb8(image::RgbImage::new(8, 8)).save(&source).unwrap();
        let meta = ImageMetadata { tags: Some(vec!["secret".to_string()]), ..Default::default() };
        std::fs::write(get_primary_sidecar_path(&source), serde_json::to_string(&meta).unwrap()).unwrap();

        let mut bytes = std::fs::read(&source).unwrap();
        write_image_with_metadata(&mut bytes, source.to_str().unwrap(), "jpg", false, false).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn png_keywords_land_in_an_itxt_chunk_before_iend() {
        use image::{DynamicImage, RgbImage};
        let dir = std::env::temp_dir().join(format!("rr-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("frame.png");
        DynamicImage::ImageRgb8(RgbImage::new(8, 8)).save(&source).unwrap();

        let meta = ImageMetadata {
            tags: Some(vec!["pizza".to_string(), "black & white".to_string()]),
            ..Default::default()
        };
        std::fs::write(
            get_primary_sidecar_path(&source),
            serde_json::to_string(&meta).unwrap(),
        )
        .unwrap();

        let mut bytes = std::fs::read(&source).unwrap();
        write_image_with_metadata(&mut bytes, source.to_str().unwrap(), "png", true, false).unwrap();

        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("XML:com.adobe.xmp"), "must use the spec keyword");
        assert_eq!(
            crate::file_management::extract_xmp_tags(&text),
            vec!["pizza".to_string(), "black & white".to_string()]
        );

        // The chunk must precede IEND, and the file must still decode.
        let itxt = bytes.windows(4).position(|w| w == b"iTXt").unwrap();
        let iend = bytes.windows(4).rposition(|w| w == b"IEND").unwrap();
        assert!(itxt < iend, "iTXt must come before IEND");
        assert!(image::load_from_memory(&bytes).is_ok(), "PNG must still decode");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn png_chunk_crc_is_valid() {
        let mut png = {
            use image::{DynamicImage, RgbImage};
            let mut buf = std::io::Cursor::new(Vec::new());
            DynamicImage::ImageRgb8(RgbImage::new(4, 4))
                .write_to(&mut buf, image::ImageFormat::Png)
                .unwrap();
            buf.into_inner()
        };
        insert_xmp_into_png(&mut png, &build_xmp_packet(&["x".to_string()])).unwrap();

        let at = png.windows(4).position(|w| w == b"iTXt").unwrap();
        let len = u32::from_be_bytes([png[at - 4], png[at - 3], png[at - 2], png[at - 1]]) as usize;
        let mut h = crc32fast::Hasher::new();
        h.update(&png[at..at + 4 + len]);
        let stored = u32::from_be_bytes([
            png[at + 4 + len],
            png[at + 5 + len],
            png[at + 6 + len],
            png[at + 7 + len],
        ]);
        assert_eq!(h.finalize(), stored, "CRC must cover type + data");
    }

    #[test]
    fn embeddable_formats_are_reported_correctly() {
        for f in ["jpg", "jpeg", "png", "tiff", "tif", "JPG", "PNG", "TIFF"] {
            assert!(supports_embedded_xmp(f), "{f} should embed");
        }
        // Container surgery not implemented for these — they fall back to a
        // .xmp sidecar written next to the export.
        for f in ["webp", "avif", "jxl"] {
            assert!(!supports_embedded_xmp(f), "{f} should use a sidecar");
        }
    }

    #[test]
    fn a_format_we_cannot_embed_leaves_the_bytes_untouched() {
        use image::{DynamicImage, RgbImage};
        let dir = std::env::temp_dir().join(format!("rr-webp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("frame.png");
        DynamicImage::ImageRgb8(RgbImage::new(8, 8)).save(&source).unwrap();
        let meta = ImageMetadata { tags: Some(vec!["t".to_string()]), ..Default::default() };
        std::fs::write(get_primary_sidecar_path(&source), serde_json::to_string(&meta).unwrap()).unwrap();

        let mut bytes = std::fs::read(&source).unwrap();
        let before = bytes.clone();
        write_image_with_metadata(&mut bytes, source.to_str().unwrap(), "webp", true, false).unwrap();
        assert_eq!(bytes, before, "webp must be left for the sidecar path");

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn encode_tiff(w: u32, h: u32) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(w, h))
            .write_to(&mut buf, image::ImageFormat::Tiff)
            .unwrap();
        buf.into_inner()
    }

    #[test]
    fn tiff_keywords_embed_and_the_image_still_decodes() {
        let mut tiff = encode_tiff(8, 8);
        let tags = vec!["pizza".to_string(), "black & white".to_string()];
        insert_xmp_into_tiff(&mut tiff, &build_xmp_packet(&tags)).unwrap();

        assert_eq!(
            crate::file_management::extract_xmp_tags(&String::from_utf8_lossy(&tiff)),
            tags
        );
        let decoded = image::load_from_memory(&tiff).expect("TIFF must still decode");
        assert_eq!(decoded.width(), 8);
        assert!(has_embedded_xmp(&tiff));
    }

    #[test]
    fn tiff_ifd_stays_sorted_and_gains_exactly_one_entry() {
        let before = encode_tiff(4, 4);
        let le = &before[0..2] == b"II";
        let rd16 = |b: &[u8], o: usize| if le {
            u16::from_le_bytes([b[o], b[o + 1]])
        } else {
            u16::from_be_bytes([b[o], b[o + 1]])
        };
        let rd32 = |b: &[u8], o: usize| if le {
            u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        } else {
            u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        };

        let n_before = rd16(&before, rd32(&before, 4) as usize);

        let mut after = before.clone();
        insert_xmp_into_tiff(&mut after, &build_xmp_packet(&["x".to_string()])).unwrap();

        let ifd = rd32(&after, 4) as usize;
        let n_after = rd16(&after, ifd);
        assert_eq!(n_after, n_before + 1, "exactly one new entry");

        let mut tags: Vec<u16> = (0..n_after as usize)
            .map(|i| rd16(&after, ifd + 2 + i * 12))
            .collect();
        let sorted = { let mut t = tags.clone(); t.sort_unstable(); t };
        assert_eq!(tags, sorted, "IFD entries must stay in ascending tag order");
        assert!(tags.contains(&700), "XMP tag must be present");
        tags.dedup();
        assert_eq!(tags.len(), n_after as usize, "no duplicate tags");
    }

    #[test]
    fn tiff_is_not_double_tagged_on_re_export() {
        let mut tiff = encode_tiff(4, 4);
        insert_xmp_into_tiff(&mut tiff, &build_xmp_packet(&["one".to_string()])).unwrap();
        let once = tiff.clone();
        insert_xmp_into_tiff(&mut tiff, &build_xmp_packet(&["two".to_string()])).unwrap();
        assert_eq!(tiff, once, "second write must be a no-op, not a rival entry");
    }

    #[test]
    fn tiff_rejects_garbage_rather_than_mangling_it() {
        let mut junk = vec![b'X'; 64];
        let before = junk.clone();
        assert!(insert_xmp_into_tiff(&mut junk, "<x/>").is_err());
        assert_eq!(junk, before);
    }

    #[test]
    fn has_embedded_xmp_is_false_for_a_plain_encode() {
        assert!(!has_embedded_xmp(&encode_tiff(4, 4)));
    }
}
