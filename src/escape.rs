//! XML escape and unescape utilities.
//!
//! This module provides fast, allocation-minimizing functions for escaping
//! and unescaping XML special characters.

use memchr::memchr3;


/// Escapes XML special characters in a string.
///
/// Returns a `Cow<str>` to avoid allocation when no escaping is needed.
#[inline]
pub fn escape(s: &str) -> std::borrow::Cow<'_, str> {
    // Fast path: check if any escaping is needed
    if !needs_escape(s.as_bytes()) {
        return std::borrow::Cow::Borrowed(s);
    }

    let mut result = String::with_capacity(s.len() + s.len() / 8);
    escape_to(s, &mut result);
    std::borrow::Cow::Owned(result)
}

/// Checks if a byte slice needs escaping.
#[inline]
fn needs_escape(bytes: &[u8]) -> bool {
    memchr3(b'<', b'>', b'&', bytes).is_some()
        || memchr::memchr2(b'"', b'\'', bytes).is_some()
}

/// Escapes XML special characters and appends to the given string.
#[inline]
pub fn escape_to(s: &str, out: &mut String) {
    let bytes = s.as_bytes();
    let mut start = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        let escaped = match byte {
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'&' => "&amp;",
            b'"' => "&quot;",
            b'\'' => "&apos;",
            _ => continue,
        };

        if start < i {
            // SAFETY: We're slicing at valid UTF-8 boundaries since we only
            // escape ASCII characters.
            out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
        }
        out.push_str(escaped);
        start = i + 1;
    }

    if start < bytes.len() {
        out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..]) });
    }
}

/// Escapes XML special characters for attribute values.
///
/// This is the same as `escape` but optimized for attribute values.
#[inline]
pub fn escape_attr(s: &str) -> std::borrow::Cow<'_, str> {
    escape(s)
}

/// Unescapes XML entities in a string.
///
/// Returns a `Cow<str>` to avoid allocation when no unescaping is needed.
#[inline]
pub fn unescape(s: &str) -> Result<std::borrow::Cow<'_, str>, UnescapeError> {
    // Fast path: check if any unescaping is needed
    if !s.contains('&') {
        return Ok(std::borrow::Cow::Borrowed(s));
    }

    let mut result = String::with_capacity(s.len());
    unescape_to(s, &mut result)?;
    Ok(std::borrow::Cow::Owned(result))
}

/// Error type for unescape operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnescapeError {
    /// The invalid entity that caused the error.
    pub entity: String,
    /// Position in the input where the error occurred.
    pub position: usize,
}

impl std::fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid XML entity '{}' at position {}", self.entity, self.position)
    }
}

impl std::error::Error for UnescapeError {}

/// Unescapes XML entities and appends to the given string.
pub fn unescape_to(s: &str, out: &mut String) -> Result<(), UnescapeError> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut start = 0;

    while i < bytes.len() {
        if bytes[i] == b'&' {
            // Append text before the entity
            if start < i {
                out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
            }

            let entity_start = i;
            i += 1;

            // Find the end of the entity
            let semicolon = bytes[i..].iter().position(|&b| b == b';');
            match semicolon {
                Some(len) if len > 0 => {
                    let entity = unsafe { std::str::from_utf8_unchecked(&bytes[i..i + len]) };
                    let decoded = decode_entity(entity);

                    match decoded {
                        Some(c) => out.push(c),
                        None => {
                            // Try numeric character reference
                            if let Some(c) = decode_numeric_entity(entity) {
                                out.push(c);
                            } else {
                                return Err(UnescapeError {
                                    entity: format!("&{};", entity),
                                    position: entity_start,
                                });
                            }
                        }
                    }

                    i += len + 1;
                    start = i;
                }
                _ => {
                    return Err(UnescapeError {
                        entity: String::from("&"),
                        position: entity_start,
                    });
                }
            }
        } else {
            i += 1;
        }
    }

    if start < bytes.len() {
        out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..]) });
    }

    Ok(())
}

/// Decodes a named XML entity.
#[inline]
fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// Decodes a numeric character reference (&#NNN; or &#xHHH;).
#[inline]
fn decode_numeric_entity(entity: &str) -> Option<char> {
    if entity.is_empty() {
        return None;
    }

    let bytes = entity.as_bytes();
    if bytes[0] != b'#' {
        return None;
    }

    let (radix, digits) = if bytes.len() > 1 && (bytes[1] == b'x' || bytes[1] == b'X') {
        (16, &entity[2..])
    } else {
        (10, &entity[1..])
    };

    if digits.is_empty() {
        return None;
    }

    let code = u32::from_str_radix(digits, radix).ok()?;
    char::from_u32(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_no_special_chars() {
        let s = "Hello, World!";
        let escaped = escape(s);
        assert!(matches!(escaped, std::borrow::Cow::Borrowed(_)));
        assert_eq!(escaped, s);
    }

    #[test]
    fn test_escape_lt() {
        assert_eq!(escape("<"), "&lt;");
    }

    #[test]
    fn test_escape_gt() {
        assert_eq!(escape(">"), "&gt;");
    }

    #[test]
    fn test_escape_amp() {
        assert_eq!(escape("&"), "&amp;");
    }

    #[test]
    fn test_escape_quot() {
        assert_eq!(escape("\""), "&quot;");
    }

    #[test]
    fn test_escape_apos() {
        assert_eq!(escape("'"), "&apos;");
    }

    #[test]
    fn test_escape_mixed() {
        assert_eq!(
            escape("<div class=\"foo\">Hello & goodbye</div>"),
            "&lt;div class=&quot;foo&quot;&gt;Hello &amp; goodbye&lt;/div&gt;"
        );
    }

    #[test]
    fn test_unescape_no_entities() {
        let s = "Hello, World!";
        let unescaped = unescape(s).unwrap();
        assert!(matches!(unescaped, std::borrow::Cow::Borrowed(_)));
        assert_eq!(unescaped, s);
    }

    #[test]
    fn test_unescape_lt() {
        assert_eq!(unescape("&lt;").unwrap(), "<");
    }

    #[test]
    fn test_unescape_gt() {
        assert_eq!(unescape("&gt;").unwrap(), ">");
    }

    #[test]
    fn test_unescape_amp() {
        assert_eq!(unescape("&amp;").unwrap(), "&");
    }

    #[test]
    fn test_unescape_quot() {
        assert_eq!(unescape("&quot;").unwrap(), "\"");
    }

    #[test]
    fn test_unescape_apos() {
        assert_eq!(unescape("&apos;").unwrap(), "'");
    }

    #[test]
    fn test_unescape_mixed() {
        assert_eq!(
            unescape("&lt;div class=&quot;foo&quot;&gt;Hello &amp; goodbye&lt;/div&gt;").unwrap(),
            "<div class=\"foo\">Hello & goodbye</div>"
        );
    }

    #[test]
    fn test_unescape_numeric_decimal() {
        assert_eq!(unescape("&#65;").unwrap(), "A");
        assert_eq!(unescape("&#97;").unwrap(), "a");
        assert_eq!(unescape("&#8364;").unwrap(), "€");
    }

    #[test]
    fn test_unescape_numeric_hex() {
        assert_eq!(unescape("&#x41;").unwrap(), "A");
        assert_eq!(unescape("&#x61;").unwrap(), "a");
        assert_eq!(unescape("&#x20AC;").unwrap(), "€");
    }

    #[test]
    fn test_unescape_invalid_entity() {
        let result = unescape("&invalid;");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.entity, "&invalid;");
        assert_eq!(err.position, 0);
    }

    #[test]
    fn test_unescape_unterminated_entity() {
        let result = unescape("&lt");
        assert!(result.is_err());
    }

    #[test]
    fn test_escape_to() {
        let mut out = String::new();
        escape_to("<test>", &mut out);
        assert_eq!(out, "&lt;test&gt;");
    }

    #[test]
    fn test_roundtrip() {
        let original = "<div class=\"foo\">Hello & goodbye</div>";
        let escaped = escape(original);
        let unescaped = unescape(&escaped).unwrap();
        assert_eq!(unescaped, original);
    }
}
