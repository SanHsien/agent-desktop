use crate::{AdapterError, adapter::NativeHandle};

pub(crate) enum ResolveAttemptOutcome<H = NativeHandle> {
    Resolved(H),
    Failed(AdapterError),
    DeadlinePassed,
}
