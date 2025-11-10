//! Identity root computation tests.

use northroot_receipts::{identity_root_from_identities, IdentityRecord};

#[test]
fn test_identity_root_two_identities() {
    let identities = vec![
        IdentityRecord {
            did: "did:key:zNorthroot".to_string(),
            kid: "did:key:zNorthroot#key-1".to_string(),
            role: Some("executor".to_string()),
            tenant: None,
        },
        IdentityRecord {
            did: "did:key:zTest".to_string(),
            kid: "did:key:zTest#key-1".to_string(),
            role: Some("submitter".to_string()),
            tenant: Some("urn:tenant:abc".to_string()),
        },
    ];

    let root = identity_root_from_identities(identities.into_iter());

    // Verify format
    assert!(root.starts_with("sha256:"));
    assert_eq!(root.len(), 71); // sha256: + 64 hex chars

    // Verify deterministic: same identities produce same root
    let identities2 = vec![
        IdentityRecord {
            did: "did:key:zNorthroot".to_string(),
            kid: "did:key:zNorthroot#key-1".to_string(),
            role: Some("executor".to_string()),
            tenant: None,
        },
        IdentityRecord {
            did: "did:key:zTest".to_string(),
            kid: "did:key:zTest#key-1".to_string(),
            role: Some("submitter".to_string()),
            tenant: Some("urn:tenant:abc".to_string()),
        },
    ];
    let root2 = identity_root_from_identities(identities2.into_iter());
    assert_eq!(root, root2);
}

#[test]
fn test_identity_root_remove_one() {
    let identity1 = IdentityRecord {
        did: "did:key:zNorthroot".to_string(),
        kid: "did:key:zNorthroot#key-1".to_string(),
        role: Some("executor".to_string()),
        tenant: None,
    };

    let identity2 = IdentityRecord {
        did: "did:key:zTest".to_string(),
        kid: "did:key:zTest#key-1".to_string(),
        role: Some("submitter".to_string()),
        tenant: None,
    };

    // Root with both identities
    let root_both =
        identity_root_from_identities([identity1.clone(), identity2.clone()].into_iter());

    // Root with only first identity
    let root_one = identity_root_from_identities([identity1].into_iter());

    // Roots should be different
    assert_ne!(root_both, root_one);
}

#[test]
fn test_identity_root_reorder() {
    let identity1 = IdentityRecord {
        did: "did:key:zA".to_string(),
        kid: "did:key:zA#key-1".to_string(),
        role: None,
        tenant: None,
    };

    let identity2 = IdentityRecord {
        did: "did:key:zB".to_string(),
        kid: "did:key:zB#key-1".to_string(),
        role: None,
        tenant: None,
    };

    // Root with order [A, B]
    let root_ab = identity_root_from_identities([identity1.clone(), identity2.clone()].into_iter());

    // Root with order [B, A] - should be same (sorted internally)
    let root_ba = identity_root_from_identities([identity2, identity1].into_iter());

    // Roots should be identical (order-independent)
    assert_eq!(root_ab, root_ba);
}

#[test]
fn test_identity_root_empty() {
    let identities: Vec<IdentityRecord> = vec![];
    let root = identity_root_from_identities(identities.into_iter());

    // Empty tree root = H(0x00 || "")
    assert!(root.starts_with("sha256:"));
    assert_eq!(root.len(), 71);

    // Should be deterministic
    let identities2: Vec<IdentityRecord> = vec![];
    let root2 = identity_root_from_identities(identities2.into_iter());
    assert_eq!(root, root2);
}

#[test]
fn test_identity_root_single() {
    let identity = IdentityRecord {
        did: "did:key:zNorthroot".to_string(),
        kid: "did:key:zNorthroot#key-1".to_string(),
        role: Some("executor".to_string()),
        tenant: None,
    };

    let root = identity_root_from_identities([identity].into_iter());

    // Single identity: leaf hash is the root
    assert!(root.starts_with("sha256:"));
    assert_eq!(root.len(), 71);
}

#[test]
fn test_identity_root_three_identities() {
    // Test odd number promotion
    let identities = vec![
        IdentityRecord {
            did: "did:key:zA".to_string(),
            kid: "did:key:zA#key-1".to_string(),
            role: None,
            tenant: None,
        },
        IdentityRecord {
            did: "did:key:zB".to_string(),
            kid: "did:key:zB#key-1".to_string(),
            role: None,
            tenant: None,
        },
        IdentityRecord {
            did: "did:key:zC".to_string(),
            kid: "did:key:zC#key-1".to_string(),
            role: None,
            tenant: None,
        },
    ];

    let root = identity_root_from_identities(identities.clone().into_iter());

    // Verify format and determinism
    assert!(root.starts_with("sha256:"));
    assert_eq!(root.len(), 71);

    let root2 = identity_root_from_identities(identities.into_iter());
    assert_eq!(root, root2);
}
