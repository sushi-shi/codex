use std::collections::BTreeMap;

use flux_attrs::ensures;
use flux_attrs::refined;

use crate::ExecServerError;

const MAX_REORDER_DISTANCE: u32 = 64;
const MAX_PENDING_BYTES: usize = 1024 * 1024;

/// Reorders relay records before they reach Noise's implicit receive nonce.
/// The window is bounded, and each sequence number is released at most once.
#[derive(Default)]
#[refined]
#[cfg_attr(flux, flux::invariant(pending_bytes <= MAX_PENDING_BYTES))]
#[cfg_attr(flux, flux::invariant(pending_bytes == pending.total))]
pub(crate) struct OrderedCiphertextFrames {
    next_seq: u32,
    pending: BTreeMap<u32, Vec<u8>>,
    pending_bytes: usize,
}

impl OrderedCiphertextFrames {
    /// Accept one relay record and return the newly contiguous ciphertext run.
    ///
    /// Returns nothing for duplicates or while a gap remains. Closing a gap also
    /// releases any buffered records that now follow it contiguously.
    #[ensures(self.next_seq >= old(self.next_seq))]
    #[ensures(if seq < old(self.next_seq) {
        result.is_ok() && self.next_seq == old(self.next_seq)
            && self.pending_bytes == old(self.pending_bytes)
            && self.pending == old(self.pending)
    } else { true })]
    pub(crate) fn push(
        &mut self,
        seq: u32,
        payload: Vec<u8>,
    ) -> Result<Vec<Vec<u8>>, ExecServerError> {
        // Keep the first ciphertext for a sequence. Later copies are duplicates.
        if seq < self.next_seq || self.pending.contains_key(&seq) {
            return Ok(Vec::new());
        }
        if seq > self.next_seq {
            // Bound both the sequence gap and buffered bytes.
            if seq - self.next_seq > MAX_REORDER_DISTANCE {
                return Err(ExecServerError::Protocol(
                    "Noise relay ciphertext exceeds reorder window".to_string(),
                ));
            }
            let pending_bytes = self.pending_bytes + payload.len();
            if pending_bytes > MAX_PENDING_BYTES {
                return Err(ExecServerError::Protocol(
                    "Noise relay pending ciphertext buffer is full".to_string(),
                ));
            }
            self.pending.insert(seq, payload);
            self.pending_bytes = pending_bytes;
            return Ok(Vec::new());
        }

        // Release the expected record and anything now contiguous behind it.
        let mut ready = vec![payload];
        self.advance()?;
        while let Some(payload) = self.pending.remove(&self.next_seq) {
            self.pending_bytes -= payload.len();
            ready.push(payload);
            self.advance()?;
        }
        Ok(ready)
    }

    #[ensures(result.is_ok() == (old(self.next_seq) < u32::MAX))]
    #[ensures(self.next_seq == if result.is_ok() {
        old(self.next_seq) + 1
    } else { old(self.next_seq) })]
    #[ensures(self.pending_bytes == old(self.pending_bytes))]
    #[ensures(self.pending == old(self.pending))]
    fn advance(&mut self) -> Result<(), ExecServerError> {
        self.next_seq = self.next_seq.checked_add(1).ok_or_else(|| {
            ExecServerError::Protocol("Noise relay sequence number exhausted".to_string())
        })?;
        Ok(())
    }
}

#[cfg(flux)]
mod sequence_proofs {
    use super::OrderedCiphertextFrames;
    use flux_attrs::ensures;
    use flux_attrs::requires;

    /// The last successful advance cannot be followed by a wrapped sequence.
    #[requires(frames.next_seq == u32::MAX - 1)]
    #[ensures(frames.next_seq == u32::MAX)]
    #[ensures(frames.pending_bytes == old(frames.pending_bytes))]
    fn exhaustion_preserves_sequence(frames: &mut OrderedCiphertextFrames) {
        assert!(frames.advance().is_ok());
        assert!(frames.advance().is_err());
        assert!(frames.next_seq == u32::MAX);
    }

    /// Replayed records leave sequence and buffer accounting unchanged.
    /// The map model tracks keys and payload lengths, not ciphertext contents.
    #[requires(seq < frames.next_seq)]
    #[ensures(frames.next_seq == old(frames.next_seq))]
    #[ensures(frames.pending_bytes == old(frames.pending_bytes))]
    #[ensures(frames.pending == old(frames.pending))]
    fn stale_frame_preserves_accounting(
        frames: &mut OrderedCiphertextFrames,
        seq: u32,
        payload: Vec<u8>,
    ) {
        assert!(frames.push(seq, payload).is_ok());
    }
}

#[cfg(test)]
#[path = "ordered_ciphertext_tests.rs"]
mod tests;
