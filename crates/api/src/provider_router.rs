//! Provider routing — selects the best provider for each request.
//!
//! `ProviderRouter` routes requests based on health status, priority,
//! and configurable strategies (priority, round-robin, least-latency,
//! cost-optimal).

use std::fmt;

use crate::provider_health::{ProviderHealth, ProviderStatus};

// ---------------------------------------------------------------------------
// RoutingStrategy
// ---------------------------------------------------------------------------

/// How the router selects among available providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoutingStrategy {
    /// Use the highest-priority available provider.
    Priority,
    /// Rotate through available providers.
    RoundRobin,
    /// Pick the provider with lowest observed p50 latency.
    LeastLatency,
    /// Pick the provider with lowest estimated cost.
    CostOptimal,
}

impl fmt::Display for RoutingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Priority => write!(f, "priority"),
            Self::RoundRobin => write!(f, "round_robin"),
            Self::LeastLatency => write!(f, "least_latency"),
            Self::CostOptimal => write!(f, "cost_optimal"),
        }
    }
}

// ---------------------------------------------------------------------------
// ProviderConfig
// ---------------------------------------------------------------------------

/// Configuration for a single provider in the router.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider name (e.g., "anthropic", "openai").
    pub name: String,
    /// Priority (lower = higher priority). Used by `Priority` strategy.
    pub priority: u32,
    /// Weight for weighted selection (not currently used, reserved).
    pub weight: u32,
    /// Maximum concurrent requests to this provider (0 = unlimited).
    pub max_concurrent: u32,
    /// Cost multiplier relative to baseline (1.0 = baseline).
    pub cost_multiplier: f64,
}

impl ProviderConfig {
    /// Create a config with just a name and priority.
    #[must_use]
    pub fn new(name: impl Into<String>, priority: u32) -> Self {
        Self {
            name: name.into(),
            priority,
            weight: 1,
            max_concurrent: 0,
            cost_multiplier: 1.0,
        }
    }

    /// Set cost multiplier.
    #[must_use]
    pub fn with_cost_multiplier(mut self, multiplier: f64) -> Self {
        self.cost_multiplier = multiplier;
        self
    }

    /// Set max concurrent requests.
    #[must_use]
    pub fn with_max_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent = max;
        self
    }
}

// ---------------------------------------------------------------------------
// SelectedProvider
// ---------------------------------------------------------------------------

/// The result of routing a request.
#[derive(Debug, Clone)]
pub struct SelectedProvider {
    /// Provider name.
    pub name: String,
    /// Provider's current status.
    pub status: ProviderStatus,
    /// Why this provider was selected.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// ProviderRouter
// ---------------------------------------------------------------------------

/// Routes requests to the best available provider.
#[derive(Debug)]
pub struct ProviderRouter {
    providers: Vec<ProviderConfig>,
    strategy: RoutingStrategy,
    /// Round-robin counter.
    rr_counter: usize,
}

impl ProviderRouter {
    /// Create a router with the given strategy and provider configs.
    #[must_use]
    pub fn new(strategy: RoutingStrategy, providers: Vec<ProviderConfig>) -> Self {
        Self {
            providers,
            strategy,
            rr_counter: 0,
        }
    }

    /// The current routing strategy.
    #[must_use]
    pub fn strategy(&self) -> RoutingStrategy {
        self.strategy
    }

    /// Change the routing strategy.
    pub fn set_strategy(&mut self, strategy: RoutingStrategy) {
        self.strategy = strategy;
    }

    /// Number of configured providers.
    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Route a request, consulting the health tracker for availability.
    ///
    /// Returns `None` if no providers are available.
    pub fn route(&mut self, health: &ProviderHealth) -> Option<SelectedProvider> {
        // Collect indices of available providers to avoid borrow conflicts.
        let available_idx: Vec<usize> = self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let status = health.status(&p.name);
                matches!(
                    status,
                    ProviderStatus::Healthy | ProviderStatus::Degraded | ProviderStatus::Unknown
                )
            })
            .map(|(i, _)| i)
            .collect();

        if available_idx.is_empty() {
            return None;
        }

        let picked_idx = match self.strategy {
            RoutingStrategy::Priority => *available_idx
                .iter()
                .min_by_key(|&&i| self.providers[i].priority)?,
            RoutingStrategy::RoundRobin => {
                let pos = self.rr_counter % available_idx.len();
                self.rr_counter = self.rr_counter.wrapping_add(1);
                available_idx[pos]
            }
            RoutingStrategy::LeastLatency => *available_idx.iter().min_by_key(|&&i| {
                health
                    .metrics(&self.providers[i].name)
                    .map_or(std::time::Duration::MAX, |m| m.latency_p50)
            })?,
            RoutingStrategy::CostOptimal => *available_idx.iter().min_by(|&&a, &&b| {
                self.providers[a]
                    .cost_multiplier
                    .partial_cmp(&self.providers[b].cost_multiplier)
                    .unwrap()
            })?,
        };

        let p = &self.providers[picked_idx];
        let reason = match self.strategy {
            RoutingStrategy::Priority => format!("highest priority ({})", p.priority),
            RoutingStrategy::RoundRobin => "round-robin".to_string(),
            RoutingStrategy::LeastLatency => {
                let lat = health
                    .metrics(&p.name)
                    .map_or(0, |m| m.latency_p50.as_millis());
                format!("lowest p50 latency ({lat}ms)")
            }
            RoutingStrategy::CostOptimal => format!("lowest cost ({}x)", p.cost_multiplier),
        };

        Some(SelectedProvider {
            name: p.name.clone(),
            status: health.status(&p.name),
            reason,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn setup_providers() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig::new("anthropic", 1).with_cost_multiplier(1.5),
            ProviderConfig::new("openai", 2).with_cost_multiplier(1.0),
            ProviderConfig::new("deepseek", 3).with_cost_multiplier(0.3),
        ]
    }

    fn healthy_all(health: &mut ProviderHealth) {
        health.record_success("anthropic", Duration::from_millis(100));
        health.record_success("openai", Duration::from_millis(200));
        health.record_success("deepseek", Duration::from_millis(50));
    }

    #[test]
    fn strategy_display() {
        assert_eq!(RoutingStrategy::Priority.to_string(), "priority");
        assert_eq!(RoutingStrategy::RoundRobin.to_string(), "round_robin");
        assert_eq!(RoutingStrategy::LeastLatency.to_string(), "least_latency");
        assert_eq!(RoutingStrategy::CostOptimal.to_string(), "cost_optimal");
    }

    #[test]
    fn priority_selects_highest() {
        let mut router = ProviderRouter::new(RoutingStrategy::Priority, setup_providers());
        let mut health = ProviderHealth::new();
        healthy_all(&mut health);

        let selected = router.route(&health).unwrap();
        assert_eq!(selected.name, "anthropic");
        assert!(selected.reason.contains("priority"));
    }

    #[test]
    fn priority_skips_down_provider() {
        let mut router = ProviderRouter::new(RoutingStrategy::Priority, setup_providers());
        let mut health = ProviderHealth::new().with_down_threshold(2);
        healthy_all(&mut health);
        // Take anthropic down
        health.record_failure("anthropic");
        health.record_failure("anthropic");

        let selected = router.route(&health).unwrap();
        assert_eq!(selected.name, "openai");
    }

    #[test]
    fn round_robin_rotates() {
        let mut router = ProviderRouter::new(RoutingStrategy::RoundRobin, setup_providers());
        let mut health = ProviderHealth::new();
        healthy_all(&mut health);

        let names: Vec<String> = (0..6)
            .map(|_| router.route(&health).unwrap().name)
            .collect();
        // Should cycle through available providers
        assert_eq!(names[0], names[3]);
        assert_eq!(names[1], names[4]);
        assert_eq!(names[2], names[5]);
    }

    #[test]
    fn least_latency_picks_fastest() {
        let mut router = ProviderRouter::new(RoutingStrategy::LeastLatency, setup_providers());
        let mut health = ProviderHealth::new();
        health.record_success("anthropic", Duration::from_millis(300));
        health.record_success("openai", Duration::from_millis(200));
        health.record_success("deepseek", Duration::from_millis(50));

        let selected = router.route(&health).unwrap();
        assert_eq!(selected.name, "deepseek");
        assert!(selected.reason.contains("latency"));
    }

    #[test]
    fn cost_optimal_picks_cheapest() {
        let mut router = ProviderRouter::new(RoutingStrategy::CostOptimal, setup_providers());
        let mut health = ProviderHealth::new();
        healthy_all(&mut health);

        let selected = router.route(&health).unwrap();
        assert_eq!(selected.name, "deepseek");
        assert!(selected.reason.contains("cost"));
    }

    #[test]
    fn no_available_providers_returns_none() {
        let mut router = ProviderRouter::new(RoutingStrategy::Priority, setup_providers());
        let mut health = ProviderHealth::new().with_down_threshold(1);
        health.record_failure("anthropic");
        health.record_failure("openai");
        health.record_failure("deepseek");

        assert!(router.route(&health).is_none());
    }

    #[test]
    fn change_strategy() {
        let mut router = ProviderRouter::new(RoutingStrategy::Priority, setup_providers());
        assert_eq!(router.strategy(), RoutingStrategy::Priority);
        router.set_strategy(RoutingStrategy::LeastLatency);
        assert_eq!(router.strategy(), RoutingStrategy::LeastLatency);
    }

    #[test]
    fn provider_count() {
        let router = ProviderRouter::new(RoutingStrategy::Priority, setup_providers());
        assert_eq!(router.provider_count(), 3);
    }

    #[test]
    fn provider_config_builder() {
        let cfg = ProviderConfig::new("test", 5)
            .with_cost_multiplier(2.0)
            .with_max_concurrent(10);
        assert_eq!(cfg.name, "test");
        assert_eq!(cfg.priority, 5);
        assert!((cfg.cost_multiplier - 2.0).abs() < f64::EPSILON);
        assert_eq!(cfg.max_concurrent, 10);
    }

    #[test]
    fn unknown_providers_are_routable() {
        // Providers with no health data (Unknown status) should still be routable
        let mut router = ProviderRouter::new(RoutingStrategy::Priority, setup_providers());
        let health = ProviderHealth::new(); // no data at all
        let selected = router.route(&health).unwrap();
        assert_eq!(selected.name, "anthropic");
    }
}
