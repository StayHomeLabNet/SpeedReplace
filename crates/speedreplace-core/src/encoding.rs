use crate::{EncodingInfo, Error, Result};
use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE};

pub(crate) struct DecodedText {
    pub text: String,
    pub encoding: &'static Encoding,
    pub info: EncodingInfo,
}

const SUPPORTED: &[&str] = &[
    "UTF-8",
    "UTF-16LE",
    "UTF-16BE",
    "Shift_JIS",
    "EUC-JP",
    "ISO-2022-JP",
    "windows-1252",
];

pub(crate) fn decode(bytes: &[u8], encoding_override: Option<&str>) -> Result<DecodedText> {
    let detected_bom = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some((UTF_8, 3))
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some((UTF_16LE, 2))
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some((UTF_16BE, 2))
    } else {
        None
    };
    let (encoding, bom_len, confidence) = if let Some(label) = encoding_override {
        let encoding = Encoding::for_label(label.as_bytes())
            .ok_or_else(|| Error::Encoding(format!("unknown encoding override: {label}")))?;
        if !SUPPORTED.iter().any(|name| *name == encoding.name()) {
            return Err(Error::Encoding(format!(
                "encoding override is not supported: {label}"
            )));
        }
        let bom_len = detected_bom
            .filter(|(bom_encoding, _)| *bom_encoding == encoding)
            .map_or(0, |(_, len)| len);
        (encoding, bom_len, "manual")
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (UTF_8, 3, "bom")
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        (UTF_16LE, 2, "bom")
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (UTF_16BE, 2, "bom")
    } else if std::str::from_utf8(bytes).is_ok() {
        (UTF_8, 0, "exact")
    } else {
        let mut detector = EncodingDetector::new();
        detector.feed(bytes, true);
        let guessed = detector.guess(None, true);
        if !SUPPORTED.iter().any(|name| *name == guessed.name()) {
            return Err(Error::Encoding(format!(
                "detected {} which is not enabled",
                guessed.name()
            )));
        }
        (guessed, 0, "heuristic")
    };

    let body = &bytes[bom_len..];
    let text = encoding
        .decode_without_bom_handling_and_without_replacement(body)
        .ok_or_else(|| Error::Encoding(format!("invalid {} byte sequence", encoding.name())))?
        .into_owned();

    let round_trip = encode_body(&text, encoding)?;
    if round_trip != body {
        return Err(Error::Encoding(format!(
            "{} did not pass exact round-trip validation",
            encoding.name()
        )));
    }

    Ok(DecodedText {
        text,
        encoding,
        info: EncodingInfo {
            name: encoding.name().to_owned(),
            has_bom: bom_len > 0,
            confidence: confidence.to_owned(),
        },
    })
}

pub(crate) fn encode(text: &str, source: &DecodedText) -> Result<Vec<u8>> {
    let encoded = encode_body(text, source.encoding)?;
    let mut output = Vec::with_capacity(encoded.len() + 3);
    if source.info.has_bom {
        if source.encoding == UTF_8 {
            output.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        } else if source.encoding == UTF_16LE {
            output.extend_from_slice(&[0xFF, 0xFE]);
        } else if source.encoding == UTF_16BE {
            output.extend_from_slice(&[0xFE, 0xFF]);
        }
    }
    output.extend_from_slice(&encoded);
    Ok(output)
}

fn encode_body(text: &str, encoding: &'static Encoding) -> Result<Vec<u8>> {
    if encoding == UTF_16LE {
        return Ok(text
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>());
    }
    if encoding == UTF_16BE {
        return Ok(text
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>());
    }
    let (encoded, _, had_errors) = encoding.encode(text);
    if had_errors {
        return Err(Error::Unmappable(encoding.name().to_owned()));
    }
    Ok(encoded.into_owned())
}

pub(crate) fn line_endings(text: &str) -> String {
    let crlf = text.matches("\r\n").count();
    let without_crlf = text.replace("\r\n", "");
    let lf = without_crlf.matches('\n').count();
    let cr = without_crlf.matches('\r').count();
    let kinds = [crlf > 0, lf > 0, cr > 0]
        .into_iter()
        .filter(|present| *present)
        .count();
    if kinds > 1 {
        "mixed".to_owned()
    } else if crlf > 0 {
        "CRLF".to_owned()
    } else if lf > 0 {
        "LF".to_owned()
    } else if cr > 0 {
        "CR".to_owned()
    } else {
        "none".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::SHIFT_JIS;

    #[test]
    fn utf8_bom_is_preserved() {
        let input = b"\xEF\xBB\xBFhello\r\n";
        let decoded = decode(input, None).unwrap();
        assert_eq!(decoded.text, "hello\r\n");
        assert!(decoded.info.has_bom);
        assert_eq!(encode(&decoded.text, &decoded).unwrap(), input);
    }

    #[test]
    fn utf16le_bom_is_preserved() {
        let input = [0xFF, 0xFE, b'a', 0, b'\n', 0];
        let decoded = decode(&input, None).unwrap();
        assert_eq!(decoded.text, "a\n");
        assert_eq!(decoded.info.name, "UTF-16LE");
        assert_eq!(encode(&decoded.text, &decoded).unwrap(), input);
    }

    #[test]
    fn shift_jis_round_trip_is_exact() {
        let (encoded, _, had_errors) = SHIFT_JIS.encode("置換テスト");
        assert!(!had_errors);
        let decoded = decode(&encoded, None).unwrap();
        assert_eq!(decoded.text, "置換テスト");
        assert_eq!(encode(&decoded.text, &decoded).unwrap(), encoded.as_ref());
    }

    #[test]
    fn line_ending_detection_covers_mixed_files() {
        assert_eq!(line_endings("a\r\nb\r\n"), "CRLF");
        assert_eq!(line_endings("a\nb\n"), "LF");
        assert_eq!(line_endings("a\r\nb\n"), "mixed");
    }

    #[test]
    fn manual_encoding_override_is_reported() {
        let (encoded, _, _) = SHIFT_JIS.encode("置換");
        let decoded = decode(&encoded, Some("shift_jis")).unwrap();
        assert_eq!(decoded.info.confidence, "manual");
        assert_eq!(decoded.text, "置換");
    }
}
