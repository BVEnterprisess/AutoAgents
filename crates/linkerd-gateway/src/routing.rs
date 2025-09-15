use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::{Route, RoutingConfig, LoadBalancingStrategy};

/// Router for matching requests to routes and handling load balancing
pub struct Router {
    config: RoutingConfig,
    round_robin_index: AtomicUsize,
    connection_counts: HashMap<String, AtomicUsize>,
}

impl Router {
    /// Create a new router
    pub fn new(config: RoutingConfig) -> Self {
        let mut connection_counts = HashMap::new();

        // Initialize connection counts for all upstreams
        for route in &config.routes {
            connection_counts.entry(route.upstream.clone())
                .or_insert_with(|| AtomicUsize::new(0));
        }

        Self {
            config,
            round_robin_index: AtomicUsize::new(0),
            connection_counts,
        }
    }

    /// Find matching route for the given path and method
    pub fn find_route(&self, path: &str, method: &hyper::Method) -> Option<&Route> {
        for route in &self.config.routes {
            if self.matches_route(route, path, method) {
                return Some(route);
            }
        }

        // Return default route if no specific match found
        if let Some(default_upstream) = &self.config.default_upstream {
            return Some(&Route {
                path: "/*".to_string(),
                upstream: default_upstream.clone(),
                methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()],
                headers: HashMap::new(),
                timeout_ms: Some(30000),
            });
        }

        None
    }

    /// Check if a route matches the given path and method
    fn matches_route(&self, route: &Route, path: &str, method: &hyper::Method) -> bool {
        // Check method
        if !route.methods.is_empty() && !route.methods.contains(&method.to_string()) {
            return false;
        }

        // Check path
        if route.path == path {
            return true;
        }

        // Handle wildcard routes
        if route.path.ends_with("/*") {
            let prefix = &route.path[..route.path.len() - 2];
            return path.starts_with(prefix);
        }

        // Handle parameterized routes
        if route.path.contains("{") && route.path.contains("}") {
            return self.matches_parameterized_route(&route.path, path);
        }

        false
    }

    /// Match parameterized routes like /api/{id}/users
    fn matches_parameterized_route(&self, route_pattern: &str, path: &str) -> bool {
        let route_parts: Vec<&str> = route_pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if route_parts.len() != path_parts.len() {
            return false;
        }

        for (route_part, path_part) in route_parts.iter().zip(path_parts.iter()) {
            if route_part.starts_with('{') && route_part.ends_with('}') {
                // Parameter - always matches
                continue;
            } else if route_part != path_part {
                return false;
            }
        }

        true
    }

    /// Select upstream using load balancing strategy
    pub fn select_upstream(&self, route: &Route) -> String {
        match &self.config.load_balancing {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(route),
            LoadBalancingStrategy::LeastConnections => self.select_least_connections(route),
            LoadBalancingStrategy::Random => self.select_random(route),
            LoadBalancingStrategy::Weighted { weights } => self.select_weighted(route, weights),
        }
    }

    /// Round-robin load balancing
    fn select_round_robin(&self, route: &Route) -> String {
        let upstreams = vec![route.upstream.clone()];
        let index = self.round_robin_index.fetch_add(1, Ordering::SeqCst) % upstreams.len();
        upstreams[index].clone()
    }

    /// Least connections load balancing
    fn select_least_connections(&self, route: &Route) -> String {
        let upstreams = vec![route.upstream.clone()];
        let mut min_connections = usize::MAX;
        let mut selected = route.upstream.clone();

        for upstream in upstreams {
            let count = self.connection_counts
                .get(&upstream)
                .map(|c| c.load(Ordering::SeqCst))
                .unwrap_or(0);

            if count < min_connections {
                min_connections = count;
                selected = upstream;
            }
        }

        selected
    }

    /// Random load balancing
    fn select_random(&self, route: &Route) -> String {
        use rand::Rng;
        let upstreams = vec![route.upstream.clone()];
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..upstreams.len());
        upstreams[index].clone()
    }

    /// Weighted load balancing
    fn select_weighted(&self, route: &Route, weights: &HashMap<String, u32>) -> String {
        let upstreams = vec![route.upstream.clone()];
        let total_weight: u32 = upstreams.iter()
            .map(|u| weights.get(u).copied().unwrap_or(1))
            .sum();

        if total_weight == 0 {
            return route.upstream.clone();
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut random_weight = rng.gen_range(0..total_weight);

        for upstream in upstreams {
            let weight = weights.get(&upstream).copied().unwrap_or(1);
            if random_weight < weight {
                return upstream;
            }
            random_weight -= weight;
        }

        route.upstream.clone()
    }

    /// Increment connection count for an upstream
    pub fn increment_connection(&self, upstream: &str) {
        if let Some(count) = self.connection_counts.get(upstream) {
            count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Decrement connection count for an upstream
    pub fn decrement_connection(&self, upstream: &str) {
        if let Some(count) = self.connection_counts.get(upstream) {
            count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Get current connection count for an upstream
    pub fn get_connection_count(&self, upstream: &str) -> usize {
        self.connection_counts
            .get(upstream)
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(0)
    }
}

impl Clone for Router {
    fn clone(&self) -> Self {
        let mut connection_counts = HashMap::new();
        for (upstream, count) in &self.connection_counts {
            connection_counts.insert(upstream.clone(), AtomicUsize::new(count.load(Ordering::SeqCst)));
        }

        Self {
            config: self.config.clone(),
            round_robin_index: AtomicUsize::new(self.round_robin_index.load(Ordering::SeqCst)),
            connection_counts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Method;

    #[test]
    fn test_exact_path_match() {
        let config = RoutingConfig {
            routes: vec![Route {
                path: "/api/users".to_string(),
                upstream: "http://localhost:8081".to_string(),
                methods: vec!["GET".to_string()],
                headers: HashMap::new(),
                timeout_ms: None,
            }],
            default_upstream: None,
            load_balancing: LoadBalancingStrategy::RoundRobin,
        };

        let router = Router::new(config);
        let route = router.find_route("/api/users", &Method::GET);
        assert!(route.is_some());
        assert_eq!(route.unwrap().upstream, "http://localhost:8081");
    }

    #[test]
    fn test_wildcard_match() {
        let config = RoutingConfig {
            routes: vec![Route {
                path: "/api/*".to_string(),
                upstream: "http://localhost:8081".to_string(),
                methods: vec![],
                headers: HashMap::new(),
                timeout_ms: None,
            }],
            default_upstream: None,
            load_balancing: LoadBalancingStrategy::RoundRobin,
        };

        let router = Router::new(config);
        let route = router.find_route("/api/users/123", &Method::GET);
        assert!(route.is_some());
    }

    #[test]
    fn test_method_filtering() {
        let config = RoutingConfig {
            routes: vec![Route {
                path: "/api/users".to_string(),
                upstream: "http://localhost:8081".to_string(),
                methods: vec!["POST".to_string()],
                headers: HashMap::new(),
                timeout_ms: None,
            }],
            default_upstream: None,
            load_balancing: LoadBalancingStrategy::RoundRobin,
        };

        let router = Router::new(config);
        assert!(router.find_route("/api/users", &Method::POST).is_some());
        assert!(router.find_route("/api/users", &Method::GET).is_none());
    }
}
