use rak_rs::{
    connection::queue::FragmentQueue,
    protocol::{
        frame::{FragmentMeta, Frame},
        reliability::Reliability,
    },
};

// Fixes issue: https://github.com/NetrexMC/RakNet/issues/55
#[test]
fn test_proper_reordering() {
    let mut queue = FragmentQueue::new();

    const SLICE_ONE: &[u8] = &[1, 2, 3, 4, 5];
    const SLICE_TWO: &[u8] = &[6, 7, 8, 9, 10];
    const SLICE_THREE: &[u8] = &[11, 12, 13, 14, 15];

    // push slice 2 first, then slice 1, then slice 3
    queue
        .insert(
            Frame::new(Reliability::ReliableOrd, Some(SLICE_TWO))
                .with_meta(FragmentMeta::new(3, 11, 1)),
        )
        .unwrap();

    queue
        .insert(
            Frame::new(Reliability::ReliableOrd, Some(SLICE_ONE))
                .with_meta(FragmentMeta::new(3, 11, 0)),
        )
        .unwrap();

    queue
        .insert(
            Frame::new(Reliability::ReliableOrd, Some(SLICE_THREE))
                .with_meta(FragmentMeta::new(3, 11, 2)),
        )
        .unwrap();

    // collect the fragments
    let res = queue.collect(11);

    assert_eq!(
        res.unwrap(),
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    );
}
