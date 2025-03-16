#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrefetchState {
    NoPrefetch,
    PrefetchFailed,
    PrefetchSuccessful,
}
