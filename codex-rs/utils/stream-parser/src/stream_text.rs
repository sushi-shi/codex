use flux_attrs::ensures;
use flux_attrs::may_panic;
use flux_attrs::refined;

/// Incremental parser result for one pushed chunk (or final flush).
#[derive(Debug, Clone, PartialEq, Eq)]
#[refined(visible_text, extracted)]
pub struct StreamTextChunk<T> {
    /// Text safe to render immediately.
    pub visible_text: String,
    /// Hidden payloads extracted from the chunk.
    pub extracted: Vec<T>,
}

impl<T> Default for StreamTextChunk<T> {
    #[ensures(result.visible_text == "" && result.extracted.is_empty())]
    fn default() -> Self {
        Self {
            visible_text: String::new(),
            extracted: Vec::new(),
        }
    }
}

impl<T> StreamTextChunk<T> {
    /// Returns true when no visible text or extracted payloads were produced.
    #[ensures(result == (self.visible_text == "" && self.extracted.is_empty()))]
    pub fn is_empty(&self) -> bool {
        self.visible_text.is_empty() && self.extracted.is_empty()
    }
}

/// Trait for parsers that consume streamed text and emit visible text plus extracted payloads.
pub trait StreamTextParser {
    /// Payload extracted by this parser (for example a citation body).
    type Extracted;

    /// Feed a new text chunk.
    #[may_panic]
    fn push_str(&mut self, chunk: &str) -> StreamTextChunk<Self::Extracted>;

    /// Flush any buffered state at end-of-stream (or end-of-item).
    #[may_panic]
    fn finish(&mut self) -> StreamTextChunk<Self::Extracted>;
}
