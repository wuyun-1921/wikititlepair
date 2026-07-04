/// Escape special characters in DSL headwords and cross-references.
pub fn escape_dsl(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 16);
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '{' => result.push_str("\\{"),
            '}' => result.push_str("\\}"),
            '[' => result.push_str("\\["),
            ']' => result.push_str("\\]"),
            '#' => result.push_str("\\#"),
            '@' => result.push_str("\\@"),
            '<' => result.push_str("\\<"),
            '>' => result.push_str("\\>"),
            '~' => result.push_str("\\~"),
            '^' => result.push_str("\\^"),
            _ => result.push(ch),
        }
    }
    result
}

pub fn unquote(s: &str) -> String {
    let s = s.trim();
    if !s.starts_with('\'') || !s.ends_with('\'') {
        return s.to_string();
    }
    let inner = &s[1..s.len() - 1];
    let bytes = inner.as_bytes();
    let len = bytes.len();
    let mut result = Vec::with_capacity(len);
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\\' && i + 1 < len {
            match bytes[i + 1] {
                b'\'' => { result.push(b'\''); i += 2; }
                b'\\' => { result.push(b'\\'); i += 2; }
                b'n' => { result.push(b'\n'); i += 2; }
                b'r' => { result.push(b'\r'); i += 2; }
                b't' => { result.push(b'\t'); i += 2; }
                b'"' => { result.push(b'"'); i += 2; }
                _ => { result.push(bytes[i]); i += 1; }
            }
        } else if bytes[i] == b'\'' && i + 1 < len && bytes[i + 1] == b'\'' {
            result.push(b'\'');
            i += 2;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// MediaWiki namespace canonical names (English).
pub static NON_ARTICLE_PREFIXES: &[&str] = &[
    "Category", "Template", "Wikipedia", "Portal", "Help",
    "Module", "WikiProject", "User", "File", "Image",
    "MediaWiki", "TimedText", "Draft", "Media", "Special",
    "Talk", "WP",
];

/// Returns true if the title is a non-article namespace page.
pub fn is_non_article(title: &str) -> bool {
    for prefix in NON_ARTICLE_PREFIXES {
        let needle = [prefix, ":"].concat();
        if title.starts_with(&needle) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_dsl_parens() {
        assert_eq!(escape_dsl("Music (2021)"), "Music \\(2021\\)");
        assert_eq!(escape_dsl("C#"), "C\\#");
        assert_eq!(escape_dsl("A < B"), "A \\< B");
        assert_eq!(escape_dsl("x ~ y"), "x \\~ y");
        assert_eq!(escape_dsl("x^2"), "x\\^2");
        assert_eq!(escape_dsl("path\\to"), "path\\\\to");
        assert_eq!(escape_dsl("no escape"), "no escape");
        assert_eq!(escape_dsl("音乐"), "音乐");
    }

    #[test]
    fn test_is_non_article() {
        assert!(is_non_article("Category:Music"));
        assert!(is_non_article("Template:Infobox"));
        assert!(is_non_article("Wikipedia:About"));
        assert!(is_non_article("Help:Contents"));
        assert!(is_non_article("Module:Math"));
        assert!(is_non_article("User:Test"));
        assert!(!is_non_article("Music"));
        assert!(!is_non_article("Doraemon: Story"));
        assert!(!is_non_article("Star Wars: Episode IV"));
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote("'hello'"), "hello");
        assert_eq!(unquote("'it\\'s'"), "it's");
        assert_eq!(unquote("'back\\\\slash'"), "back\\slash");
        assert_eq!(unquote("'quote\\\"test\\\"'"), "quote\"test\"");
        assert_eq!(unquote("'new\\nline'"), "new\nline");
    }
}
