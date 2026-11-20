use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::decode::formats::is_raw_file;
use crate::library::file_ops;
use crate::metadata::exif_cache::get_exif_from_rrcache;
use crate::metadata::sidecar::{
    update_sidecar,
    get_primary_sidecar_path, load_primary_metadata, update_primary_metadata,
};
use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use exif::{Exif, In, Value};
use rawler::decoders::RawMetadata;
use rawler::lens::LensDescription;
use std::path::PathBuf;

fn clean_creation_datetime_str(s: &str) -> &str {
    s.trim().trim_matches('"').trim_matches('\'').trim()
}

pub fn format_creation_datetime(s: &str, format: &str) -> String {
    if let Some(dt) = parse_creation_datetime(s) {
        return dt.format(format).to_string();
    }
    clean_creation_datetime_str(s).to_string()
}

fn fmt_date_str(s: String) -> String {
    format_creation_datetime(&s, "%Y-%m-%d %H:%M:%S")
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

fn creation_datetime_to_utc(dt: NaiveDateTime) -> DateTime<Utc> {
    match Local.from_local_datetime(&dt) {
        LocalResult::Single(local_dt) | LocalResult::Ambiguous(local_dt, _) => {
            local_dt.with_timezone(&Utc)
        }
        LocalResult::None => DateTime::from_naive_utc_and_offset(dt, Utc),
    }
}

fn parse_creation_field(field: &exif::Field) -> Option<DateTime<Utc>> {
    parse_creation_datetime(&field.display_value().to_string()).map(creation_datetime_to_utc)
}

fn parse_raw_creation_date(date_str: Option<&str>) -> Option<DateTime<Utc>> {
    parse_creation_datetime(date_str?).map(creation_datetime_to_utc)
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

fn format_lens_spec(focal_min: f32, focal_max: f32, aperture: Option<(f32, f32)>) -> String {
    let mut spec = format!("{} mm", format_min_max(focal_min, focal_max, 0.01));
    if let Some((amin, amax)) = aperture {
        spec.push_str(&format!(", f/{}", format_min_max(amin, amax, 0.01)));
    }
    spec
}

fn format_lens_specification(components: &[exif::Rational]) -> Option<String> {
    if components.len() < 4 {
        return None;
    }

    let focal_min = rational_to_f32_checked(&components[0])?;
    let focal_max = rational_to_f32_checked(&components[1])?;
    let aperture =
        rational_to_f32_checked(&components[2]).zip(rational_to_f32_checked(&components[3]));

    Some(format_lens_spec(focal_min, focal_max, aperture))
}

pub fn is_tiff(file_bytes: &[u8]) -> bool {
    file_bytes.starts_with(b"II*\0") || file_bytes.starts_with(b"MM\0*")
}

pub fn read_exif(file_bytes: &[u8]) -> Option<Exif> {
    let exifreader = exif::Reader::new();
    if is_tiff(file_bytes) {
        let mut len = 64 * 1024;
        while len < file_bytes.len() && len <= 16 * 1024 * 1024 {
            if let Ok(exif) = exifreader.read_raw(file_bytes[..len].to_vec()) {
                return Some(exif);
            }
            len *= 4;
        }
    }
    exifreader
        .read_from_container(&mut Cursor::new(file_bytes))
        .ok()
}

pub fn read_raw_metadata(file_bytes: &[u8]) -> Option<RawMetadata> {
    crate::decode::raw_processing::read_from_prefix(file_bytes, |bytes, _| {
        raw_metadata_from_source(&rawler::rawsource::RawSource::new_from_slice(bytes))
    })
}

pub fn read_raw_metadata_from_path(path_str: &str) -> Option<RawMetadata> {
    match file_ops::read_file_mapped(Path::new(path_str)) {
        Ok(mmap) => read_raw_metadata(&mmap),
        Err(_) => read_raw_metadata(&fs::read(path_str).ok()?),
    }
}

fn raw_metadata_from_source(raw_source: &rawler::rawsource::RawSource) -> Option<RawMetadata> {
    let decoder = rawler::get_decoder(raw_source).ok()?;
    decoder.raw_metadata(raw_source, &Default::default()).ok()
}

fn insert_missing_lens_fields(map: &mut HashMap<String, String>, lens: &LensDescription) {
    let rat =
        |r: &rawler::formats::tiff::Rational| rawler_rational_to_f32_checked(r).unwrap_or(0.0);

    let aperture = (rat(&lens.aperture_range[0]), rat(&lens.aperture_range[1]));
    let spec = format_lens_spec(
        rat(&lens.focal_range[0]),
        rat(&lens.focal_range[1]),
        Some(aperture).filter(|(amin, amax)| *amin > 0.0 || *amax > 0.0),
    );

    for (key, val) in [
        ("LensModel", lens.lens_model.as_str()),
        ("LensMake", lens.lens_make.as_str()),
        ("LensSpecification", spec.as_str()),
    ] {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            map.entry(key.to_string())
                .or_insert_with(|| truncate_large_exif(trimmed));
        }
    }
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

pub fn read_focal_length_35mm(file_bytes: &[u8]) -> Option<f32> {
    let exif = read_exif(file_bytes)?;
    let field = exif.get_field(exif::Tag::FocalLengthIn35mmFilm, In::PRIMARY)?;
    let value = field.value.get_uint(0)? as f32;
    (value > 0.0).then_some(value)
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

                        let aperture = aperture.filter(|(amin, amax)| *amin > 0.0 || *amax > 0.0);

                        map.insert(
                            "LensSpecification".to_string(),
                            format_lens_spec(focal_min, focal_max, aperture),
                        );
                    }
                }
                _ => match &field.value {
                    exif::Value::Ascii(_) => {
                        if let Some(val) = clean_ascii_value(&field.value) {
                            map.insert(field.tag.to_string(), truncate_large_exif(&val));
                        }
                    }
                    _ => {
                        let val = field.display_value().with_unit(&exif_obj).to_string();
                        if !val.trim().is_empty() {
                            map.insert(field.tag.to_string(), truncate_large_exif(&val));
                        }
                    }
                },
            }
        }
    }

    if !map.is_empty() {
        if !map.contains_key("LensModel")
            && let Some(lens) = read_raw_metadata(file_bytes).and_then(|meta| meta.lens)
        {
            insert_missing_lens_fields(&mut map, &lens);
        }
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
    }

    if let Some(v) = exif.lens_make {
        insert_if_present("LensMake", v);
    }

    if let Some(v) = exif.lens_serial_number {
        insert_if_present("LensSerialNumber", v);
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

    if let Some(lens) = &metadata.lens {
        insert_missing_lens_fields(&mut map, lens);
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

/// Positions `file` at the embedded JPEG of a Fuji RAF (its EXIF sits at the
/// front of that JPEG), or back at the start for any other file. kamadak-exif
/// doesn't know the RAF container, so without this every EXIF read of a RAF
/// fails: dates fall through to rawler (which maps the whole 30+ MB file) and
/// the orientation tag is never seen.
pub(crate) fn seek_to_raf_embedded_jpeg(file: &mut fs::File) {
    use std::io::{Read, Seek, SeekFrom};

    let mut header = [0u8; 88];
    let jpeg_offset = match file.read_exact(&mut header) {
        Ok(()) if header.starts_with(b"FUJIFILMCCD-RAW") => {
            u32::from_be_bytes([header[84], header[85], header[86], header[87]]) as u64
        }
        _ => 0,
    };
    let _ = file.seek(SeekFrom::Start(jpeg_offset));
}

pub fn try_get_exif_creation_date(path: &Path) -> Option<DateTime<Utc>> {
    if let Some(map) = read_rrexif_sidecar(path)
        && let Some(dt_str) = map.get("DateTimeOriginal").or(map.get("CreateDate"))
        && let Some(dt) = parse_creation_datetime(dt_str)
    {
        return Some(creation_datetime_to_utc(dt));
    }

    if let Ok(mut file) = std::fs::File::open(path) {
        seek_to_raf_embedded_jpeg(&mut file);
        let mut bufreader = std::io::BufReader::new(&file);
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

    if is_raw_file(path)
        && let Some(metadata) = read_raw_metadata_from_path(&path.to_string_lossy())
    {
        if let Some(dt) = parse_raw_creation_date(metadata.exif.date_time_original.as_deref()) {
            return Some(dt);
        }
        if let Some(dt) = parse_raw_creation_date(metadata.exif.create_date.as_deref()) {
            return Some(dt);
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

pub fn read_rrexif_sidecar(image_path: &Path) -> Option<HashMap<String, String>> {
    let metadata = load_primary_metadata(image_path);
    if let Some(mut exif) = metadata.exif {
        if heal_cached_user_comment(&mut exif) {
            let healed = exif.clone();
            let _ = update_sidecar(&get_primary_sidecar_path(image_path), image_path, |m| {
                m.exif = Some(healed);
            });
        }
        return Some(exif);
    }

    if let Some(exif) = get_exif_from_rrcache(image_path) {
        return Some(exif);
    }

    let legacy = get_rrexif_path(image_path);
    if legacy.exists()
        && let Ok(content) = fs::read_to_string(&legacy)
        && let Ok(mut map) = serde_json::from_str::<HashMap<String, String>>(&content)
    {
        heal_cached_user_comment(&mut map);
        let migrated = map.clone();
        if update_sidecar(&get_primary_sidecar_path(image_path), image_path, |m| {
            m.exif = Some(migrated);
        })
        .is_ok()
        {
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

    update_primary_metadata(target_image_path, true, |metadata| {
        metadata.exif = Some(exif_data);
    })
    .map(|_| ())
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
pub(crate) fn write_xmp_only(image_bytes: &mut Vec<u8>, original_path: &Path, format: &str) {
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

#[cfg(test)]
mod xmp_tests {
    use super::*;
    use crate::metadata::export_metadata::{ExportXmp, write_image_with_metadata};
    use crate::metadata::sidecar::ImageMetadata;

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
    fn round_trips_through_the_apps_own_tag_parser() {
        let tags = vec!["landscape".to_string(), "black & white".to_string()];
        let xmp = build_xmp_packet(&tags);
        // The importer is the real consumer — what we write it must read back.
        let parsed = crate::metadata::xmp::extract_xmp_tags(&xmp);
        assert_eq!(parsed, tags, "ampersand must survive escape + parse");
    }

    /// End-to-end: a real `.rrdata` sidecar on disk -> exported JPEG -> tags
    /// parsed back out by the importer's own XMP parser.
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
        write_image_with_metadata(
            &mut bytes,
            source.to_str().unwrap(),
            "jpg",
            false,
            &ExportXmp::from_metadata(&meta),
            None,
            true,
        )
        .unwrap();

        let text = String::from_utf8_lossy(&bytes);
        let parsed = crate::metadata::xmp::extract_xmp_tags(&text);
        assert_eq!(parsed, vec!["pizza".to_string(), "black & white".to_string()]);

        // Still a valid JPEG after the splice.
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
        assert!(image::load_from_memory(&bytes).is_ok(), "must still decode");

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
        write_image_with_metadata(
            &mut bytes,
            source.to_str().unwrap(),
            "png",
            false,
            &ExportXmp::from_metadata(&meta),
            None,
            true,
        )
        .unwrap();

        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("XML:com.adobe.xmp"), "must use the spec keyword");
        assert_eq!(
            crate::metadata::xmp::extract_xmp_tags(&text),
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
    fn a_format_we_cannot_embed_leaves_the_bytes_untouched() {
        use image::{DynamicImage, RgbImage};
        let dir = std::env::temp_dir().join(format!("rr-avif-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("frame.png");
        DynamicImage::ImageRgb8(RgbImage::new(8, 8)).save(&source).unwrap();
        let meta = ImageMetadata { tags: Some(vec!["t".to_string()]), ..Default::default() };
        std::fs::write(get_primary_sidecar_path(&source), serde_json::to_string(&meta).unwrap()).unwrap();

        let mut bytes = std::fs::read(&source).unwrap();
        let before = bytes.clone();
        write_image_with_metadata(
            &mut bytes,
            source.to_str().unwrap(),
            "avif",
            false,
            &ExportXmp::from_metadata(&meta),
            None,
            true,
        )
        .unwrap();
        assert_eq!(bytes, before, "avif must be left for the sidecar path");

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
            crate::metadata::xmp::extract_xmp_tags(&String::from_utf8_lossy(&tiff)),
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

#[cfg(test)]
mod raf_date_tests {
    use super::*;

    /// A RAF with nothing but the header and its embedded JPEG — no sensor data,
    /// so rawler cannot decode it. Getting a date back proves the header path.
    #[test]
    fn raf_capture_date_comes_from_the_embedded_jpeg_exif() {
        let field = exif::Field {
            tag: exif::Tag::DateTimeOriginal,
            ifd_num: exif::In::PRIMARY,
            value: exif::Value::Ascii(vec![b"2024:03:05 14:07:09".to_vec()]),
        };
        let mut writer = exif::experimental::Writer::new();
        writer.push_field(&field);
        let mut tiff = Cursor::new(Vec::new());
        writer.write(&mut tiff, false).unwrap();
        let tiff = tiff.into_inner();

        const JPEG_OFFSET: usize = 148;
        let mut raf = b"FUJIFILMCCD-RAW ".to_vec();
        raf.resize(84, 0);
        raf.extend_from_slice(&(JPEG_OFFSET as u32).to_be_bytes());
        raf.resize(JPEG_OFFSET, 0);
        raf.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE1]);
        raf.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        raf.extend_from_slice(b"Exif\0\0");
        raf.extend_from_slice(&tiff);
        raf.extend_from_slice(&[0xFF, 0xD9]);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("DSCF0001.RAF");
        fs::write(&path, raf).unwrap();

        let expected = creation_datetime_to_utc(
            NaiveDateTime::parse_from_str("2024-03-05 14:07:09", "%Y-%m-%d %H:%M:%S").unwrap(),
        );
        assert_eq!(try_get_exif_creation_date(&path), Some(expected));
    }
}

pub fn get_rrexif_path(image_path: &Path) -> PathBuf {
    let mut filename = image_path.file_name().unwrap_or_default().to_os_string();
    filename.push(".rrexif");
    image_path.with_file_name(filename)
}

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
