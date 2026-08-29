mod generic;
mod go;
mod java;
mod node;
mod python;
mod rust;

pub use generic::GenericDetector;
pub use go::GoDetector;
pub use java::JavaDetector;
pub use node::NodeDetector;
pub use python::PythonDetector;
pub use rust::RustDetector;

use std::sync::Arc;

use crate::detector::ProjectDetector;

pub fn all_detectors() -> Vec<Arc<dyn ProjectDetector>> {
    vec![
        Arc::new(RustDetector),
        Arc::new(PythonDetector),
        Arc::new(NodeDetector),
        Arc::new(GoDetector),
        Arc::new(JavaDetector),
        Arc::new(GenericDetector),
    ]
}
