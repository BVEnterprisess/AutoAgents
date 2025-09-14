//! Revenue analytics and performance tracking for Infrastructure Assassin
//!
//! This module provides cost disruption metrics comparing against AWS/Google competitors
//! and tracks productivity gains through tool orchestration.

pub mod revenue;
pub mod performance;

use crate::{InfrastructureMetrics, RevenueAnalytics, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Analytics tracker for revenue and performance metrics
#[derive(Debug, Clone)]
pub struct AnalyticsTracker {
    pub revenue_data: RevenueAnalytics,
    pub performance_metrics: Vec<InfrastructureMetrics>,
    pub baseline_metrics: Option<BaselineMetrics>,
    pub historical_data: Vec<ExecutionRecord>,
}

/// Baseline metrics for AWS/Google competitive benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    pub aws_lambda_cost_per_request: f64,
    pub google_cloud_run_cost_per_request: f64,
    pub baseline_execution_time: f64,
    pub baseline_memory_usage: usize,
    pub recorded_at: DateTime<Utc>,
}

/// Historical execution record for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub execution_time: f64,
    pub memory_used: usize,
    pub cpu_used: f64,
    pub network_latency: f64,
    pub tools_orchestrated: usize,
    pub cost_savings: f64,
}

/// Revenue projection for enterprise customers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueProjection {
    pub conservative_estimate: f64,  // $25K/year per customer
    pub aggressive_estimate: f64,   // $100K/year per customer
    pub market_penetration: f32,    // Percentage of AWS market
    pub customer_acquisition_time: f32, // Months to acquire
}

/// Competitive analysis against cloud providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitiveAnalysis {
    pub aws_serverless_cost: f64,
    pub google_serverless_cost: f64,
    pub infrastructure_assassin_cost: f64, // Always $0
    pub productivity_multiplier: f64, // 10x claimed
    pub tool_ecosystem_size: usize, // 16K+ MCP tools
}

/// Performance dashboard for infrastructure monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDashboard {
    pub average_session_duration: f64,
    pub peak_memory_usage: usize,
    pub container_efficiency: f32,
    pub network_latency_p95: f64,
    pub orchestrations_per_hour: f64,
    pub error_rate: f32,
}

impl AnalyticsTracker {
    /// Create new analytics tracker with baseline setup
    pub fn new() -> Self {
        Self {
            revenue_data: RevenueAnalytics::default(),
            performance_metrics: Vec::new(),
            baseline_metrics: Some(BaselineMetrics::default()),
            historical_data: Vec::new(),
        }
    }

    /// Record execution metrics and update analytics
    pub fn record_execution(&mut self, metrics: InfrastructureMetrics, result: &super::ExecutionResult) {
        // Update performance metrics
        self.performance_metrics.push(metrics.clone());

        // Calculate cost savings vs competitors
        let execution_record = ExecutionRecord {
            timestamp: chrono::Utc::now(),
            session_id: result.session_id.to_string(),
            execution_time: metrics.session_duration,
            memory_used: metrics.memory_usage,
            cpu_used: metrics.cpu_cycles,
            network_latency: metrics.network_latency,
            tools_orchestrated: result.tools_used.len(),
            cost_savings: self.calculate_cost_savings(&metrics),
        };

        self.historical_data.push(execution_record);

        // Update revenue analytics
        self.update_revenue_analytics(&result);
    }

    /// Calculate cost savings compared to AWS/Google
    pub fn calculate_cost_savings(&self, metrics: &InfrastructureMetrics) -> f64 {
        if let Some(baseline) = &self.baseline_metrics {
            // AWS Lambda pricing: $0.20 per 1M requests + duration costs
            let aws_cost = (baseline.aws_lambda_cost_per_request * 1000.0) +
                          (metrics.session_duration * 0.0000166667); // ~$0.0001 per GB-second

            // Google Cloud Run pricing: similar structure
            let google_cost = baseline.google_cloud_run_cost_per_request * 1000.0;

            // Infrastructure Assassin = $0 cost
            aws_cost.max(google_cost)
        } else {
            0.0
        }
    }

    /// Update revenue analytics based on execution results
    fn update_revenue_analytics(&mut self, result: &super::ExecutionResult) {
        self.revenue_data.tool_orchestrations += result.tools_used.len() as u64;

        // Assume 10x productivity gain leads to revenue
        // Each orchestration saves developer hours = dollars
        let productivity_savings = result.tools_used.len() as f64 * 50.0; // $50/hour saved
        self.revenue_data.productivity_gain += productivity_savings;
    }

    /// Generate competitive analysis report
    pub fn generate_competitive_analysis(&self) -> CompetitiveAnalysis {
        CompetitiveAnalysis {
            aws_serverless_cost: 12000.0, // $12K/month example
            google_serverless_cost: 9500.0, // $9.5K/month example
            infrastructure_assassin_cost: 0.0, // $0 cost
            productivity_multiplier: 10.0,
            tool_ecosystem_size: 16000, // 16K+ MCP tools
        }
    }

    /// Generate performance dashboard
    pub fn generate_performance_dashboard(&self) -> PerformanceDashboard {
        if self.performance_metrics.is_empty() {
            return PerformanceDashboard::default();
        }

        let session_durations: Vec<f64> = self.performance_metrics.iter().map(|m| m.session_duration).collect();
        let memory_usages: Vec<usize> = self.performance_metrics.iter().map(|m| m.memory_usage).collect();
        let latencies: Vec<f64> = self.performance_metrics.iter().map(|m| m.network_latency).collect();

        PerformanceDashboard {
            average_session_duration: session_durations.iter().sum::<f64>() / session_durations.len() as f64,
            peak_memory_usage: *memory_usages.iter().max().unwrap_or(&0),
            container_efficiency: self.performance_metrics.iter().map(|m| m.container_efficiency).sum::<f32>() /
                                self.performance_metrics.len() as f32,
            network_latency_p95: calculate_p95(&latencies),
            orchestrations_per_hour: self.revenue_data.tool_orchestrations as f64 / 24.0,
            error_rate: 0.01, // Placeholder - calculate from actual errors
        }
    }

    /// Generate enterprise revenue projection
    pub fn generate_revenue_projection(&self) -> RevenueProjection {
        RevenueProjection {
            conservative_estimate: 25000.0,  // $25K/year per customer
            aggressive_estimate: 100000.0,  // $100K/year per customer
            market_penetration: 0.1,         // 10% of market initially
            customer_acquisition_time: 6.0,  // 6 months to acquire
        }
    }

    /// Calculate total cost disruption impact
    pub fn calculate_disruption_impact(&self) -> HashMap<String, f64> {
        let mut impact = HashMap::new();

        // Total AWS cost saved across all sessions
        let total_aws_savings: f64 = self.historical_data.iter().map(|r| r.cost_savings).sum();
        impact.insert("total_aws_cost_saved".to_string(), total_aws_savings);

        // Productivity gains in developer hours
        impact.insert("productivity_gain_multiplier".to_string(), self.revenue_data.productivity_gain / 1000.0);

        // Market disruption potential
        let analysis = self.generate_competitive_analysis();
        let disruption_ratio = (analysis.aws_serverless_cost + analysis.google_serverless_cost) / 2.0;
        impact.insert("market_disruption_ratio".to_string(), disruption_ratio);

        // Tool ecosystem leverage
        impact.insert("tool_ecosystem_leverage".to_string(), analysis.tool_ecosystem_size as f64);

        impact
    }
}

impl Default for BaselineMetrics {
    fn default() -> Self {
        Self {
            aws_lambda_cost_per_request: 0.0000002, // $0.20 per 1M requests
            google_cloud_run_cost_per_request: 0.00000016, // $0.16 per 1M requests
            baseline_execution_time: 1.0, // 1 second average
            baseline_memory_usage: 128, // 128MB
            recorded_at: chrono::Utc::now(),
        }
    }
}

impl Default for PerformanceDashboard {
    fn default() -> Self {
        Self {
            average_session_duration: 0.0,
            peak_memory_usage: 0,
            container_efficiency: 0.0,
            network_latency_p95: 0.0,
            orchestrations_per_hour: 0.0,
            error_rate: 0.0,
        }
    }
}

/// Calculate 95th percentile of a vector of f64 values
fn calculate_p95(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((sorted.len() - 1) as f64 * 0.95) as usize;

    *sorted.get(index).unwrap_or(&0.0)
}
