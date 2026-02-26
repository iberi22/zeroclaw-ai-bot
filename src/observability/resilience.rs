use crate::observability::traits::{Observer, ObserverEvent, ObserverMetric};
use crate::resilience;
use std::path::PathBuf;

pub struct ResilienceObserver {
    workspace_dir: PathBuf,
}

impl ResilienceObserver {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

impl Observer for ResilienceObserver {
    fn record_event(&self, event: &ObserverEvent) {
        if let ObserverEvent::Error { component, message } = event {
            resilience::report_task(
                &format!("Observed Error in {}", component),
                &format!("Automated report from ResilienceObserver: {}", message),
                &self.workspace_dir,
            );
        }
    }

    fn record_metric(&self, _metric: &ObserverMetric) {
        // Not used for resilience tasks yet
    }

    fn name(&self) -> &str {
        "resilience"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
