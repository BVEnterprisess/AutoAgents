use std::sync::Arc;
use std::time::Duration;

use hyper::http::StatusCode;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    CounterVec, HistogramVec, GaugeVec, Encoder, TextEncoder,
};
use tokio::sync::RwLock;
use tracing::info;

/// Metrics collector for the gateway
#[derive(Clone)]
pub struct MetricsCollector {
    http_requests_total: CounterVec,
    http_request_duration: HistogramVec,
    active_connections: GaugeVec,
    cache_hits_total: CounterVec,
    cache_misses_total: CounterVec,
    rate_limit_exceeded_total: CounterVec,
    upstream_errors_total: CounterVec,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let http_requests_total = register_counter_vec!(
            "gateway_http_requests_total",
            "Total number of HTTP requests processed",
            &["method", "status", "path"]
        ).unwrap();

        let http_request_duration = register_histogram_vec!(
            "gateway_http_request_duration_seconds",
            "HTTP request duration in seconds",
            &["method", "status"],
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0]
        ).unwrap();

        let active_connections = register_gauge_vec!(
            "gateway_active_connections",
            "Number of active connections",
            &["upstream"]
        ).unwrap();

        let cache_hits_total = register_counter_vec!(
            "gateway_cache_hits_total",
            "Total number of cache hits",
            &["cache_type"]
        ).unwrap();

        let cache_misses_total = register_counter_vec!(
            "gateway_cache_misses_total",
            "Total number of cache misses",
            &["cache_type"]
        ).unwrap();

        let rate_limit_exceeded_total = register_counter_vec!(
            "gateway_rate_limit_exceeded_total",
            "Total number of rate limit violations",
            &["client_type"]
        ).unwrap();

        let upstream_errors_total = register_counter_vec!(
            "gateway_upstream_errors_total",
            "Total number of upstream errors",
            &["upstream", "error_type"]
        ).unwrap();

        Self {
            http_requests_total,
            http_request_duration,
            active_connections,
            cache_hits_total,
            cache_misses_total,
            rate_limit_exceeded_total,
            upstream_errors_total,
        }
    }

    /// Record an HTTP request
    pub fn record_request(&self, status: StatusCode, duration: Duration) {
        let method = "unknown"; // TODO: Pass method from caller
        let path = "unknown";   // TODO: Pass path from caller

        self.http_requests_total
            .with_label_values(&[method, &status.as_u16().to_string(), path])
            .inc();

        self.http_request_duration
            .with_label_values(&[method, &status.as_u16().to_string()])
            .observe(duration.as_secs_f64());
    }

    /// Record cache hit
    pub fn record_cache_hit(&self, cache_type: &str) {
        self.cache_hits_total
            .with_label_values(&[cache_type])
            .inc();
    }

    /// Record cache miss
    pub fn record_cache_miss(&self, cache_type: &str) {
        self.cache_misses_total
            .with_label_values(&[cache_type])
            .inc();
    }

    /// Record rate limit exceeded
    pub fn record_rate_limit_exceeded(&self, client_type: &str) {
        self.rate_limit_exceeded_total
            .with_label_values(&[client_type])
            .inc();
    }

    /// Record upstream error
    pub fn record_upstream_error(&self, upstream: &str, error_type: &str) {
        self.upstream_errors_total
            .with_label_values(&[upstream, error_type])
            .inc();
    }

    /// Update active connections gauge
    pub fn update_active_connections(&self, upstream: &str, count: f64) {
        self.active_connections
            .with_label_values(&[upstream])
            .set(count);
    }

    /// Increment active connections
    pub fn increment_active_connections(&self, upstream: &str) {
        self.active_connections
            .with_label_values(&[upstream])
            .inc();
    }

    /// Decrement active connections
    pub fn decrement_active_connections(&self, upstream: &str) {
        self.active_connections
            .with_label_values(&[upstream])
            .dec();
    }

    /// Get metrics in Prometheus format
    pub fn gather_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Get cache hit ratio
    pub fn get_cache_hit_ratio(&self, cache_type: &str) -> f64 {
        let hits = self.cache_hits_total
            .with_label_values(&[cache_type])
            .get() as f64;

        let misses = self.cache_misses_total
            .with_label_values(&[cache_type])
            .get() as f64;

        let total = hits + misses;
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }

    /// Get total requests by status code
    pub fn get_requests_by_status(&self, status: StatusCode) -> u64 {
        // This is a simplified version - in practice you'd need to aggregate across all labels
        self.http_requests_total
            .with_label_values(&["GET", &status.as_u16().to_string(), "/"])
            .get()
    }

    /// Get average request duration
    pub fn get_average_request_duration(&self) -> Duration {
        // This is a simplified version - in practice you'd need to calculate from histogram
        Duration::from_millis(100) // Placeholder
    }

    /// Reset all metrics (useful for testing)
    pub fn reset(&self) {
        // Note: Prometheus counters can't be reset, but we can recreate them
        // This is mainly for testing purposes
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check metrics
pub struct HealthMetrics {
    pub last_health_check: Arc<RwLock<std::time::Instant>>,
    pub health_check_failures: CounterVec,
}

impl HealthMetrics {
    pub fn new() -> Self {
        let health_check_failures = register_counter_vec!(
            "gateway_health_check_failures_total",
            "Total number of health check failures",
            &["service"]
        ).unwrap();

        Self {
            last_health_check: Arc::new(RwLock::new(std::time::Instant::now())),
            health_check_failures,
        }
    }

    pub async fn record_health_check(&self, service: &str, success: bool) {
        *self.last_health_check.write().await = std::time::Instant::now();

        if !success {
            self.health_check_failures
                .with_label_values(&[service])
                .inc();
        }
    }

    pub async fn get_last_health_check(&self) -> std::time::Instant {
        *self.last_health_check.read().await
    }
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics
pub struct PerformanceMetrics {
    pub request_queue_size: GaugeVec,
    pub memory_usage: GaugeVec,
    pub cpu_usage: GaugeVec,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        let request_queue_size = register_gauge_vec!(
            "gateway_request_queue_size",
            "Current request queue size",
            &["queue_type"]
        ).unwrap();

        let memory_usage = register_gauge_vec!(
            "gateway_memory_usage_bytes",
            "Current memory usage in bytes",
            &["memory_type"]
        ).unwrap();

        let cpu_usage = register_gauge_vec!(
            "gateway_cpu_usage_percent",
            "Current CPU usage percentage",
            &["cpu_type"]
        ).unwrap();

        Self {
            request_queue_size,
            memory_usage,
            cpu_usage,
        }
    }

    pub fn update_request_queue_size(&self, queue_type: &str, size: f64) {
        self.request_queue_size
            .with_label_values(&[queue_type])
            .set(size);
    }

    pub fn update_memory_usage(&self, memory_type: &str, bytes: f64) {
        self.memory_usage
            .with_label_values(&[memory_type])
            .set(bytes);
    }

    pub fn update_cpu_usage(&self, cpu_type: &str, percentage: f64) {
        self.cpu_usage
            .with_label_values(&[cpu_type])
            .set(percentage);
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}
