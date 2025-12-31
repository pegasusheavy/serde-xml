//! XML escape and unescape utilities.
//!
//! This module provides fast, allocation-minimizing functions for escaping
//! and unescaping XML special characters.

use memchr::memchr;

/// Escapes XML special characters in a string.
///
/// Returns a `Cow<str>` to avoid allocation when no escaping is needed.
#[inline]
pub fn escape(s: &str) -> std::borrow::Cow<'_, str> {
    let bytes = s.as_bytes();
    
    // Fast path: scan for any character needing escape
    let needs_escape = bytes.iter().any(|&b| matches!(b, b'<' | b'>' | b'&' | b'"' | b'\''));
    
    if !needs_escape {
        return std::borrow::Cow::Borrowed(s);
    }

    let mut result = String::with_capacity(s.len() + s.len() / 8);
    escape_to_inner(bytes, &mut result);
    std::borrow::Cow::Owned(result)
}

/// Escapes XML special characters and appends to the given string.
#[inline]
pub fn escape_to(s: &str, out: &mut String) {
    escape_to_inner(s.as_bytes(), out);
}

/// Internal escape implementation - simple byte-by-byte with batching.
#[inline(always)]
fn escape_to_inner(bytes: &[u8], out: &mut String) {
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
        
        // Batch append non-escaped bytes
        if start < i {
            // SAFETY: Only escaping ASCII chars, so UTF-8 boundaries are preserved
            out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
        }
        out.push_str(escaped);
        start = i + 1;
    }
    
    // Append remaining
    if start < bytes.len() {
        out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..]) });
    }
}

/// Escapes XML special characters for attribute values.
#[inline]
pub fn escape_attr(s: &str) -> std::borrow::Cow<'_, str> {
    escape(s)
}

/// Unescapes XML entities in a string.
///
/// Returns a `Cow<str>` to avoid allocation when no unescaping is needed.
#[inline]
pub fn unescape(s: &str) -> Result<std::borrow::Cow<'_, str>, UnescapeError> {
    let bytes = s.as_bytes();
    
    // Fast path: check if any unescaping is needed using memchr
    match memchr(b'&', bytes) {
        None => Ok(std::borrow::Cow::Borrowed(s)),
        Some(first_amp) => {
            let mut result = String::with_capacity(s.len());
            // Add everything before the first &
            if first_amp > 0 {
                result.push_str(unsafe { 
                    std::str::from_utf8_unchecked(&bytes[..first_amp]) 
                });
            }
            unescape_from(bytes, first_amp, &mut result)?;
            Ok(std::borrow::Cow::Owned(result))
        }
    }
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
#[inline]
pub fn unescape_to(s: &str, out: &mut String) -> Result<(), UnescapeError> {
    let bytes = s.as_bytes();
    match memchr(b'&', bytes) {
        None => {
            out.push_str(s);
            Ok(())
        }
        Some(first_amp) => {
            if first_amp > 0 {
                out.push_str(unsafe { 
                    std::str::from_utf8_unchecked(&bytes[..first_amp]) 
                });
            }
            unescape_from(bytes, first_amp, out)
        }
    }
}

/// Internal unescape starting from a position known to have '&'.
#[inline(always)]
fn unescape_from(bytes: &[u8], start: usize, out: &mut String) -> Result<(), UnescapeError> {
    let mut i = start;
    
    while i < bytes.len() {
        if bytes[i] == b'&' {
            let entity_start = i;
            i += 1;
            
            // Find semicolon using memchr for speed
            match memchr(b';', &bytes[i..]) {
                Some(len) if len > 0 && len <= 10 => {
                    let entity = unsafe { 
                        std::str::from_utf8_unchecked(&bytes[i..i + len]) 
                    };
                    
                    if let Some(c) = decode_entity_fast(entity) {
                        out.push(c);
                        i += len + 1;
                        
                        // Find and append text until next &
                        if let Some(next_amp) = memchr(b'&', &bytes[i..]) {
                            if next_amp > 0 {
                                out.push_str(unsafe { 
                                    std::str::from_utf8_unchecked(&bytes[i..i + next_amp]) 
                                });
                            }
                            i += next_amp;
                        } else {
                            // No more entities
                            out.push_str(unsafe { 
                                std::str::from_utf8_unchecked(&bytes[i..]) 
                            });
                            return Ok(());
                        }
                    } else {
                        return Err(UnescapeError {
                            entity: format!("&{};", entity),
                            position: entity_start,
                        });
                    }
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
    
    Ok(())
}

/// Fast entity decoder with common cases first.
#[inline(always)]
fn decode_entity_fast(entity: &str) -> Option<char> {
    // Check length first to avoid string comparisons
    match entity.len() {
        2 => match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => decode_numeric_entity(entity),
        },
        3 => match entity {
            "amp" => Some('&'),
            _ => decode_numeric_entity(entity),
        },
        4 => match entity {
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => decode_numeric_entity(entity),
        },
        _ => decode_numeric_entity(entity),
    }
}

/// Decodes a numeric character reference (&#NNN; or &#xHHH;).
#[inline]
fn decode_numeric_entity(entity: &str) -> Option<char> {
    let bytes = entity.as_bytes();
    if bytes.is_empty() || bytes[0] != b'#' {
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
