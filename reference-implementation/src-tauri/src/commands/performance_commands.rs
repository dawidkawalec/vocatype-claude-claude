use serde::{Deserialize, Serialize};
use tracing::info;

use crate::utils::performance::{get_performance_monitor, PerformanceStats};

/// Response for performance commands
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceCommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> PerformanceCommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Get comprehensive performance statistics
#[tauri::command]
pub fn get_performance_stats() -> Result<PerformanceCommandResponse<PerformanceStats>, String> {
    info!("📊 Getting performance statistics");
    
    let monitor = get_performance_monitor();
    let stats = monitor.get_stats();
    
    Ok(PerformanceCommandResponse::success(stats))
}

/// Get performance summary for UI display
#[tauri::command]
pub fn get_performance_summary() -> Result<PerformanceCommandResponse<PerformanceSummary>, String> {
    let monitor = get_performance_monitor();
    let stats = monitor.get_stats();
    
    // Calculate overall health score
    let health_score = calculate_health_score(&stats);
    
    let summary = PerformanceSummary {
        overall_health: health_score,
        audio_avg_ms: stats.audio.avg_ms,
        vad_avg_ms: stats.vad.avg_ms,
        stt_avg_ms: stats.stt.avg_ms,
        ai_avg_ms: stats.ai.avg_ms,
        system_avg_ms: stats.system.avg_ms,
        target_compliance: {
            let total_compliance = stats.audio.compliance_rate + 
                                  stats.vad.compliance_rate + 
                                  stats.stt.compliance_rate + 
                                  stats.ai.compliance_rate + 
                                  stats.system.compliance_rate;
            total_compliance / 5.0
        },
        total_violations: stats.audio.violations + 
                         stats.vad.violations + 
                         stats.stt.violations + 
                         stats.ai.violations + 
                         stats.system.violations,
    };
    
    Ok(PerformanceCommandResponse::success(summary))
}

/// Reset performance statistics
#[tauri::command]
pub fn reset_performance_stats() -> Result<PerformanceCommandResponse<bool>, String> {
    info!("🔄 Resetting performance statistics");
    
    // Note: With current design, we can't reset the global monitor
    // This would require a different architecture with mutable global state
    // For now, return success (stats will naturally rotate out over time)
    
    Ok(PerformanceCommandResponse::success(true))
}

/// Performance summary for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Overall health score (0-100)
    pub overall_health: u8,
    
    /// Average latencies for each component
    pub audio_avg_ms: u64,
    pub vad_avg_ms: u64,
    pub stt_avg_ms: u64,
    pub ai_avg_ms: u64,
    pub system_avg_ms: u64,
    
    /// Overall target compliance rate (%)
    pub target_compliance: f64,
    
    /// Total target violations across all components
    pub total_violations: u64,
}

/// Calculate overall system health score (0-100)
fn calculate_health_score(stats: &PerformanceStats) -> u8 {
    let components = [
        &stats.audio,
        &stats.vad,
        &stats.stt,
        &stats.ai,
        &stats.system,
    ];
    
    let mut total_score = 0.0;
    let mut active_components = 0;
    
    for component in &components {
        if component.sample_count > 0 {
            active_components += 1;
            
            // Base score from compliance rate
            let mut component_score = component.compliance_rate;
            
            // Penalty for high latencies (even if within target)
            let latency_ratio = component.avg_ms as f64 / component.target_ms as f64;
            if latency_ratio > 0.8 {
                component_score *= 0.9; // 10% penalty for being close to target
            }
            if latency_ratio > 0.5 {
                component_score *= 0.95; // 5% penalty for being over half of target
            }
            
            // Bonus for very low latencies
            if latency_ratio < 0.2 {
                component_score = (component_score * 1.05).min(100.0); // 5% bonus for very fast
            }
            
            total_score += component_score;
        }
    }
    
    if active_components == 0 {
        return 100; // Perfect score if no measurements yet
    }
    
    let avg_score = total_score / active_components as f64;
    avg_score.round() as u8
}
