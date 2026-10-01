use super::approx_bytes_for_tokens;
use super::split_budget;
use super::split_string;
use super::truncate_middle_chars;
use super::truncate_middle_with_token_budget;
use pretty_assertions::assert_eq;

#[test]
fn token_byte_budget_saturates_and_never_decreases() {
    let last_exact = usize::MAX / 4;
    let cases = [
        (0, 0),
        (1, 4),
        (last_exact, usize::MAX - 3),
        (last_exact + 1, usize::MAX),
        (usize::MAX, usize::MAX),
    ];
    for (tokens, expected_bytes) in cases {
        assert_eq!(approx_bytes_for_tokens(tokens), expected_bytes);
    }
    for pair in cases.windows(2) {
        assert!(approx_bytes_for_tokens(pair[0].0) <= approx_bytes_for_tokens(pair[1].0));
    }
}

#[test]
fn split_string_works() {
    assert_eq!(
        split_string(
            "hello world",
            /*beginning_bytes*/ 5,
            /*end_bytes*/ 5
        ),
        (1, "hello", "world")
    );
    assert_eq!(
        split_string("abc", /*beginning_bytes*/ 0, /*end_bytes*/ 0),
        (3, "", "")
    );
}

#[test]
fn split_string_handles_empty_string() {
    assert_eq!(
        split_string("", /*beginning_bytes*/ 4, /*end_bytes*/ 4),
        (0, "", "")
    );
}

#[test]
fn split_string_only_keeps_prefix_when_tail_budget_is_zero() {
    assert_eq!(
        split_string("abcdef", /*beginning_bytes*/ 3, /*end_bytes*/ 0),
        (3, "abc", "")
    );
}

#[test]
fn split_string_only_keeps_suffix_when_prefix_budget_is_zero() {
    assert_eq!(
        split_string("abcdef", /*beginning_bytes*/ 0, /*end_bytes*/ 3),
        (3, "", "def")
    );
}

#[test]
fn split_string_handles_overlapping_budgets_without_removal() {
    assert_eq!(
        split_string("abcdef", /*beginning_bytes*/ 4, /*end_bytes*/ 4),
        (0, "abcd", "ef")
    );
}

#[test]
fn split_string_respects_utf8_boundaries() {
    assert_eq!(
        split_string("😀abc😀", /*beginning_bytes*/ 5, /*end_bytes*/ 5),
        (1, "😀a", "c😀")
    );

    assert_eq!(
        split_string(
            "😀😀😀😀😀",
            /*beginning_bytes*/ 1,
            /*end_bytes*/ 1
        ),
        (5, "", "")
    );
    assert_eq!(
        split_string(
            "😀😀😀😀😀",
            /*beginning_bytes*/ 7,
            /*end_bytes*/ 7
        ),
        (3, "😀", "😀")
    );
    assert_eq!(
        split_string(
            "😀😀😀😀😀",
            /*beginning_bytes*/ 8,
            /*end_bytes*/ 8
        ),
        (1, "😀😀", "😀😀")
    );
}

#[test]
fn split_string_matches_boundary_oracle() {
    // Include all UTF-8 widths, combining marks, joined emoji, and embedded NUL.
    let texts = ["", "abc", "é中😀", "a\u{301}b", "👩\u{200d}💻", "\0é\0"];
    for text in texts {
        let boundaries: Vec<_> = (0..=text.len())
            .filter(|&offset| text.is_char_boundary(offset))
            .collect();
        let budgets: Vec<_> = (0..=text.len() + 4)
            .chain([usize::MAX - 1, usize::MAX])
            .collect();
        for &beginning_bytes in &budgets {
            for &end_bytes in &budgets {
                let prefix_end = boundaries
                    .iter()
                    .copied()
                    .filter(|&offset| offset <= beginning_bytes)
                    .max()
                    .unwrap();
                let suffix_start = boundaries
                    .iter()
                    .copied()
                    .find(|&offset| offset >= prefix_end && text.len() - offset <= end_bytes)
                    .unwrap();
                let expected = (
                    text[prefix_end..suffix_start].chars().count(),
                    &text[..prefix_end],
                    &text[suffix_start..],
                );
                assert_eq!(
                    split_string(text, beginning_bytes, end_bytes),
                    expected,
                    "text={text:?}, beginning_bytes={beginning_bytes}, end_bytes={end_bytes}"
                );
            }
        }
    }
}

#[test]
fn truncate_with_token_budget_returns_original_when_under_limit() {
    let s = "short output";
    let limit = 100;
    let (out, original) = truncate_middle_with_token_budget(s, limit);
    assert_eq!(out, s);
    assert_eq!(original, None);
}

#[test]
fn truncate_with_token_budget_reports_truncation_at_zero_limit() {
    let s = "abcdef";
    let (out, original) = truncate_middle_with_token_budget(s, /*max_tokens*/ 0);
    assert_eq!(out, "…2 tokens truncated…");
    assert_eq!(original, Some(2));
}

#[test]
fn truncate_middle_tokens_handles_utf8_content() {
    let s = "😀😀😀😀😀😀😀😀😀😀\nsecond line with text\n";
    let (out, tokens) = truncate_middle_with_token_budget(s, /*max_tokens*/ 8);
    assert_eq!(out, "😀😀😀😀…8 tokens truncated… line with text\n");
    assert_eq!(tokens, Some(16));
}

#[test]
fn truncate_middle_bytes_handles_utf8_content() {
    let s = "😀😀😀😀😀😀😀😀😀😀\nsecond line with text\n";
    let out = truncate_middle_chars(s, /*max_bytes*/ 20);
    assert_eq!(out, "😀😀…21 chars truncated…with text\n");
}

#[test]
fn split_budget_preserves_and_balances_the_budget() {
    for budget in (0..513).chain([usize::MAX - 1, usize::MAX]) {
        let (prefix, suffix) = split_budget(budget);
        assert_eq!(prefix.checked_add(suffix), Some(budget));
        assert!(prefix <= suffix);
        assert!(suffix - prefix <= 1);
    }
}
