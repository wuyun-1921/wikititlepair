use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use rayon::prelude::*;

use crate::error::Result;
use crate::escape::escape_dsl;

pub fn titles_listing_url(lang: &str, project: &str) -> String {
    format!("https://dumps.wikimedia.org/{}{}/latest/", lang, project)
}

pub fn ensure_titles_dump(
    cache_dir: &Path,
    lang: &str,
    project: &str,
    allow_download: bool,
) -> Result<PathBuf> {
    let filename = format!("{}{}-latest-all-titles-in-ns0.gz", lang, project);
    let url = format!(
        "https://dumps.wikimedia.org/{}{}/latest/{}{}-latest-all-titles-in-ns0.gz",
        lang, project, lang, project
    );
    crate::download::ensure_dump(cache_dir, &filename, &url, allow_download)
}

/// Extract all article titles from an all-titles-in-ns0 dump.
/// Returns (escaped_headword, "<a href=\"url\">url</a>") pairs.
pub fn parse_all_titles(path: &Path, lang: &str, project: &str) -> Result<Vec<(String, String)>> {
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut contents = Vec::new();
    decoder.read_to_end(&mut contents)?;

    // Convert to string - all-titles dumps are reasonably sized
    let xml = String::from_utf8_lossy(&contents);

    let titles = extract_titles(&xml);

    let base_url = format!("https://{}.{}.org/wiki/", lang, project);

    let mut entries: Vec<(String, String)> = titles
        .par_iter()
        .map(|title| {
            let escaped = escape_dsl(title);
            let url_title = url_encode_title(title);
            let url = format!("{}{}", base_url, url_title);
            let body = format!("<a href=\"{}\">{}</a>", url, url);
            (escaped, body)
        })
        .collect();

    entries.par_sort();
    entries.dedup();
    Ok(entries)
}

/// Scan XML for <title> tags inside <page> blocks.
/// all-titles-in-ns0 dumps are namespace-0 only, so no namespace filtering needed.
fn extract_titles(xml: &str) -> Vec<String> {
    let mut titles = Vec::new();
    let bytes = xml.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;

    while pos < len {
        // Find next <page> tag
        let page_start = match find_after(bytes, pos, b"<page>") {
            Some(p) => p,
            None => break,
        };

        // Find next </page> to bound our search
        let page_end = match find_after(bytes, page_start, b"</page>") {
            Some(p) => p - 7, // back to start of </page>
            None => len,
        };

        // Find <title> within this page
        let title_start = match find_after(bytes, page_start, b"<title>") {
            Some(p) => p,
            None => {
                pos = page_end;
                continue;
            }
        };

        // Must be before </page>
        if title_start >= page_end {
            pos = page_end;
            continue;
        }

        // Find </title>
        let title_end = match find_after(bytes, title_start, b"</title>") {
            Some(p) => p - 8,
            None => {
                pos = page_end;
                continue;
            }
        };

        if title_end > title_start {
            if let Ok(s) = std::str::from_utf8(&bytes[title_start..title_end]) {
                if !s.is_empty() {
                    titles.push(s.to_string());
                }
            }
        }

        pos = page_end;
    }

    titles
}

/// Find `needle` in `haystack` starting from `start`, return position just after the match.
fn find_after(haystack: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    let pos = haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle)?;
    Some(start + pos + needle.len())
}

/// Encode a page title for use in a URL path segment.
/// Spaces → underscores. Special chars → percent-encoded per MediaWiki URL conventions.
/// Keeps unreserved RFC 3986 chars plus sub-delims that MediaWiki does not encode:
/// $ & ' ( ) * , ; = : / @
fn url_encode_title(title: &str) -> String {
    let mut result = String::with_capacity(title.len() + 16);
    for b in title.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
            | b'-' | b'.' | b'_' | b'~'
            // Sub-delims that MediaWiki leaves literal
            | b'$' | b'&' | b'\'' | b'(' | b')'
            | b'*' | b',' | b';' | b'=' | b':' | b'/' | b'@' | b'!' => {
                result.push(*b as char);
            }
            b' ' => result.push('_'),
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

/// Internal helper for testing URL encoding without full XML round-trip.
#[cfg(test)]
fn parse_all_titles_inner(lang: &str, project: &str, titles: &[&str]) -> Vec<(String, String)> {
    let base_url = format!("https://{}.{}.org/wiki/", lang, project);
    titles
        .iter()
        .map(|title| {
            let escaped = escape_dsl(title);
            let url_title = url_encode_title(title);
            let url = format!("{}{}", base_url, url_title);
            let body = format!("<a href=\"{}\">{}</a>", url, url);
            (escaped, body)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_all_titles_basic() {
        // Build a minimal gzipped all-titles XML with 3 articles
        let xml = r#"<mediawiki xmlns="http://www.mediawiki.org/xml/export-0.11/" xsi:schemaLocation="http://www.mediawiki.org/xml/export-0.11/ http://www.mediawiki.org/xml/export-0.11.xsd" version="0.11" xml:lang="en">
  <siteinfo>
    <sitename>Wikipedia</sitename>
    <dbname>enwiki</dbname>
    <base>https://en.wikipedia.org/wiki/Main_Page</base>
  </siteinfo>
  <page>
    <title>Music</title>
    <ns>0</ns>
    <id>1</id>
  </page>
  <page>
    <title>Hello World</title>
    <ns>0</ns>
    <id>2</id>
  </page>
  <page>
    <title>C++</title>
    <ns>0</ns>
    <id>3</id>
  </page>
</mediawiki>"#;

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(xml.as_bytes()).unwrap();
        let compressed = gz.finish().unwrap();

        let tmp = std::env::temp_dir().join("wikitools_test_titles.xml.gz");
        std::fs::write(&tmp, &compressed).unwrap();

        let entries = parse_all_titles(&tmp, "en", "wikipedia").unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(entries.len(), 3);

        // Find Music entry
        let music = entries.iter().find(|(h, _)| h == "Music").unwrap();
        assert!(music.1.contains("https://en.wikipedia.org/wiki/Music"));

        // Find Hello World — space → underscore in URL
        let hello = entries.iter().find(|(h, _)| h == "Hello World").unwrap();
        assert!(hello.1.contains("Hello_World"));

        // C++ — special chars percent-encoded
        let cpp = entries.iter().find(|(h, _)| h == "C++").unwrap();
        assert!(cpp.1.contains("C%2B%2B"));
    }

    #[test]
    fn test_parse_all_titles_wiktionary() {
        let xml = r#"<mediawiki xmlns="http://www.mediawiki.org/xml/export-0.11/">
  <page><title>word</title><ns>0</ns><id>1</id></page>
</mediawiki>"#;

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(xml.as_bytes()).unwrap();
        let compressed = gz.finish().unwrap();

        let tmp = std::env::temp_dir().join("wikitools_test_wiktionary.xml.gz");
        std::fs::write(&tmp, &compressed).unwrap();

        let entries = parse_all_titles(&tmp, "en", "wiktionary").unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].1.contains("https://en.wiktionary.org/wiki/word"));
    }

    #[test]
    fn test_title_url_encoding() {
        let entries = parse_all_titles_inner("en", "wikipedia", &["C# (programming language)"]);

        assert_eq!(entries.len(), 1);
        let (_headword, body) = &entries[0];
        // URL must contain percent-encoded space and #
        assert!(body.contains("C%23_(programming_language)"));
        // Headword is human-readable, DSL-escaped
        assert_eq!(entries[0].0, "C\\# \\(programming language\\)");
    }
}
