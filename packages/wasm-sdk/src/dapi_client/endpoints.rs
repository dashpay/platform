//! DAPI endpoint management

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Endpoint health status
#[derive(Debug, Clone)]
pub struct EndpointHealth {
    pub url: String,
    pub is_healthy: bool,
    pub last_check: Instant,
    pub consecutive_failures: u32,
    pub average_latency: Option<Duration>,
}

/// Endpoint manager for load balancing and failover
pub struct EndpointManager {
    endpoints: Vec<EndpointHealth>,
    health_check_interval: Duration,
}

impl EndpointManager {
    /// Create a new endpoint manager
    pub fn new(urls: Vec<String>) -> Self {
        let endpoints = urls.into_iter()
            .map(|url| EndpointHealth {
                url,
                is_healthy: true,
                last_check: Instant::now(),
                consecutive_failures: 0,
                average_latency: None,
            })
            .collect();

        EndpointManager {
            endpoints,
            health_check_interval: Duration::from_secs(30),
        }
    }

    /// Get the next healthy endpoint
    pub fn get_next_endpoint(&self) -> Option<&str> {
        self.endpoints.iter()
            .find(|ep| ep.is_healthy)
            .map(|ep| ep.url.as_str())
    }

    /// Mark endpoint as failed
    pub fn mark_failed(&mut self, url: &str) {
        if let Some(endpoint) = self.endpoints.iter_mut().find(|ep| ep.url == url) {
            endpoint.consecutive_failures += 1;
            if endpoint.consecutive_failures >= 3 {
                endpoint.is_healthy = false;
            }
            endpoint.last_check = Instant::now();
        }
    }

    /// Mark endpoint as successful
    pub fn mark_success(&mut self, url: &str, latency: Duration) {
        if let Some(endpoint) = self.endpoints.iter_mut().find(|ep| ep.url == url) {
            endpoint.consecutive_failures = 0;
            endpoint.is_healthy = true;
            endpoint.last_check = Instant::now();
            
            // Update average latency
            if let Some(avg) = endpoint.average_latency {
                // Simple moving average
                endpoint.average_latency = Some(Duration::from_millis(
                    ((avg.as_millis() * 4 + latency.as_millis()) / 5) as u64
                ));
            } else {
                endpoint.average_latency = Some(latency);
            }
        }
    }

    /// Get all endpoints sorted by health and latency
    pub fn get_sorted_endpoints(&self) -> Vec<&str> {
        let mut sorted: Vec<_> = self.endpoints.iter().collect();
        
        sorted.sort_by(|a, b| {
            // First sort by health
            if a.is_healthy != b.is_healthy {
                return b.is_healthy.cmp(&a.is_healthy);
            }
            
            // Then by latency
            match (a.average_latency, b.average_latency) {
                (Some(a_lat), Some(b_lat)) => a_lat.cmp(&b_lat),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        sorted.into_iter().map(|ep| ep.url.as_str()).collect()
    }

    /// Check if health checks are needed
    pub fn needs_health_check(&self) -> bool {
        self.endpoints.iter().any(|ep| {
            !ep.is_healthy && ep.last_check.elapsed() > self.health_check_interval
        })
    }
}