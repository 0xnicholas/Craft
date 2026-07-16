use std::io::{self, Read};

#[derive(Debug, Clone, PartialEq)]
pub struct LspMessage {
    pub json: serde_json::Value,
}

pub fn parse_content_length_header(line: &str) -> Option<usize> {
    let lower = line.trim().to_ascii_lowercase();
    let prefix = "content-length:";
    if !lower.starts_with(prefix) {
        return None;
    }
    lower[prefix.len()..].trim().parse().ok()
}

pub fn frame_inbound<R: Read>(reader: &mut R) -> io::Result<Option<LspMessage>> {
    let mut headers: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut byte = [0u8; 1];

    loop {
        match reader.read(&mut byte) {
            Ok(0) => return Ok(None),
            Ok(_) => {
                if byte[0] == b'\n' {
                    let trimmed = current.trim_end_matches('\r').to_string();
                    current.clear();
                    if trimmed.is_empty() {
                        if headers.is_empty() {
                            return Ok(None);
                        }
                        break;
                    }
                    headers.push(trimmed);
                } else {
                    current.push(byte[0] as char);
                }
            }
            Err(e) => return Err(e),
        }
    }

    let len = headers
        .iter()
        .find_map(|h| parse_content_length_header(h))
        .ok_or_else(|| io::Error::other("missing Content-Length"))?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    let json: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| io::Error::other(format!("bad LSP JSON: {e}")))?;
    Ok(Some(LspMessage { json }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_content_length_header_extracts_number() {
        assert_eq!(parse_content_length_header("Content-Length: 42"), Some(42));
        assert_eq!(parse_content_length_header("content-length: 0"), Some(0));
        assert_eq!(
            parse_content_length_header("Content-Type: application/json"),
            None
        );
    }

    #[test]
    fn frame_inbound_decodes_message() {
        let body = br#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let framed = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut bytes = framed.into_bytes();
        bytes.extend_from_slice(body);
        let mut cursor = Cursor::new(bytes);
        let msg = frame_inbound(&mut cursor).unwrap().unwrap();
        assert_eq!(msg.json["id"], 1);
        assert_eq!(msg.json["result"], serde_json::Value::Null);
    }
}
