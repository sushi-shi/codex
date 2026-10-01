use super::ExtractedInlineTag;
use super::InlineHiddenTagParser;
use super::InlineTagSpec;
use crate::StreamTextChunk;
use crate::StreamTextParser;

#[test]
fn finish_is_idempotent_across_chunk_partitions_and_incomplete_tags() {
    for (text, visible, extracted) in [
        ("α<a>β</a>γ", "αγ", Some("β")),
        ("α<a>β", "α", Some("β")),
        ("α<a", "α<a", None),
        ("<a>", "", Some("")),
        ("", "", None),
    ] {
        let mut boundaries: Vec<_> = text.char_indices().map(|(offset, _)| offset).collect();
        boundaries.push(text.len());
        let split_count = boundaries.len().saturating_sub(2);
        for splits in 0..1usize << split_count {
            let mut parser = InlineHiddenTagParser::new(vec![InlineTagSpec {
                tag: (),
                open: "<a>",
                close: "</a>",
            }]);
            let mut all = StreamTextChunk::default();
            let mut start = 0;
            for (index, end) in boundaries.iter().copied().enumerate().skip(1) {
                if end != text.len() && splits & (1 << (index - 1)) == 0 {
                    continue;
                }
                let next = parser.push_str(&text[start..end]);
                all.visible_text.push_str(&next.visible_text);
                all.extracted.extend(next.extracted);
                start = end;
            }
            let tail = parser.finish();
            all.visible_text.push_str(&tail.visible_text);
            all.extracted.extend(tail.extracted);
            assert_eq!(
                all,
                StreamTextChunk {
                    visible_text: visible.to_owned(),
                    extracted: extracted
                        .into_iter()
                        .map(|content| ExtractedInlineTag {
                            tag: (),
                            content: content.to_owned(),
                        })
                        .collect(),
                }
            );
            assert_eq!(
                (&parser.pending, parser.active.is_none()),
                (&String::new(), true)
            );
            assert_eq!(parser.finish(), StreamTextChunk::default());
        }
    }
}
