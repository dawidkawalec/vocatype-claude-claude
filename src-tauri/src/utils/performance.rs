use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Performance monitoring and benchmarking utilities
/// Tracks latency targets as per PRD requirements
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    /// Audio capture latency measurements (target: <10ms)
    audio_latencies: Arc<Mutex<VecDeque<Duration>>>,
    
    /// VAD processing times (target: <5ms)
    vad_latencies: Arc<Mutex<VecDeque<Duration>>>,
    
    /// STT processing times (target: <200ms)
    stt_latencies: Arc<Mutex<VecDeque<Duration>>>,
    
    /// AI processing times (target: <300ms first token)
    ai_latencies: Arc<Mutex<VecDeque<Duration>>>,
    
    /// Overall system latency (target: <50ms)
    system_latencies: Arc<Mutex<VecDeque<Duration>>>,
    
    /// Window size for moving averages
    window_size: usize,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new(window_size: usize) -> Self {
        info!("📊 Initializing performance monitor (window: {} samples)", window_size);
        
        Self {
            audio_latencies: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            vad_latencies: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            stt_latencies: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            ai_latencies: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            system_latencies: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            window_size,
        }
    }
    
    /// Record audio capture latency
    pub fn record_audio_latency(&self, latency: Duration) {
        self.record_latency(&self.audio_latencies, latency, "Audio", Duration::from_millis(10));
    }
    
    /// Record VAD processing latency
    pub fn record_vad_latency(&self, latency: Duration) {
        self.record_latency(&self.vad_latencies, latency, "VAD", Duration::from_millis(5));
    }
    
    /// Record STT processing latency
    pub fn record_stt_latency(&self, latency: Duration) {
        self.record_latency(&self.stt_latencies, latency, "STT", Duration::from_millis(200));
    }
    
    /// Record AI processing latency
    pub fn record_ai_latency(&self, latency: Duration) {
        self.record_latency(&self.ai_latencies, latency, "AI", Duration::from_millis(300));
    }
    
    /// Record overall system latency
    pub fn record_system_latency(&self, latency: Duration) {
        self.record_latency(&self.system_latencies, latency, "System", Duration::from_millis(50));
    }
    
    /// Get comprehensive performance stats
    pub fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            audio: self.get_component_stats(&self.audio_latencies, Duration::from_millis(10)),
            vad: self.get_component_stats(&self.vad_latencies, Duration::from_millis(5)),
            stt: self.get_component_stats(&self.stt_latencies, Duration::from_millis(200)),
            ai: self.get_component_stats(&self.ai_latencies, Duration::from_millis(300)),
            system: self.get_component_stats(&self.system_latencies, Duration::from_millis(50)),
        }
    }
    
    /// Internal function to record latency with target checking
    fn record_latency(
        &self, 
        latencies: &Arc<Mutex<VecDeque<Duration>>>, 
        latency: Duration, 
        component: &str,
        target: Duration
    ) {
        if let Ok(mut queue) = latencies.lock() {
            // Add new measurement
            queue.push_back(latency);
            
            // Remove old measurements if exceeding window size
            if queue.len() > self.window_size {
                queue.pop_front();
            }
            
            // Performance warning if exceeding target
            if latency > target {
                warn!("⚠️  {} latency exceeded target: {:?} > {:?}", 
                      component, latency, target);
            } else {
                debug!("⚡ {} latency within target: {:?}", component, latency);
            }
        }
    }
    
    /// Get statistics for a specific component
    fn get_component_stats(&self, latencies: &Arc<Mutex<VecDeque<Duration>>>, target: Duration) -> ComponentStats {
        if let Ok(queue) = latencies.lock() {
            if queue.is_empty() {
                return ComponentStats::default();
            }
            
            let samples: Vec<u64> = queue.iter().map(|d| d.as_millis() as u64).collect();
            let count = samples.len() as u64;
            
            let avg = samples.iter().sum::<u64>() / count;
            let min = *samples.iter().min().unwrap_or(&0);
            let max = *samples.iter().max().unwrap_or(&0);
            
            // Calculate percentiles
            let mut sorted_samples = samples.clone();
            sorted_samples.sort_unstable();
            
            let p50 = percentile(&sorted_samples, 50.0);
            let p95 = percentile(&sorted_samples, 95.0);
            let p99 = percentile(&sorted_samples, 99.0);
            
            // Check target compliance
            let target_ms = target.as_millis() as u64;
            let violations = samples.iter().filter(|&&latency| latency > target_ms).count() as u64;
            let compliance_rate = ((count - violations) as f64 / count as f64) * 100.0;
            
            ComponentStats {
                sample_count: count,
                avg_ms: avg,
                min_ms: min,
                max_ms: max,
                p50_ms: p50,
                p95_ms: p95,
                p99_ms: p99,
                target_ms,
                compliance_rate,
                violations,
            }
        } else {
            ComponentStats::default()
        }
    }
}

/// Calculate percentile from sorted samples
fn percentile(sorted_samples: &[u64], p: f64) -> u64 {
    if sorted_samples.is_empty() {
        return 0;
    }
    
    let index = (p / 100.0 * (sorted_samples.len() - 1) as f64).round() as usize;
    sorted_samples[index.min(sorted_samples.len() - 1)]
}

/// Performance statistics for all components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub audio: ComponentStats,
    pub vad: ComponentStats,
    pub stt: ComponentStats,
    pub ai: ComponentStats,
    pub system: ComponentStats,
}

/// Performance statistics for individual component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStats {
    /// Number of samples
    pub sample_count: u64,
    
    /// Average latency (ms)
    pub avg_ms: u64,
    
    /// Minimum latency (ms)
    pub min_ms: u64,
    
    /// Maximum latency (ms)
    pub max_ms: u64,
    
    /// 50th percentile (median)
    pub p50_ms: u64,
    
    /// 95th percentile
    pub p95_ms: u64,
    
    /// 99th percentile
    pub p99_ms: u64,
    
    /// Target latency (ms)
    pub target_ms: u64,
    
    /// Compliance rate (%)
    pub compliance_rate: f64,
    
    /// Number of target violations
    pub violations: u64,
}

impl Default for ComponentStats {
    fn default() -> Self {
        Self {
            sample_count: 0,
            avg_ms: 0,
            min_ms: 0,
            max_ms: 0,
            p50_ms: 0,
            p95_ms: 0,
            p99_ms: 0,
            target_ms: 0,
            compliance_rate: 100.0,
            violations: 0,
        }
    }
}

/// Simple benchmark utility for measuring operation performance
pub struct Benchmark {
    start_time: Instant,
    name: String,
}

impl Benchmark {
    /// Start a new benchmark
    pub fn start(name: &str) -> Self {
        debug!("🏁 Starting benchmark: {}", name);
        Self {
            start_time: Instant::now(),
            name: name.to_string(),
        }
    }
    
    /// Finish benchmark and return duration
    pub fn finish(self) -> Duration {
        let duration = self.start_time.elapsed();
        debug!("⏱️  Benchmark '{}' completed in {:?}", self.name, duration);
        duration
    }
    
    /// Finish with performance check against target
    pub fn finish_with_target(self, target: Duration) -> Duration {
        let duration = self.start_time.elapsed();
        
        if duration > target {
            warn!("⚠️  Benchmark '{}' exceeded target: {:?} > {:?}", 
                  self.name, duration, target);
        } else {
            debug!("✅ Benchmark '{}' within target: {:?} < {:?}", 
                   self.name, duration, target);
        }
        
        duration
    }
}

/// Macro for easy benchmarking
#[macro_export]
macro_rules! benchmark {
    ($name:expr, $target:expr, $block:block) => {{
        let _bench = crate::utils::performance::Benchmark::start($name);
        let result = $block;
        let duration = _bench.finish_with_target($target);
        (result, duration)
    }};
}

/// Global performance monitor instance
static PERFORMANCE_MONITOR: once_cell::sync::Lazy<PerformanceMonitor> = 
    once_cell::sync::Lazy::new(|| PerformanceMonitor::new(1000)); // Keep last 1000 samples

/// Get global performance monitor
pub fn get_performance_monitor() -> &'static PerformanceMonitor {
    &PERFORMANCE_MONITOR
}
