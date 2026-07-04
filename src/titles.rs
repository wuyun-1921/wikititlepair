use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use rayon::prelude::*;

use crate::error::Result;

/// Map user-facing project name to Wikimedia dump code.
/// "wikipedia" → "wiki", all others pass through as-is.
fn project_code(project: &str) -> &str {
    if project == "wikipedia" { "wiki" } else { project }
}

pub fn titles_listing_url(lang: &str, project: &str) -> String {
    let code = project_code(project);
    format!("https://dumps.wikimedia.org/{}{}/latest/", lang, code)
}

pub fn ensure_titles_dump(
    cache_dir: &Path,
    lang: &str,
    project: &str,
    allow_download: bool,
) -> Result<PathBuf> {
    let code = project_code(project);
    let filename = format!("{}{}-latest-all-titles-in-ns0.gz", lang, code);
    let url = format!(
        "https://dumps.wikimedia.org/{}{}/latest/{}{}-latest-all-titles-in-ns0.gz",
        lang, code, lang, code
    );
    crate::download::ensure_dump(cache_dir, &filename, &url, allow_download)
}

/// Extract all article titles from an all-titles-in-ns0 dump.
/// The dump is a gzipped TSV with header `page_title`, one title per line.
/// Titles use underscores for spaces.
/// Returns (display_name, url_path) pairs — no DSL escaping needed for MDX output.
pub fn parse_all_titles(path: &Path, _lang: &str, _project: &str) -> Result<Vec<(String, String)>> {
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut contents = String::new();
    decoder.read_to_string(&mut contents)?;

    let titles: Vec<&str> = contents
        .lines()
        .skip(1)  // skip "page_title" header
        .filter(|l| !l.is_empty())
        .collect();

    let mut entries: Vec<(String, String)> = titles
        .par_iter()
        .map(|title| {
            let display = title.replace('_', " ");
            let url_encoded = url_encode_title(&display);
            let path = format!("/wiki/{}", url_encoded);
            (display, path)
        })
        .collect();

    entries.par_sort();
    entries.dedup();
    Ok(entries)
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

/// Internal helper for testing URL encoding without full round-trip.
#[cfg(test)]
fn parse_all_titles_inner(_lang: &str, _project: &str, titles: &[&str]) -> Vec<(String, String)> {
    titles
        .iter()
        .map(|title| {
            let url_encoded = url_encode_title(title);
            let path = format!("/wiki/{}", url_encoded);
            (title.to_string(), path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_all_titles_basic() {
        // Build a minimal gzipped all-titles TSV dump with 3 articles
        // Format: header line "page_title", then one title per line (underscore-separated)
        let tsv = "page_title\nMusic\nHello_World\nC++\n";

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(tsv.as_bytes()).unwrap();
        let compressed = gz.finish().unwrap();

        let tmp = std::env::temp_dir().join("wikitools_test_titles.xml.gz");
        std::fs::write(&tmp, &compressed).unwrap();

        let entries = parse_all_titles(&tmp, "en", "wikipedia").unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(entries.len(), 3);

        // Find Music entry
        let music = entries.iter().find(|(h, _)| h == "Music").unwrap();
        assert_eq!(music.1, "/wiki/Music");

        // Find Hello_World — underscore → space for display, underscore for URL
        let hello = entries.iter().find(|(h, _)| h == "Hello World").unwrap();
        assert!(hello.1.contains("Hello_World"));

        // C++ — special chars percent-encoded
        let cpp = entries.iter().find(|(h, _)| h == "C++").unwrap();
        assert!(cpp.1.contains("C%2B%2B"));
    }

    #[test]
    fn test_parse_all_titles_wiktionary() {
        let tsv = "page_title\nword\n";

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(tsv.as_bytes()).unwrap();
        let compressed = gz.finish().unwrap();

        let tmp = std::env::temp_dir().join("wikitools_test_wiktionary.xml.gz");
        std::fs::write(&tmp, &compressed).unwrap();

        let entries = parse_all_titles(&tmp, "en", "wiktionary").unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1, "/wiki/word");
    }

    #[test]
    fn test_title_url_encoding() {
        let entries = parse_all_titles_inner("en", "wikipedia", &["C# (programming language)"]);

        assert_eq!(entries.len(), 1);
        let (_headword, body) = &entries[0];
        // URL must contain percent-encoded space and #
        assert!(body.contains("C%23_(programming_language)"));
        // Headword is plain display text
        assert_eq!(entries[0].0, "C# (programming language)");
    }

    #[test]
    fn test_parse_all_titles_empty_lines_skipped() {
        let tsv = "page_title\nOne\n\nTwo\n\n";

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(tsv.as_bytes()).unwrap();
        let compressed = gz.finish().unwrap();

        let tmp = std::env::temp_dir().join("wikitools_test_empty.xml.gz");
        std::fs::write(&tmp, &compressed).unwrap();

        let entries = parse_all_titles(&tmp, "en", "wikipedia").unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(entries.len(), 2);
    }
}
