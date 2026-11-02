use std::fmt;
use std::io;

#[derive(Debug)]
pub(crate) struct CaptureFlowClosed {
    operation: &'static str,
}

impl CaptureFlowClosed {
    pub(crate) fn new(operation: &'static str) -> Self {
        Self { operation }
    }
}

impl fmt::Display for CaptureFlowClosed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Capture TCP flow ended: {}",
            self.operation,
        )
    }
}

impl std::error::Error for CaptureFlowClosed {}

pub(crate) fn is_capture_flow_closed(
    error: &anyhow::Error,
) -> bool {
    error.chain().any(|cause| {
        if cause.is::<CaptureFlowClosed>() {
            return true;
        }

        cause
            .downcast_ref::<io::Error>()
            .and_then(|error| error.get_ref())
            .is_some_and(|inner| inner.is::<CaptureFlowClosed>())
    })
}
