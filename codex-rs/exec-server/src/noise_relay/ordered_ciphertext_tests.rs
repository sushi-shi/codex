use pretty_assertions::assert_eq;

use super::MAX_PENDING_BYTES;
use super::OrderedCiphertextFrames;

#[test]
fn releases_ciphertexts_only_in_nonce_order() {
    let mut frames = OrderedCiphertextFrames::default();

    assert_eq!(
        frames.push(/*seq*/ 1, b"second".to_vec()).unwrap(),
        Vec::<Vec<u8>>::new()
    );
    assert_eq!(
        frames.push(/*seq*/ 0, b"first".to_vec()).unwrap(),
        vec![b"first".to_vec(), b"second".to_vec()]
    );
}

#[test]
fn ignores_duplicate_ciphertexts_without_replacing_buffered_record() {
    let mut frames = OrderedCiphertextFrames::default();

    assert_eq!(
        frames.push(/*seq*/ 1, b"first copy".to_vec()).unwrap(),
        Vec::<Vec<u8>>::new()
    );
    assert_eq!(
        frames.push(/*seq*/ 1, b"replacement".to_vec()).unwrap(),
        Vec::<Vec<u8>>::new()
    );
    assert_eq!(
        frames.push(/*seq*/ 0, b"zero".to_vec()).unwrap(),
        vec![b"zero".to_vec(), b"first copy".to_vec()]
    );
    assert_eq!(
        frames.push(/*seq*/ 0, b"duplicate".to_vec()).unwrap(),
        Vec::<Vec<u8>>::new()
    );
}

#[test]
fn rejects_unbounded_reordering() {
    let mut frames = OrderedCiphertextFrames::default();

    assert!(frames.push(/*seq*/ 65, Vec::new()).is_err());
    assert!(
        frames
            .push(/*seq*/ 1, vec![0; MAX_PENDING_BYTES + 1])
            .is_err()
    );
}

#[test]
fn exhausted_sequence_rejects_repeated_advances_without_changing_buffer() {
    let pending = std::collections::BTreeMap::from([(u32::MAX, b"last".to_vec())]);
    let mut frames = OrderedCiphertextFrames {
        next_seq: u32::MAX - 1,
        pending: pending.clone(),
        pending_bytes: 4,
    };

    assert!(frames.advance().is_ok());
    for _ in 0..2 {
        assert!(frames.advance().is_err());
        assert_eq!(
            (frames.next_seq, &frames.pending, frames.pending_bytes),
            (u32::MAX, &pending, 4)
        );
    }
}

#[test]
fn buffered_bytes_match_payloads_through_duplicates_and_gap_closing() {
    let mut frames = OrderedCiphertextFrames::default();
    for batch in 0..32u32 {
        let first = batch * 9;
        let expected: Vec<Vec<u8>> = (first..first + 9)
            .map(|seq| vec![seq as u8; (seq % 3) as usize])
            .collect();
        for offset in (1..9).rev() {
            let seq = first + offset as u32;
            assert!(
                frames
                    .push(seq, expected[offset].clone())
                    .unwrap()
                    .is_empty()
            );
            assert!(frames.push(seq, vec![255; 10]).unwrap().is_empty());
            assert_eq!(
                (frames.pending.len(), frames.pending_bytes),
                (9 - offset, expected[offset..].iter().map(Vec::len).sum())
            );
        }
        assert_eq!(frames.push(first, expected[0].clone()).unwrap(), expected);
        assert_eq!(
            (frames.next_seq, &frames.pending, frames.pending_bytes),
            (first + 9, &std::collections::BTreeMap::new(), 0)
        );
    }
}

#[test]
fn full_buffer_rejects_an_extra_byte_without_changing_accounting() {
    let mut frames = OrderedCiphertextFrames::default();
    let payload = vec![0; MAX_PENDING_BYTES];
    assert!(frames.push(/*seq*/ 1, payload.clone()).unwrap().is_empty());
    assert!(frames.push(/*seq*/ 2, vec![1]).is_err());
    assert_eq!(
        (frames.next_seq, &frames.pending, frames.pending_bytes),
        (
            0,
            &std::collections::BTreeMap::from([(1, payload.clone())]),
            MAX_PENDING_BYTES
        )
    );
    assert_eq!(
        frames.push(/*seq*/ 0, Vec::new()).unwrap(),
        vec![Vec::new(), payload]
    );
    assert_eq!((frames.pending.len(), frames.pending_bytes), (0, 0));
}
