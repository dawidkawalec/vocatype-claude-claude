pub mod error;
pub mod performance;

pub use error::{AppError, AudioResult};
pub use performance::{PerformanceMonitor, PerformanceStats, ComponentStats, Benchmark, get_performance_monitor};
