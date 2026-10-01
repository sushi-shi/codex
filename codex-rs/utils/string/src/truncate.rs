//! Utilities for truncating large chunks of output while preserving a prefix
//! and suffix on UTF-8 boundaries.

#![cfg_attr(flux, flux::defs {
    local qualifier Utf8Boundary(offset: int, text: str) {
        flux_core::str::boundary(text, offset)
    }
    local qualifier SuffixBudget(start: int, text: str, budget: int) {
        flux_core::str::byte_len(text) - start <= budget
    }
})]

use flux_attrs::ensures;

const APPROX_BYTES_PER_TOKEN: usize = 4;

/// Truncate a string to `max_bytes` using a character-count marker.
pub fn truncate_middle_chars(s: &str, max_bytes: usize) -> String {
    truncate_with_byte_estimate(s, max_bytes, /*use_tokens*/ false)
}

/// Truncate the middle of a UTF-8 string to at most `max_tokens` approximate
/// tokens, preserving the beginning and the end. Returns the possibly
/// truncated string and `Some(original_token_count)` if truncation occurred;
/// otherwise returns the original string and `None`.
pub fn truncate_middle_with_token_budget(s: &str, max_tokens: usize) -> (String, Option<u64>) {
    if s.is_empty() {
        return (String::new(), None);
    }

    if max_tokens > 0 && s.len() <= approx_bytes_for_tokens(max_tokens) {
        return (s.to_string(), None);
    }

    let truncated = truncate_with_byte_estimate(
        s,
        approx_bytes_for_tokens(max_tokens),
        /*use_tokens*/ true,
    );
    let total_tokens = u64::try_from(approx_token_count(s)).unwrap_or(u64::MAX);

    if truncated == s {
        (truncated, None)
    } else {
        (truncated, Some(total_tokens))
    }
}

fn truncate_with_byte_estimate(s: &str, max_bytes: usize, use_tokens: bool) -> String {
    if s.is_empty() {
        return String::new();
    }

    let total_chars = s.chars().count();

    if max_bytes == 0 {
        return format_truncation_marker(
            use_tokens,
            removed_units(use_tokens, s.len(), total_chars),
        );
    }

    if s.len() <= max_bytes {
        return s.to_string();
    }

    let total_bytes = s.len();
    let (left_budget, right_budget) = split_budget(max_bytes);
    let (removed_chars, left, right) = split_string(s, left_budget, right_budget);
    let marker = format_truncation_marker(
        use_tokens,
        removed_units(
            use_tokens,
            total_bytes.saturating_sub(max_bytes),
            removed_chars,
        ),
    );

    assemble_truncated_output(left, right, &marker)
}

#[ensures(result == (text.len() + 3) / 4)]
#[ensures(text.is_empty() || result > 0)]
pub fn approx_token_count(text: &str) -> usize {
    let len = text.len();
    len.saturating_add(APPROX_BYTES_PER_TOKEN.saturating_sub(1)) / APPROX_BYTES_PER_TOKEN
}

/// Convert the token budget to bytes, saturating at the largest byte budget.
#[cfg_attr(
    flux,
    flux::sig(fn(tokens: usize) -> usize{bytes:
        bytes == (if tokens * 4 <= usize::MAX { tokens * 4 } else { usize::MAX })
        && tokens <= bytes})
)]
pub const fn approx_bytes_for_tokens(tokens: usize) -> usize {
    let bytes = tokens.saturating_mul(APPROX_BYTES_PER_TOKEN);
    #[cfg(flux)]
    flux_core::num::lemmas::saturating_mul_expands(tokens, APPROX_BYTES_PER_TOKEN, bytes);
    bytes
}

#[cfg(flux)]
mod budget_proofs {
    use flux_attrs::requires;

    /// Increasing the token limit never decreases the available byte budget.
    #[requires(smaller <= larger)]
    fn token_budget_is_monotone(smaller: usize, larger: usize) {
        let smaller_bytes = super::approx_bytes_for_tokens(smaller);
        let larger_bytes = super::approx_bytes_for_tokens(larger);
        flux_core::num::lemmas::saturating_mul_monotone(
            smaller,
            larger,
            super::APPROX_BYTES_PER_TOKEN,
            smaller_bytes,
            larger_bytes,
        );
        assert!(smaller_bytes <= larger_bytes);
    }

    /// A longer UTF-8 payload cannot receive a smaller approximate token count.
    #[requires(shorter.len() <= longer.len())]
    fn text_token_count_is_monotone(shorter: &str, longer: &str) {
        let shorter_tokens = super::approx_token_count(shorter);
        let longer_tokens = super::approx_token_count(longer);
        assert!(shorter_tokens <= longer_tokens);
    }
}

pub fn approx_tokens_from_byte_count(bytes: usize) -> u64 {
    let bytes_u64 = bytes as u64;
    bytes_u64.saturating_add((APPROX_BYTES_PER_TOKEN as u64).saturating_sub(1))
        / (APPROX_BYTES_PER_TOKEN as u64)
}

/// Keep a prefix and suffix within their byte budgets, rounding inward to UTF-8
/// boundaries. The prefix wins when budgets overlap. Count removed Unicode
/// scalar values (not bytes or grapheme clusters); retained slices never overlap.
// These candidate loop invariants are checked by Flux and apply only here.
#[cfg_attr(flux, flux::qualifiers(Utf8Boundary, SuffixBudget))]
#[cfg_attr(
    flux,
    flux::sig(fn(s: &str[@text], beginning_bytes: usize, end_bytes: usize) ->
        (usize, &str[#prefix], &str{suffix:
            flux_core::str::byte_len(prefix) <= beginning_bytes
            && flux_core::str::byte_len(suffix) <= end_bytes
            && flux_core::str::byte_len(prefix) + flux_core::str::byte_len(suffix)
                <= flux_core::str::byte_len(text)
            && str_prefix_of(prefix, text) && str_suffix_of(suffix, text)}))
)]
fn split_string(s: &str, beginning_bytes: usize, end_bytes: usize) -> (usize, &str, &str) {
    if s.is_empty() {
        return (0, "", "");
    }

    let len = s.len();
    let tail_start_target = len.saturating_sub(end_bytes);
    let mut prefix_end = 0usize;
    let mut suffix_start = len;
    let mut removed_chars = 0usize;
    let mut suffix_started = false;

    for (idx, ch) in s.char_indices() {
        let char_end = idx + ch.len_utf8();
        if char_end <= beginning_bytes {
            prefix_end = char_end;
            continue;
        }

        if idx >= tail_start_target {
            if !suffix_started {
                suffix_start = idx;
                suffix_started = true;
            }
            continue;
        }

        removed_chars = removed_chars.saturating_add(1);
    }

    if suffix_start < prefix_end {
        suffix_start = prefix_end;
    }

    let before = &s[..prefix_end];
    let after = &s[suffix_start..];

    (removed_chars, before, after)
}

// Preserve the whole budget and give the suffix the extra byte when odd.
#[cfg_attr(
    flux,
    flux::sig(fn(budget: usize[@b]) ->
        (usize[#prefix], usize{suffix:
            prefix + suffix == b && prefix <= suffix && suffix <= prefix + 1}))
)]
fn split_budget(budget: usize) -> (usize, usize) {
    let left = budget / 2;
    (left, budget - left)
}

fn format_truncation_marker(use_tokens: bool, removed_count: u64) -> String {
    if use_tokens {
        format!("…{removed_count} tokens truncated…")
    } else {
        format!("…{removed_count} chars truncated…")
    }
}

fn removed_units(use_tokens: bool, removed_bytes: usize, removed_chars: usize) -> u64 {
    if use_tokens {
        approx_tokens_from_byte_count(removed_bytes)
    } else {
        u64::try_from(removed_chars).unwrap_or(u64::MAX)
    }
}

fn assemble_truncated_output(prefix: &str, suffix: &str, marker: &str) -> String {
    let mut out = String::with_capacity(prefix.len() + marker.len() + suffix.len() + 1);
    out.push_str(prefix);
    out.push_str(marker);
    out.push_str(suffix);
    out
}

#[cfg(test)]
mod tests;
