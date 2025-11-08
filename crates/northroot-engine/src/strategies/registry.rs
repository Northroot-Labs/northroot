//! Strategy registry for discovering and managing compute strategies.
//!
//! This module provides a registry system for managing and discovering strategies.
//! Strategies can be registered by name and retrieved dynamically, enabling
//! extensible compute pipelines.

use std::collections::HashMap;
use crate::strategies::trait_::Strategy;

/// Registry for managing strategies by name.
///
/// The registry allows strategies to be registered and retrieved by name,
/// enabling dynamic strategy selection and composition.
pub struct StrategyRegistry {
    strategies: HashMap<String, Box<dyn Strategy + Send + Sync>>,
}

impl std::fmt::Debug for StrategyRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrategyRegistry")
            .field("strategies", &self.strategies.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl StrategyRegistry {
    /// Create a new empty strategy registry.
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }

    /// Register a strategy in the registry.
    ///
    /// If a strategy with the same name already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `strategy` - Strategy to register (must implement `Strategy` trait)
    ///
    /// # Example
    ///
    /// ```rust
    /// use northroot_engine::strategies::{StrategyRegistry, IncrementalSumStrategy};
    ///
    /// let mut registry = StrategyRegistry::new();
    /// registry.register(IncrementalSumStrategy::new());
    /// ```
    pub fn register<S>(&mut self, strategy: S)
    where
        S: Strategy + Send + Sync + 'static,
    {
        let name = strategy.name().to_string();
        self.strategies.insert(name, Box::new(strategy));
    }

    /// Get a strategy by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Strategy name
    ///
    /// # Returns
    ///
    /// Reference to strategy if found, `None` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// use northroot_engine::strategies::{StrategyRegistry, IncrementalSumStrategy};
    ///
    /// let mut registry = StrategyRegistry::new();
    /// registry.register(IncrementalSumStrategy::new());
    ///
    /// let strategy = registry.get("incremental_sum");
    /// assert!(strategy.is_some());
    /// ```
    pub fn get(&self, name: &str) -> Option<&(dyn Strategy + Send + Sync)> {
        self.strategies.get(name).map(|s| s.as_ref())
    }

    /// List all registered strategy names.
    ///
    /// # Returns
    ///
    /// Vector of strategy names in arbitrary order
    ///
    /// # Example
    ///
    /// ```rust
    /// use northroot_engine::strategies::{StrategyRegistry, IncrementalSumStrategy, PartitionStrategy};
    ///
    /// let mut registry = StrategyRegistry::new();
    /// registry.register(IncrementalSumStrategy::new());
    /// registry.register(PartitionStrategy::new());
    ///
    /// let names = registry.list();
    /// assert_eq!(names.len(), 2);
    /// ```
    pub fn list(&self) -> Vec<&str> {
        self.strategies.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a strategy is registered.
    ///
    /// # Arguments
    ///
    /// * `name` - Strategy name
    ///
    /// # Returns
    ///
    /// `true` if strategy is registered, `false` otherwise
    pub fn contains(&self, name: &str) -> bool {
        self.strategies.contains_key(name)
    }

    /// Get the number of registered strategies.
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default registry with all built-in strategies.
///
/// This function creates a registry and registers all built-in strategies:
/// - `partition`: PartitionStrategy
/// - `incremental_sum`: IncrementalSumStrategy
///
/// # Example
///
/// ```rust
/// use northroot_engine::strategies::default_registry;
///
/// let registry = default_registry();
/// assert!(registry.contains("partition"));
/// assert!(registry.contains("incremental_sum"));
/// ```
pub fn default_registry() -> StrategyRegistry {
    use crate::strategies::{IncrementalSumStrategy, PartitionStrategy};
    
    let mut registry = StrategyRegistry::new();
    registry.register(PartitionStrategy::new());
    registry.register(IncrementalSumStrategy::new());
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::{IncrementalSumStrategy, PartitionStrategy};

    #[test]
    fn test_strategy_registry_new() {
        let registry = StrategyRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_strategy_registry_register_get() {
        let mut registry = StrategyRegistry::new();
        registry.register(IncrementalSumStrategy::new());

        let strategy = registry.get("incremental_sum");
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name(), "incremental_sum");
    }

    #[test]
    fn test_strategy_registry_list() {
        let mut registry = StrategyRegistry::new();
        registry.register(IncrementalSumStrategy::new());
        registry.register(PartitionStrategy::new());

        let names = registry.list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"incremental_sum"));
        assert!(names.contains(&"partition"));
    }

    #[test]
    fn test_strategy_registry_contains() {
        let mut registry = StrategyRegistry::new();
        registry.register(IncrementalSumStrategy::new());

        assert!(registry.contains("incremental_sum"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_default_registry() {
        let registry = default_registry();
        assert!(!registry.is_empty());
        assert!(registry.contains("partition"));
        assert!(registry.contains("incremental_sum"));
    }

    #[test]
    fn test_strategy_registry_replace() {
        let mut registry = StrategyRegistry::new();
        registry.register(IncrementalSumStrategy::new());
        assert_eq!(registry.len(), 1);

        // Registering again should replace, not add
        registry.register(IncrementalSumStrategy::with_field("amount".to_string()));
        assert_eq!(registry.len(), 1);
    }
}

