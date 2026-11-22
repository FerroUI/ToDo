use crate::{InteropError, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use encoding_rs::Encoding;
use outlook_pst::ltp::prop_context::PropertyValue;
use serde_json::{json, Value};

pub(crate) fn integer(value: Option<&PropertyValue>) -> Option<i64> {
    match value? {
        PropertyValue::Integer16(n) => Some(*n as i64),
        PropertyValue::Integer32(n) => Some(*n as i64),
        PropertyValue::Integer64(n) => Some(*n),
        _ => None,
    }
}
pub(crate) fn boolean(value: Option<&PropertyValue>) -> Option<bool> {
    match value? {
        PropertyValue::Boolean(value) => Some(*value),
        _ => None,
    }
}
pub(crate) fn text(value: Option<&PropertyValue>, codepage: u32) -> Result<Option<String>> {
    let value = match value {
        Some(PropertyValue::Unicode(value)) => String::from_utf16(value.buffer())
            .map_err(|_| InteropError::new("A PST text property contains invalid UTF-16."))?,
        Some(PropertyValue::String8(value)) => decode(value.buffer(), codepage)?,
        _ => return Ok(None),
    };
    Ok(Some(value.trim_end_matches('\0').to_owned()))
}
pub(crate) fn decode(bytes: &[u8], codepage: u32) -> Result<String> {
    let label = match codepage {
        0 | 1252 => "windows-1252".to_owned(),
        65001 => "utf-8".into(),
        932 => "shift_jis".into(),
        936 => "gbk".into(),
        949 => "euc-kr".into(),
        950 => "big5".into(),
        1200 => "utf-16le".into(),
        1201 => "utf-16be".into(),
        n => format!("windows-{n}"),
    };
    let encoding = Encoding::for_label(label.as_bytes()).ok_or_else(|| {
        InteropError::new(format!(
            "PST code page {codepage} is unsupported; export a Unicode PST."
        ))
    })?;
    let (value, _, invalid) = encoding.decode(bytes);
    if invalid {
        return Err(InteropError::new(
            "A PST text property contains invalid encoded text.",
        ));
    }
    Ok(value.trim_end_matches('\0').to_owned())
}
pub(crate) fn timestamp(value: Option<&PropertyValue>) -> Result<Option<DateTime<Utc>>> {
    let Some(PropertyValue::Time(ticks)) = value else {
        return Ok(None);
    };
    // MS-OXOTASK uses this sentinel for an absent due/start date.
    if *ticks == 0 || *ticks == 0x5AE980E0 || *ticks == 0x5AE980E0_i64 * 60 * 10_000_000 {
        return Ok(None);
    }
    let seconds = ticks.div_euclid(10_000_000) - 11_644_473_600;
    let nanos = ticks.rem_euclid(10_000_000) as u32 * 100;
    DateTime::from_timestamp(seconds, nanos)
        .map(Some)
        .ok_or_else(|| InteropError::new("A PST timestamp is outside the supported date range."))
}

/// Preserve the original typed values as data; never execute/render HTML or follow
/// URLs read from an archive. Large binary metadata is bounded by the document cap.
pub(crate) fn raw(value: &PropertyValue, codepage: u32) -> Result<Value> {
    use PropertyValue::*;
    Ok(match value {
        Null => Value::Null,
        Integer16(v) => json!({"type":"Integer16","value":v}),
        Integer32(v) => json!({"type":"Integer32","value":v}),
        Integer64(v) => json!({"type":"Integer64","value":v.to_string()}),
        Time(v) => json!({"type":"Time","value":v.to_string()}),
        Floating32(v) => json!({"type":"Floating32","value":v}),
        Floating64(v) => json!({"type":"Floating64","value":v}),
        FloatingTime(v) => json!({"type":"FloatingTime","value":v}),
        Currency(v) => json!({"type":"Currency","value":v.to_string()}),
        ErrorCode(v) => json!({"type":"ErrorCode","value":v}),
        Boolean(v) => json!({"type":"Boolean","value":v}),
        Unicode(_) | String8(_) => {
            json!({"type":if matches!(value,Unicode(_)){"Unicode"}else{"String8"},"value":text(Some(value),codepage)?})
        }
        Binary(v) => json!({"type":"Binary","base64":STANDARD.encode(v.buffer())}),
        Guid(v) => json!({"type":"Guid","value":format!("{v:?}")}),
        Object(v) => json!({"type":"Object","node":u32::from(v.node()),"size":v.size()}),
        MultipleUnicode(values) => {
            json!({"type":"MultipleUnicode","value":values.iter().map(|v|text(Some(&Unicode(v.clone())),codepage)).collect::<Result<Vec<_>>>()?})
        }
        MultipleString8(values) => {
            json!({"type":"MultipleString8","value":values.iter().map(|v|decode(v.buffer(),codepage)).collect::<Result<Vec<_>>>()?})
        }
        MultipleBinary(values) => {
            json!({"type":"MultipleBinary","base64":values.iter().map(|v|STANDARD.encode(v.buffer())).collect::<Vec<_>>()})
        }
        MultipleInteger16(v) => json!({"type":"MultipleInteger16","value":v}),
        MultipleInteger32(v) => json!({"type":"MultipleInteger32","value":v}),
        MultipleInteger64(v) => {
            json!({"type":"MultipleInteger64","value":v.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        MultipleTime(v) => {
            json!({"type":"MultipleTime","value":v.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        MultipleCurrency(v) => {
            json!({"type":"MultipleCurrency","value":v.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        MultipleFloating32(v) => json!({"type":"MultipleFloating32","value":v}),
        MultipleFloating64(v) => json!({"type":"MultipleFloating64","value":v}),
        MultipleFloatingTime(v) => json!({"type":"MultipleFloatingTime","value":v}),
        MultipleGuid(v) => {
            json!({"type":"MultipleGuid","value":v.iter().map(|v|format!("{v:?}")).collect::<Vec<_>>()})
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_codepages_and_filetimes_are_explicit() {
        assert_eq!(decode(&[0x80, b' ', 0xe9], 1252).unwrap(), "€ é");
        assert!(decode(&[0xff], 65001).is_err());
        let value = PropertyValue::Time(116444736000000000);
        assert_eq!(
            timestamp(Some(&value)).unwrap().unwrap().to_rfc3339(),
            "1970-01-01T00:00:00+00:00"
        );
        assert!(timestamp(Some(&PropertyValue::Time(0x5AE980E0)))
            .unwrap()
            .is_none());
        assert!(
            timestamp(Some(&PropertyValue::Time(0x5AE980E0_i64 * 60 * 10_000_000)))
                .unwrap()
                .is_none()
        );
    }
}
