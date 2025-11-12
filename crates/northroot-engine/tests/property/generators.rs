//! Custom generators for property tests.
//!
//! This module provides proptest generators for creating test data for property tests.

use northroot_engine::shapes::{ChunkScheme, DataShape, KeyFormat, RowValueRepr};
use proptest::prelude::*;

/// Generate a valid SHA-256 hash string in format `sha256:<64hex>`.
pub fn hash_strategy() -> impl Strategy<Value = String> {
    "[0-9a-f]{64}"
        .prop_map(|hex| format!("sha256:{}", hex))
}

/// Generate a ChunkScheme.
pub fn chunk_scheme_strategy() -> impl Strategy<Value = ChunkScheme> {
    prop_oneof![
        (1u64..=1_000_000u64).prop_map(|avg_size| ChunkScheme::CDC { avg_size }),
        (1u64..=1_000_000u64).prop_map(|size| ChunkScheme::Fixed { size }),
    ]
}

/// Generate a DataShape::ByteStream.
pub fn bytestream_strategy() -> impl Strategy<Value = DataShape> {
    (hash_strategy(), 0u64..=10_000_000u64, chunk_scheme_strategy())
        .prop_map(|(manifest_root, manifest_len, chunk_scheme)| {
            DataShape::ByteStream {
                manifest_root,
                manifest_len,
                chunk_scheme,
            }
        })
}

/// Generate a DataShape::RowMap.
pub fn rowmap_strategy() -> impl Strategy<Value = DataShape> {
    (
        hash_strategy(),
        0u64..=10_000_000u64,
        Just(KeyFormat::Sha256Hex),
        prop_oneof![
            Just(RowValueRepr::Number),
            Just(RowValueRepr::String),
            Just(RowValueRepr::Pointer),
        ],
    )
        .prop_map(|(merkle_root, row_count, key_fmt, value_repr)| {
            DataShape::RowMap {
                merkle_root,
                row_count,
                key_fmt,
                value_repr,
            }
        })
}

/// Generate any DataShape.
pub fn data_shape_strategy() -> impl Strategy<Value = DataShape> {
    prop_oneof![bytestream_strategy(), rowmap_strategy()]
}

/// Generate a valid code hash (with or without sha256: prefix).
pub fn code_hash_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        hash_strategy(),
        "[0-9a-f]{64}".prop_map(|hex| hex),
    ]
}

/// Generate a function name.
pub fn function_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,50}"
}

/// Generate a type name.
pub fn type_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_<>,\\s]{0,100}"
}

/// Generate a vector of type names.
pub fn type_names_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(type_name_strategy(), 0..=10)
}

/// Generate arbitrary bytes for testing.
pub fn bytes_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=100_000)
}

/// Generate a set of chunk identifiers (strings).
pub fn chunk_set_strategy() -> impl Strategy<Value = std::collections::HashSet<String>> {
    prop::collection::hash_set(hash_strategy(), 0..=1000)
}

/// Generate a JSON value for parameters.
pub fn json_value_strategy() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|i| serde_json::Value::Number(i.into())),
        any::<f64>().prop_map(|f| {
            serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }),
        "[a-zA-Z0-9_]{0,100}".prop_map(serde_json::Value::String),
    ];

    leaf.prop_recursive(
        8,   // max depth
        256, // max size
        10,  // items per collection
        |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..=10)
                    .prop_map(serde_json::Value::Array),
                prop::collection::hash_map(
                    "[a-zA-Z0-9_]{1,20}",
                    inner,
                    0..=10
                )
                    .prop_map(|map| {
                        serde_json::Value::Object(
                            map.into_iter()
                                .map(|(k, v)| (k, v))
                                .collect(),
                        )
                    }),
            ]
        },
    )
}

