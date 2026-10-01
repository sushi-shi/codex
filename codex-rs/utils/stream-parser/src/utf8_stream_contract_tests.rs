use pretty_assertions::assert_eq;

use super::Utf8StreamParser;
use crate::StreamTextChunk;
use crate::StreamTextParser;

#[derive(Default)]
struct Recorder {
    seen: String,
}

impl StreamTextParser for Recorder {
    type Extracted = ();

    fn push_str(&mut self, chunk: &str) -> StreamTextChunk<()> {
        self.seen.push_str(chunk);
        StreamTextChunk {
            visible_text: chunk.to_owned(),
            extracted: Vec::new(),
        }
    }

    fn finish(&mut self) -> StreamTextChunk<()> {
        StreamTextChunk::default()
    }
}

#[test]
fn every_chunk_partition_preserves_remainder_and_invalid_chunk_rollback() {
    let text = "Aé中🦀Z";
    let bytes = text.as_bytes();
    for boundaries in 0..1usize << (bytes.len() - 1) {
        let mut parser = Utf8StreamParser::new(Recorder::default());
        let mut rendered = String::new();
        let mut start = 0;
        for end in 1..=bytes.len() {
            if end != bytes.len() && boundaries & (1 << (end - 1)) == 0 {
                continue;
            }
            let before = (parser.pending_utf8.clone(), parser.inner.seen.clone());
            assert!(parser.push_bytes(b"bad\xff").is_err());
            assert_eq!(
                (&parser.pending_utf8, &parser.inner.seen),
                (&before.0, &before.1)
            );

            let output = parser.push_bytes(&bytes[start..end]).unwrap();
            rendered.push_str(&output.visible_text);
            assert!(output.extracted.is_empty());
            let valid = match std::str::from_utf8(&bytes[..end]) {
                Ok(_) => end,
                Err(error) => error.valid_up_to(),
            };
            assert_eq!(parser.pending_utf8.as_slice(), &bytes[valid..end]);
            assert!(parser.pending_utf8.len() <= 3);
            start = end;
        }
        let tail = parser.finish().unwrap();
        assert!(tail.is_empty());
        assert!(parser.pending_utf8.is_empty());
        assert_eq!(
            (
                rendered.as_str(),
                parser.into_inner().unwrap().seen.as_str()
            ),
            (text, text)
        );
    }
}

struct Panics;

impl StreamTextParser for Panics {
    type Extracted = ();

    fn push_str(&mut self, _chunk: &str) -> StreamTextChunk<()> {
        panic!("callback failure");
    }

    fn finish(&mut self) -> StreamTextChunk<()> {
        panic!("callback failure");
    }
}

#[test]
fn callback_panic_is_outside_the_normal_return_state_contract() {
    let mut parser = Utf8StreamParser::new(Panics);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = parser.push_bytes(b"abcdef");
    }));
    assert!(result.is_err());
    assert_eq!(parser.pending_utf8, b"abcdef");
}
