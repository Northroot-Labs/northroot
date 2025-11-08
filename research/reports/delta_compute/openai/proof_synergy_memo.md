# Proof Synergy Memo: Deterministic Serialization as Bridge

**Research Agent:** openai  
**Date:** 2025-11-08  
**Version:** 0.1  
**Namespace:** northroot.research.delta_compute

## Executive Summary

Deterministic serialization is the bridge between delta reuse and verifiable proofs. By combining CBOR deterministic encoding (RFC 8949), JSON Canonicalization Scheme (RFC 8785), FastCDC chunk IDs, and Bazel/DVC cache keys, Northroot receipts can cite: (a) the subset of content-defined chunks reused, (b) the α/ΔC pair for the operator, and (c) the deterministic digest that auditors recompute without access to proprietary data.

**Key Insight:** Canonical receipts become the public inputs to proof circuits without leaking side-channel variance, enabling verifiable delta compute both inside TEEs and in zk-proof experiments.

---

## 1. Deterministic Serialization Requirements

### 1.1 CBOR Deterministic Encoding (RFC 8949)

**Requirements:**
- Preferred argument sizes (no indefinite-length items)
- Lexicographically sorted map keys
- Deterministic encoding rules

**Purpose:** CBOR receipts can be hashed identically by every verifier, enabling cross-organizational verification.

**Implementation:**
- Enforce deterministic encoding in `engine/src/commitments.rs`
- Validate CBOR receipts against RFC 8949 rules
- Enable byte-for-byte verification

### 1.2 JSON Canonicalization Scheme (RFC 8785)

**Requirements:**
- I-JSON compliant payloads
- Deterministic property ordering
- ECMAScript-consistent number formatting

**Purpose:** JSON mirrors stay hashable, enabling deterministic proof roots.

**Implementation:**
- Implement JCS in `engine/src/commitments.rs`
- Validate JSON receipts against RFC 8785 rules
- Enable deterministic hash computation

---

## 2. Receipt Structure for Verifiable Delta Compute

### 2.1 Content-Defined Chunks

**FastCDC Chunk IDs:** Enable precise chunk-level reuse tracking.

**MinHash Sketches:** Enable quick overlap estimation without full set operations.

**Receipt Fields:**
- `execution.payload.chunk_ids`: FastCDC chunk identifiers
- `execution.payload.minhash_sketch`: Overlap estimation
- `spend.justification.overlap_j`: Measured Jaccard overlap

### 2.2 Operator Economics

**α (Incrementality):** Operator-specific efficiency factor for delta application.

**ΔC (Economic Delta):** Savings estimate from reuse: ΔC ≈ α · C_comp · J - C_id

**Receipt Fields:**
- `spend.justification.alpha`: Operator incrementality
- `spend.justification.c_id`: Identity/integration cost
- `spend.justification.c_comp`: Baseline compute cost
- `spend.justification.delta_c`: Economic delta (savings)

### 2.3 Deterministic Digest

**Proof Root:** Canonical hash of receipt payload (CBOR or JCS).

**Verification:** Auditors can recompute proof roots without proprietary data access.

**Receipt Fields:**
- `hash`: SHA-256 of canonical receipt (deterministic)
- `sig`: Cryptographic signature
- `execution.payload.roots`: Merkle roots for state commitments

---

## 3. Verifiable Delta Compute Workflow

### 3.1 Receipt Generation

1. **Chunk Identification:** FastCDC chunk IDs for content-defined chunking
2. **Overlap Estimation:** MinHash sketches for quick overlap comparison
3. **Reuse Decision:** J > C_id / (α · C_comp)
4. **Receipt Emission:** Deterministic CBOR/JCS with α, ΔC, chunk IDs

### 3.2 Cross-Organizational Verification

1. **Receipt Validation:** Verify CBOR/JCS deterministic encoding
2. **Proof Root Computation:** Recompute hash from canonical receipt
3. **Overlap Verification:** Validate MinHash sketches and chunk IDs
4. **Economic Verification:** Verify reuse decision rule (J > threshold)

### 3.3 TEE and ZK Proof Integration

**TEE (Trusted Execution Environments):**
- Deterministic binary blobs are crucial for TEE attestation
- Canonical receipts enable verifiable TEE execution
- Proof roots can be verified inside TEEs

**ZK Proofs (Zero-Knowledge):**
- Canonical receipts become public inputs to proof circuits
- No side-channel variance from non-deterministic serialization
- Enables privacy-preserving verification

---

## 4. Integration with Existing Systems

### 4.1 Bazel/DVC Cache Keys

**Pattern:** Package operator manifests with Bazel-style cache keys (hash of canonical inputs, operator id, α, ΔC).

**Benefits:**
- Leverage existing cache infrastructure
- Add verifiability to cache hits
- Transparent reuse decisions

### 4.2 FastCDC Chunk IDs

**Pattern:** Use FastCDC for content-defined chunking with 10× speedup over Rabin hashing.

**Benefits:**
- Efficient chunk-level reuse
- MinHash similarity for overlap estimation
- Aggressive chunk reuse without CPU burn

### 4.3 Delta Lake CDF

**Pattern:** Integrate with Delta Lake's change data feed for row-level change tracking.

**Benefits:**
- Precise partition-level reuse
- Verifiable ETL refresh decisions
- 30–45% compute savings

---

## 5. Proof Circuit Integration

### 5.1 Public Inputs

**Canonical Receipts:** Become public inputs to ZK proof circuits.

**Benefits:**
- No side-channel variance
- Deterministic proof generation
- Privacy-preserving verification

### 5.2 Private Inputs

**Proprietary Data:** Remains private, not included in receipts.

**Benefits:**
- Data privacy preserved
- Verifiable without data access
- Cross-organizational trust

### 5.3 Proof Verification

**Deterministic Digest:** Auditors can recompute proof roots without proprietary data.

**Benefits:**
- Trustless verification
- Cross-organizational auditability
- Compliance automation

---

## 6. Implementation Requirements

### 6.1 Engine Support

**Location:** `engine/src/commitments.rs`

**Requirements:**
- Accept deterministic CBOR blobs (RFC 8949)
- Optionally mirrored JCS JSON bodies
- Enable byte-for-byte verification

### 6.2 Receipt Schema

**Location:** `receipts/schemas/`

**Requirements:**
- CBOR deterministic encoding support
- JCS JSON mirror support
- Chunk ID and MinHash sketch fields

### 6.3 Validation

**Location:** `receipts/src/validation.rs`

**Requirements:**
- Validate CBOR against RFC 8949
- Validate JSON against RFC 8785
- Verify deterministic encoding

---

## 7. Benefits Summary

### 7.1 Verifiable Delta Compute

- **Deterministic Proof Roots:** Auditors can recompute without proprietary data
- **Cross-Organizational Verification:** Portable receipts across organizations
- **TEE Integration:** Deterministic binary blobs for TEE attestation

### 7.2 Privacy-Preserving Verification

- **ZK Proof Integration:** Canonical receipts as public inputs
- **No Side-Channel Variance:** Deterministic serialization eliminates variance
- **Trustless Markets:** Enable cross-org compute credit markets

### 7.3 Compliance & Audit

- **Eliminate Duplicate Reruns:** Portable receipts remove redundant compliance runs
- **Verifiable Reuse Decisions:** Economic justification in receipts
- **Audit Trail:** Complete history of reuse decisions

---

## 8. Conclusion

Deterministic serialization (CBOR/JCS) combined with FastCDC chunk IDs and operator economics (α, ΔC) enables verifiable delta compute that can be verified across organizations without proprietary data access. This unlocks:

1. **TEE Integration:** Deterministic binary blobs for trusted execution
2. **ZK Proof Integration:** Canonical receipts as public inputs
3. **Cross-Org Markets:** Trustless compute credit markets
4. **Compliance Automation:** Eliminate duplicate compliance reruns

**Next Steps:**
1. Implement CBOR/JCS deterministic encoding in engine
2. Integrate FastCDC chunk IDs
3. Enable cross-org receipt verification
4. Prototype TEE and ZK proof integration

---

**References:**
- RFC 8949: CBOR Deterministic Encoding
- RFC 8785: JSON Canonicalization Scheme
- FastCDC: Content-Defined Chunking
- See `bibliography.md` for full citations

