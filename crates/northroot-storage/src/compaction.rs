//! Compaction tier support for storage optimization.
//!
//! This module provides compaction tier management for manifests, enabling
//! cost-effective storage based on access patterns:
//!
//! - **Hot**: Full manifests, summaries, Bloom filters (fast access)
//! - **Warm**: Summaries only, manifests compressed
//! - **Cold**: Summaries only, manifests archived
//!
//! This is part of ADR-0009-P05: Summarized manifests for fast overlap.

use crate::error::StorageError;

/// Compaction tier for manifest storage.
///
/// Tiers represent different storage strategies based on access patterns
/// and cost optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionTier {
    /// Hot tier: Full manifests, summaries, Bloom filters (fast access)
    ///
    /// Used for recently accessed or frequently queried manifests.
    /// All data is immediately available.
    Hot = 1,

    /// Warm tier: Summaries only, manifests compressed
    ///
    /// Used for manifests that are accessed occasionally.
    /// Full manifest must be decompressed before use.
    Warm = 2,

    /// Cold tier: Summaries only, manifests archived
    ///
    /// Used for old manifests that are rarely accessed.
    /// Full manifest must be retrieved from archive (e.g., S3/Glacier).
    Cold = 3,
}

impl CompactionTier {
    /// Create a tier from a numeric value.
    ///
    /// # Arguments
    ///
    /// * `tier` - Tier number (1=Hot, 2=Warm, 3=Cold)
    ///
    /// # Returns
    ///
    /// `Some(CompactionTier)` if valid, `None` otherwise
    pub fn from_u8(tier: u8) -> Option<Self> {
        match tier {
            1 => Some(Self::Hot),
            2 => Some(Self::Warm),
            3 => Some(Self::Cold),
            _ => None,
        }
    }

    /// Get the numeric value of the tier.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Check if this tier has full manifest available.
    pub fn has_full_manifest(self) -> bool {
        matches!(self, Self::Hot | Self::Warm)
    }

    /// Check if this tier has manifest summary available.
    pub fn has_summary(self) -> bool {
        true // All tiers have summaries
    }

    /// Check if this tier has Bloom filter available.
    pub fn has_bloom_filter(self) -> bool {
        matches!(self, Self::Hot)
    }

    /// Check if manifest needs decompression.
    pub fn needs_decompression(self) -> bool {
        matches!(self, Self::Warm)
    }

    /// Check if manifest needs archive retrieval.
    pub fn needs_archive_retrieval(self) -> bool {
        matches!(self, Self::Cold)
    }
}

/// Metadata about a manifest's compaction tier.
#[derive(Debug, Clone)]
pub struct CompactionTierInfo {
    /// Current tier
    pub tier: CompactionTier,
    /// Whether full manifest is available
    pub has_full_manifest: bool,
    /// Whether summary is available
    pub has_summary: bool,
    /// Whether Bloom filter is available
    pub has_bloom_filter: bool,
    /// Timestamp when tier was set (Unix epoch seconds)
    pub tier_set_at: i64,
}

impl CompactionTierInfo {
    /// Create new tier info.
    pub fn new(tier: CompactionTier, tier_set_at: i64) -> Self {
        Self {
            has_full_manifest: tier.has_full_manifest(),
            has_summary: tier.has_summary(),
            has_bloom_filter: tier.has_bloom_filter(),
            tier,
            tier_set_at,
        }
    }
}

/// Trait for storage backends that support compaction tiers.
///
/// This trait extends `ReceiptStore` with compaction tier management.
pub trait CompactionTierStore {
    /// Get the compaction tier for a manifest.
    ///
    /// # Arguments
    ///
    /// * `manifest_hash` - Hash of the manifest (32 bytes)
    ///
    /// # Returns
    ///
    /// `Some(CompactionTierInfo)` if found, `None` if not found
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn get_manifest_tier(
        &self,
        manifest_hash: &[u8; 32],
    ) -> Result<Option<CompactionTierInfo>, StorageError>;

    /// Set the compaction tier for a manifest.
    ///
    /// # Arguments
    ///
    /// * `manifest_hash` - Hash of the manifest (32 bytes)
    /// * `tier` - New compaction tier
    /// * `tier_set_at` - Timestamp when tier is set (Unix epoch seconds)
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn set_manifest_tier(
        &self,
        manifest_hash: &[u8; 32],
        tier: CompactionTier,
        tier_set_at: i64,
    ) -> Result<(), StorageError>;

    /// Get all manifests in a specific tier.
    ///
    /// # Arguments
    ///
    /// * `tier` - Compaction tier to query
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    ///
    /// Vector of manifest hashes in the tier
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn get_manifests_in_tier(
        &self,
        tier: CompactionTier,
        limit: Option<usize>,
    ) -> Result<Vec<[u8; 32]>, StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_tier_from_u8() {
        assert_eq!(CompactionTier::from_u8(1), Some(CompactionTier::Hot));
        assert_eq!(CompactionTier::from_u8(2), Some(CompactionTier::Warm));
        assert_eq!(CompactionTier::from_u8(3), Some(CompactionTier::Cold));
        assert_eq!(CompactionTier::from_u8(0), None);
        assert_eq!(CompactionTier::from_u8(4), None);
    }

    #[test]
    fn test_compaction_tier_capabilities() {
        assert!(CompactionTier::Hot.has_full_manifest());
        assert!(CompactionTier::Hot.has_summary());
        assert!(CompactionTier::Hot.has_bloom_filter());
        assert!(!CompactionTier::Hot.needs_decompression());
        assert!(!CompactionTier::Hot.needs_archive_retrieval());

        assert!(CompactionTier::Warm.has_full_manifest());
        assert!(CompactionTier::Warm.has_summary());
        assert!(!CompactionTier::Warm.has_bloom_filter());
        assert!(CompactionTier::Warm.needs_decompression());
        assert!(!CompactionTier::Warm.needs_archive_retrieval());

        assert!(!CompactionTier::Cold.has_full_manifest());
        assert!(CompactionTier::Cold.has_summary());
        assert!(!CompactionTier::Cold.has_bloom_filter());
        assert!(!CompactionTier::Cold.needs_decompression());
        assert!(CompactionTier::Cold.needs_archive_retrieval());
    }

    #[test]
    fn test_compaction_tier_info() {
        let info = CompactionTierInfo::new(CompactionTier::Hot, 1234567890);
        assert_eq!(info.tier, CompactionTier::Hot);
        assert!(info.has_full_manifest);
        assert!(info.has_summary);
        assert!(info.has_bloom_filter);
        assert_eq!(info.tier_set_at, 1234567890);
    }
}
