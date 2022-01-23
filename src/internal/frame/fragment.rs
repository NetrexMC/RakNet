/// The information for the given fragment.
/// This is used to determine how to reassemble the frame.
#[derive(Debug, Clone)]
pub struct FragmentMeta {
    /// The total number of fragments in this frame.
    pub(crate) size: u32,
    /// The identifier for this fragment.
    /// This is used similar to a ordered channel, where the trailing buffer
    /// will be stored with this identifier.
    pub(crate) id: u16,
    /// The index of the fragment.
    /// This is the arrangement of the fragments in the frame.
    pub(crate) index: u32,
}
