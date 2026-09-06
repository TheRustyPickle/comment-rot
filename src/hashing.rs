/// Collapses all whitespace runs to a single space and trims the ends, so
/// pure reformatting/reindentation never changes a hash.
pub fn normalize_for_hash(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = true; // trims leading whitespace

    for c in s.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(c);
            last_was_space = false;
        }
    }

    if out.ends_with(' ') {
        out.pop();
    }

    out
}

pub fn hash_str(s: &str) -> String {
    blake3::hash(normalize_for_hash(s).as_bytes())
        .to_hex()
        .to_string()
}

/// Short, stable anchor from content for free comments
pub fn short_anchor(s: &str) -> String {
    hash_str(s)[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_internal_whitespace_runs() {
        assert_eq!(normalize_for_hash("a   b\tc"), "a b c");
    }

    #[test]
    fn trims_leading_and_trailing_whitespace() {
        assert_eq!(normalize_for_hash("  \n  hello  \n  "), "hello");
    }

    #[test]
    fn reformatting_does_not_change_the_hash() {
        let a = "fn f(a: i32) -> i32 {\n    a + 1\n}";
        let b = "fn f(a: i32) -> i32 {\n        a\n        +\n        1\n}";
        assert_eq!(hash_str(a), hash_str(b));
    }

    #[test]
    fn a_real_change_does_change_the_hash() {
        assert_ne!(hash_str("a + 1"), hash_str("a + 2"));
    }

    #[test]
    fn short_anchor_is_a_12_char_prefix_of_the_full_hash() {
        let full = hash_str("some code");
        assert_eq!(short_anchor("some code"), full[..12]);
    }
}
