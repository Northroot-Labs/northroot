use crate::events::{AttestationEvent, AuthorizationEvent, CheckpointEvent, ExecutionEvent};
use crate::shared::Meter;
use northroot_canonical::{Canonicalizer, Digest, DigestAlg, Quantity, Timestamp};
use num_bigint::BigInt;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use std::collections::HashMap;

/// Domain separator for price index digest computation: `b"northroot:price-index:v1\0"`.
const PRICE_INDEX_DOMAIN_SEPARATOR: &[u8] = b"northroot:price-index:v1\0";

/// Computes the digest for a price index snapshot.
///
/// Formula: `sha256(domain_separator || canonical_bytes(snapshot))`
///
/// The snapshot must be serializable and will be canonicalized before hashing.
fn compute_price_index_digest(
    snapshot: &PriceIndexSnapshot,
    canonicalizer: &Canonicalizer,
) -> Result<Digest, String> {
    // Serialize to JSON Value
    let value: Value = serde_json::to_value(snapshot)
        .map_err(|e| format!("price index serialization failed: {}", e))?;

    // Canonicalize the JSON value
    let result = canonicalizer
        .canonicalize(&value)
        .map_err(|e| format!("price index canonicalization failed: {}", e))?;

    // Hash: domain_separator || canonical_bytes
    let mut hasher = Sha256::new();
    hasher.update(PRICE_INDEX_DOMAIN_SEPARATOR);
    hasher.update(&result.bytes);
    let hash_bytes = hasher.finalize();

    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash_bytes);
    Digest::new(DigestAlg::Sha256, b64).map_err(|e| format!("digest construction failed: {}", e))
}

/// Verification verdict: explicit outcome of verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationVerdict {
    /// Verification passed; all constraints satisfied.
    Ok,
    /// Authorization was denied.
    Denied,
    /// Constraint violation detected (e.g., exceeded bounds).
    Violation,
    /// Invalid evidence (missing, malformed, or inconsistent).
    Invalid,
}

/// Token type for price index entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// Input tokens.
    Input,
    /// Output tokens.
    Output,
}

/// Token price entry for conversion from tokens to USD.
#[derive(Debug, Clone, Serialize)]
pub struct TokenPrice {
    /// Model identifier (e.g., "gpt-4", "claude-3-opus").
    pub model_id: String,
    /// Provider identifier (e.g., "openai", "anthropic").
    pub provider: String,
    /// Token type (input or output).
    pub token_type: TokenType,
    /// Price per token in USD.
    pub price_per_token: Quantity,
    /// Timestamp when this price was valid.
    pub timestamp: Timestamp,
}

/// Unit rate entry for conversion from compute/storage units to USD.
#[derive(Debug, Clone, Serialize)]
pub struct UnitRate {
    /// Unit identifier (e.g., "compute.seconds", "storage.bytes").
    pub unit: String,
    /// Price per unit in USD.
    pub price_per_unit: Quantity,
    /// Timestamp when this rate was valid.
    pub timestamp: Timestamp,
}

/// Price index snapshot for deterministic conversion.
#[derive(Debug, Clone, Serialize)]
pub struct PriceIndexSnapshot {
    /// Timestamp when this snapshot was created.
    pub as_of: Timestamp,
    /// Token prices by model and type.
    pub token_prices: Vec<TokenPrice>,
    /// Optional compute rates.
    pub compute_rates: Option<Vec<UnitRate>>,
    /// Optional storage rates.
    pub storage_rates: Option<Vec<UnitRate>>,
}

/// Conversion context for cross-unit meter comparisons.
#[derive(Debug, Clone)]
pub struct ConversionContext {
    /// Price index snapshot for conversions.
    pub snapshot: PriceIndexSnapshot,
}

impl ConversionContext {
    /// Creates a new conversion context from a price index snapshot.
    pub fn new(snapshot: PriceIndexSnapshot) -> Self {
        Self { snapshot }
    }

    /// Computes the digest of the price index snapshot.
    ///
    /// Formula: `sha256(domain_separator || canonical_bytes(snapshot))`
    ///
    /// Uses domain separator `b"northroot:price-index:v1\0"` to distinguish
    /// from event IDs.
    pub fn compute_snapshot_digest(&self, canonicalizer: &Canonicalizer) -> Result<Digest, String> {
        compute_price_index_digest(&self.snapshot, canonicalizer)
    }
}

/// Verifier for checking event consistency and bounds.
pub struct Verifier {
    canonicalizer: Canonicalizer,
}

/// Result of comparing two quantities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComparisonResult {
    /// Used quantity is less than or equal to cap.
    WithinBounds,
    /// Used quantity exceeds cap.
    ExceedsBounds,
    /// Comparison cannot be performed (incompatible types or missing evidence).
    Invalid,
}

impl Verifier {
    /// Creates a new verifier with the given canonicalizer.
    pub fn new(canonicalizer: Canonicalizer) -> Self {
        Self { canonicalizer }
    }

    /// Compares two quantities deterministically.
    ///
    /// Returns `Invalid` for mixed types (no implicit coercion).
    fn compare_quantities(&self, used: &Quantity, cap: &Quantity) -> ComparisonResult {
        match (used, cap) {
            // Integer comparison
            (Quantity::Int { v: used_v }, Quantity::Int { v: cap_v }) => {
                match self.cmp_int_strings(used_v, cap_v) {
                    Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) => {
                        ComparisonResult::WithinBounds
                    }
                    Some(std::cmp::Ordering::Greater) => ComparisonResult::ExceedsBounds,
                    None => ComparisonResult::Invalid,
                }
            }
            // Decimal comparison (normalize to same scale)
            (
                Quantity::Dec {
                    m: used_m,
                    s: used_s,
                },
                Quantity::Dec { m: cap_m, s: cap_s },
            ) => match self.cmp_dec(used_m, *used_s, cap_m, *cap_s) {
                Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) => {
                    ComparisonResult::WithinBounds
                }
                Some(std::cmp::Ordering::Greater) => ComparisonResult::ExceedsBounds,
                None => ComparisonResult::Invalid,
            },
            // Rational comparison (cross-multiply)
            (
                Quantity::Rat {
                    n: used_n,
                    d: used_d,
                },
                Quantity::Rat { n: cap_n, d: cap_d },
            ) => match self.cmp_rat(used_n, used_d, cap_n, cap_d) {
                Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) => {
                    ComparisonResult::WithinBounds
                }
                Some(std::cmp::Ordering::Greater) => ComparisonResult::ExceedsBounds,
                None => ComparisonResult::Invalid,
            },
            // F64 comparison (bitwise comparison)
            (Quantity::F64 { bits: used_bits }, Quantity::F64 { bits: cap_bits }) => {
                if used_bits == cap_bits {
                    ComparisonResult::WithinBounds
                } else {
                    // For F64, we'd need to decode bits and compare, but for now
                    // we'll treat exact match as within bounds, otherwise invalid
                    // (proper F64 comparison would require decoding IEEE-754)
                    ComparisonResult::Invalid
                }
            }
            // Mixed types: no coercion allowed
            _ => ComparisonResult::Invalid,
        }
    }

    /// Compares two integer strings.
    fn cmp_int_strings(&self, a: &str, b: &str) -> Option<std::cmp::Ordering> {
        // Handle zero case
        if a == "0" && b == "0" {
            return Some(std::cmp::Ordering::Equal);
        }

        // Parse signs
        let (a_sign, a_abs) = if let Some(stripped) = a.strip_prefix('-') {
            (-1, stripped)
        } else {
            (1, a)
        };
        let (b_sign, b_abs) = if let Some(stripped) = b.strip_prefix('-') {
            (-1, stripped)
        } else {
            (1, b)
        };

        // Compare signs first
        match a_sign.cmp(&b_sign) {
            std::cmp::Ordering::Less => return Some(std::cmp::Ordering::Less),
            std::cmp::Ordering::Greater => return Some(std::cmp::Ordering::Greater),
            std::cmp::Ordering::Equal => {}
        }

        // Same sign, compare absolute values
        let abs_cmp = match a_abs.len().cmp(&b_abs.len()) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => {
                // Same length, lexicographic comparison works for same-length numbers
                a_abs.cmp(b_abs)
            }
        };

        // If negative, reverse the comparison
        if a_sign < 0 {
            Some(abs_cmp.reverse())
        } else {
            Some(abs_cmp)
        }
    }

    /// Compares two decimal quantities by normalizing to the same scale.
    fn cmp_dec(
        &self,
        used_m: &str,
        used_s: u32,
        cap_m: &str,
        cap_s: u32,
    ) -> Option<std::cmp::Ordering> {
        // Normalize to the maximum scale
        let max_scale = used_s.max(cap_s);
        let used_normalized = self.normalize_dec_scale(used_m, used_s, max_scale)?;
        let cap_normalized = self.normalize_dec_scale(cap_m, cap_s, max_scale)?;

        // Compare normalized mantissas
        self.cmp_int_strings(&used_normalized, &cap_normalized)
    }

    /// Normalizes a decimal mantissa to a target scale.
    fn normalize_dec_scale(
        &self,
        mantissa: &str,
        from_scale: u32,
        to_scale: u32,
    ) -> Option<String> {
        if to_scale < from_scale {
            // Would require truncation, which we don't allow
            return None;
        }
        if to_scale == from_scale {
            return Some(mantissa.to_string());
        }

        // Add zeros to increase scale
        let scale_diff = to_scale - from_scale;
        Some(format!("{}{}", mantissa, "0".repeat(scale_diff as usize)))
    }

    /// Compares two rational quantities by cross-multiplying.
    fn cmp_rat(
        &self,
        used_n: &str,
        used_d: &str,
        cap_n: &str,
        cap_d: &str,
    ) -> Option<std::cmp::Ordering> {
        // Cross-multiply: used_n * cap_d vs cap_n * used_d
        // We need to compare used_n * cap_d with cap_n * used_d
        // For simplicity, we'll parse to i128 if possible, otherwise use string arithmetic

        // Try parsing as integers first
        if let (Ok(used_n_i), Ok(used_d_i), Ok(cap_n_i), Ok(cap_d_i)) = (
            used_n.parse::<i128>(),
            used_d.parse::<i128>(),
            cap_n.parse::<i128>(),
            cap_d.parse::<i128>(),
        ) {
            // Cross-multiply: used_n * cap_d vs cap_n * used_d
            let left = used_n_i.checked_mul(cap_d_i)?;
            let right = cap_n_i.checked_mul(used_d_i)?;
            return Some(left.cmp(&right));
        }

        // For large numbers, we'd need big integer arithmetic
        // For now, return None to indicate we can't compare
        None
    }

    /// Verifies an authorization event's structure and computes its event ID.
    ///
    /// Returns the computed event ID and a verdict.
    pub fn verify_authorization(
        &self,
        event: &AuthorizationEvent,
    ) -> Result<(Digest, VerificationVerdict), String> {
        // Compute event ID from canonical bytes
        let computed_id = crate::event_id::compute_event_id(event, &self.canonicalizer)
            .map_err(|e| format!("event ID computation failed: {}", e))?;

        // Verify event_id matches computed
        if event.event_id != computed_id {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Verify required fields
        if event.event_type != "authorization" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }
        if event.event_version != "1" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // If decision is deny, verdict is Denied
        if event.decision == crate::events::Decision::Deny {
            return Ok((computed_id, VerificationVerdict::Denied));
        }

        // If decision is allow, verify bounds are present
        match &event.authorization {
            crate::events::AuthorizationKind::Grant { bounds } => {
                if bounds.allowed_tools.is_empty() {
                    return Ok((computed_id, VerificationVerdict::Invalid));
                }
                if bounds.meter_caps.is_empty() {
                    return Ok((computed_id, VerificationVerdict::Invalid));
                }
            }
            crate::events::AuthorizationKind::Action { action, .. } => {
                if action.tool_params_digest.alg != DigestAlg::Sha256 {
                    return Ok((computed_id, VerificationVerdict::Invalid));
                }
            }
        }

        Ok((computed_id, VerificationVerdict::Ok))
    }

    /// Verifies a checkpoint event's structure and computes its event ID.
    ///
    /// Returns the computed event ID and a verdict.
    pub fn verify_checkpoint(
        &self,
        event: &CheckpointEvent,
    ) -> Result<(Digest, VerificationVerdict), String> {
        let computed_id = crate::event_id::compute_event_id(event, &self.canonicalizer)
            .map_err(|e| format!("event ID computation failed: {}", e))?;

        if event.event_id != computed_id {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        if event.event_type != "checkpoint" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }
        if event.event_version != "1" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Schema: if merkle_root exists, window is required.
        if event.merkle_root.is_some() && event.window.is_none() {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Basic coherence: if both bounds are present, start <= end.
        if let Some(w) = &event.window {
            if let (Some(start), Some(end)) = (w.start_height, w.end_height) {
                if start > end {
                    return Ok((computed_id, VerificationVerdict::Invalid));
                }
            }
        }

        Ok((computed_id, VerificationVerdict::Ok))
    }

    /// Verifies an attestation event's structure and computes its event ID.
    ///
    /// Returns the computed event ID and a verdict.
    pub fn verify_attestation(
        &self,
        event: &AttestationEvent,
    ) -> Result<(Digest, VerificationVerdict), String> {
        let computed_id = crate::event_id::compute_event_id(event, &self.canonicalizer)
            .map_err(|e| format!("event ID computation failed: {}", e))?;

        if event.event_id != computed_id {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        if event.event_type != "attestation" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }
        if event.event_version != "1" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Schema: 1..=16 signatures.
        if event.signatures.is_empty() || event.signatures.len() > 16 {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        for sig in &event.signatures {
            if sig.alg.is_empty() || sig.alg.len() > 64 {
                return Ok((computed_id, VerificationVerdict::Invalid));
            }
            if sig.key_id.is_empty() || sig.key_id.len() > 256 {
                return Ok((computed_id, VerificationVerdict::Invalid));
            }
            // Base64url-no-pad: 16..=4096 chars, [A-Za-z0-9_-]+
            if sig.sig.len() < 16 || sig.sig.len() > 4096 {
                return Ok((computed_id, VerificationVerdict::Invalid));
            }
            if !sig
                .sig
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Ok((computed_id, VerificationVerdict::Invalid));
            }
        }

        Ok((computed_id, VerificationVerdict::Ok))
    }

    /// Verifies an execution event and its linkage to authorization.
    ///
    /// Requires the corresponding authorization event to be provided.
    ///
    /// If `conversion` is provided and `exec.pricing_snapshot_digest` is present,
    /// validates that the digest matches the snapshot in the conversion context.
    pub fn verify_execution(
        &self,
        exec: &ExecutionEvent,
        auth: &AuthorizationEvent,
        conversion: Option<&ConversionContext>,
    ) -> Result<(Digest, VerificationVerdict), String> {
        // Compute event ID
        let computed_id = crate::event_id::compute_event_id(exec, &self.canonicalizer)
            .map_err(|e| format!("event ID computation failed: {}", e))?;

        // Verify event_id matches
        if exec.event_id != computed_id {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Verify required fields
        if exec.event_type != "execution" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }
        if exec.event_version != "1" {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Verify linkage to authorization
        if exec.auth_event_id != auth.event_id {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Verify intent digest matches
        if exec.intents.intent_digest != auth.intents.intent_digest {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Verify authorization decision
        if auth.decision == crate::events::Decision::Deny {
            // Execution should not occur if authorization was denied
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        // Validate pricing snapshot digest if both are present
        if let (Some(event_digest), Some(ctx)) = (exec.pricing_snapshot_digest.as_ref(), conversion)
        {
            let computed_digest = ctx
                .compute_snapshot_digest(&self.canonicalizer)
                .map_err(|e| format!("failed to compute snapshot digest: {}", e))?;
            if *event_digest != computed_digest {
                return Ok((computed_id, VerificationVerdict::Invalid));
            }
        }

        // Verify bounds against usage
        let verdict = match &auth.authorization {
            crate::events::AuthorizationKind::Grant { bounds } => {
                self.check_meter_bounds(&exec.meter_used, &bounds.meter_caps, exec, conversion)
            }
            crate::events::AuthorizationKind::Action { action, .. } => {
                // Verify tool name matches
                if exec.tool_name != action.tool_name {
                    return Ok((computed_id, VerificationVerdict::Invalid));
                }
                // Check meter reservation if present
                if let Some(reservation) = &action.meter_reservation {
                    self.check_meter_bounds(&exec.meter_used, reservation, exec, conversion)
                } else {
                    VerificationVerdict::Ok
                }
            }
        };

        // Verify failure outcome has error code
        if exec.outcome == crate::events::Outcome::Failure && exec.error_code.is_none() {
            return Ok((computed_id, VerificationVerdict::Invalid));
        }

        Ok((computed_id, verdict))
    }

    /// Checks if meter usage exceeds caps with optional conversion context.
    ///
    /// Constraints:
    /// - Same-unit comparisons are REQUIRED (must compare deterministically)
    /// - Cross-unit conversions are OPTIONAL (only if conversion context provided)
    /// - If cap is in USD and usage is in other units, conversion context is REQUIRED
    fn check_meter_bounds(
        &self,
        used: &[Meter],
        caps: &[Meter],
        exec: &ExecutionEvent,
        conversion: Option<&ConversionContext>,
    ) -> VerificationVerdict {
        // Build a map of caps by unit
        let mut cap_map: HashMap<String, &Quantity> = HashMap::new();
        for cap in caps {
            cap_map.insert(cap.unit.clone(), &cap.amount);
        }

        let mut has_violation = false;
        let mut has_missing_evidence = false;

        // Check each used meter against its cap
        for meter in used {
            // REQUIRED: Direct unit match
            if let Some(cap_amount) = cap_map.get(&meter.unit) {
                match self.compare_quantities(&meter.amount, cap_amount) {
                    ComparisonResult::WithinBounds => {
                        // OK
                    }
                    ComparisonResult::ExceedsBounds => {
                        has_violation = true;
                    }
                    ComparisonResult::Invalid => {
                        has_missing_evidence = true;
                    }
                }
                continue;
            }

            // OPTIONAL: Cross-unit conversion (only if context provided)
            // If cap is in USD, we need conversion
            if cap_map.contains_key("usd") {
                if let Some(ctx) = conversion {
                    // Try to convert this meter to USD
                    if let Some(converted_usd) = self.convert_to_usd(meter, exec, ctx) {
                        if let Some(usd_cap) = cap_map.get("usd") {
                            match self.compare_quantities(&converted_usd, usd_cap) {
                                ComparisonResult::WithinBounds => {
                                    // OK
                                }
                                ComparisonResult::ExceedsBounds => {
                                    has_violation = true;
                                }
                                ComparisonResult::Invalid => {
                                    has_missing_evidence = true;
                                }
                            }
                        }
                    } else {
                        // Conversion failed - if we have a USD cap, this is missing evidence
                        has_missing_evidence = true;
                    }
                } else {
                    // USD cap exists but no conversion context provided
                    has_missing_evidence = true;
                }
            }
            // If no USD cap and no direct match, skip (optional check)
        }

        if has_violation {
            VerificationVerdict::Violation
        } else if has_missing_evidence {
            VerificationVerdict::Invalid
        } else {
            VerificationVerdict::Ok
        }
    }

    /// Converts a meter to USD using price index snapshot.
    ///
    /// Returns None if conversion not available (missing model/provider info or price entry).
    fn convert_to_usd(
        &self,
        meter: &Meter,
        exec: &ExecutionEvent,
        ctx: &ConversionContext,
    ) -> Option<Quantity> {
        // Check if this is a token unit
        if meter.unit.starts_with("tokens.") {
            // Extract token type from unit (e.g., "tokens.input" -> Input)
            let token_type = if meter.unit == "tokens.input" {
                TokenType::Input
            } else if meter.unit == "tokens.output" {
                TokenType::Output
            } else {
                return None;
            };

            // Get model and provider from execution event
            let model_id = exec.model_id.as_ref()?;
            let provider = exec.provider.as_ref()?;

            // Find matching price entry
            let _price_entry = ctx.snapshot.token_prices.iter().find(|p| {
                p.model_id == *model_id && p.provider == *provider && p.token_type == token_type
            })?;

            let price_entry = ctx.snapshot.token_prices.iter().find(|p| {
                p.model_id == *model_id && p.provider == *provider && p.token_type == token_type
            })?;

            // Convert: tokens * price_per_token = usd
            self.mul_quantities(&meter.amount, &price_entry.price_per_token)
        } else if meter.unit == "compute.seconds" || meter.unit == "storage.bytes" {
            // Look up unit rate
            let rates = if meter.unit == "compute.seconds" {
                ctx.snapshot.compute_rates.as_ref()?
            } else {
                ctx.snapshot.storage_rates.as_ref()?
            };

            let rate_entry = rates.iter().find(|r| r.unit == meter.unit)?;

            // Convert: units * price_per_unit = usd
            self.mul_quantities(&meter.amount, &rate_entry.price_per_unit)
        } else {
            // Unknown unit type
            None
        }
    }

    /// Multiplies two quantities. Supported:
    /// - Int * Int -> Int
    /// - Int * Dec -> Dec
    /// - Dec * Int -> Dec
    /// - Dec * Dec -> Dec (scale sums)
    ///
    /// Returns None for unsupported combos or overflow.
    fn mul_quantities(&self, a: &Quantity, b: &Quantity) -> Option<Quantity> {
        match (a, b) {
            (Quantity::Int { v: av }, Quantity::Int { v: bv }) => {
                let left = BigInt::parse_bytes(av.as_bytes(), 10)?;
                let right = BigInt::parse_bytes(bv.as_bytes(), 10)?;
                let prod = left * right;
                Some(Quantity::Int {
                    v: prod.to_string(),
                })
            }
            (Quantity::Int { v: av }, Quantity::Dec { m: bm, s: bs })
            | (Quantity::Dec { m: bm, s: bs }, Quantity::Int { v: av }) => {
                let left = BigInt::parse_bytes(av.as_bytes(), 10)?;
                let right = BigInt::parse_bytes(bm.as_bytes(), 10)?;
                let prod = left * right;
                Some(Quantity::Dec {
                    m: prod.to_string(),
                    s: *bs,
                })
            }
            (Quantity::Dec { m: am, s: as_ }, Quantity::Dec { m: bm, s: bs }) => {
                let left = BigInt::parse_bytes(am.as_bytes(), 10)?;
                let right = BigInt::parse_bytes(bm.as_bytes(), 10)?;
                let prod = left * right;
                let scale = as_.saturating_add(*bs);
                Some(Quantity::Dec {
                    m: prod.to_string(),
                    s: scale,
                })
            }
            _ => None,
        }
    }
}
