# CANONICALIZATION.md

## 0. Purpose

Northroot requires **deterministic, cross-language canonical bytes** for hashing,
identifiers, receipts, and offline verification.

This document defines:

- The canonical JSON profile used for all hashes and IDs
- Lossless numeric representation rules (fixed-point by default)
- Serialization and deserialization invariants
- Hygiene classification for non-conforming inputs

This document is **normative** for the current canonicalization profile.

---

## 1. Canonical JSON Profile

### 1.1 Base standard

Northroot canonical JSON MUST follow **RFC 8785 (JSON Canonicalization Scheme)** for:

- Object member ordering (lexicographic by Unicode code point)
- Whitespace handling (no insignificant whitespace)
- String escaping rules
- UTF-8 encoding of canonical bytes

### 1.2 Additional Northroot constraints

1. **Duplicate object keys are forbidden**

   If duplicate keys are observed in input, canonicalization MUST fail with
   `HygieneStatus::Invalid` and warning `DuplicateKeys`.

2. **UTF-8 only**

   Canonical bytes MUST be UTF-8. Non-UTF8 inputs MUST be rejected
   (`HygieneStatus::Invalid`).

3. **No NaN or Infinity**

   Non-finite numbers are not valid JSON and MUST be rejected if encountered via
   non-standard parsers.

4. **Arrays are order-significant**

   Canonicalization MUST NOT reorder arrays.

---

## 2. Numeric Model

### 2.1 Core invariant: lossless by default

Northroot protocol numerics MUST NOT be represented as JSON numbers.

Instead, numeric values are represented as **explicitly typed numeric objects** to ensure:

- Lossless representation across languages
- Deterministic canonical bytes
- Explicit scale and intent
- Avoidance of floating-point and formatting ambiguity

### 2.2 Forbidden: floating-point protocol numerics

JSON numbers MUST NOT be used for quantities that carry meaning for:

- Authorization or policy checks
- Cost or spend accounting
- Limits, caps, or thresholds
- Measurement or verification
- Identifiers or counters

If a JSON number appears in a field defined as a numeric quantity, canonicalization MUST
fail with `HygieneStatus::Invalid`, unless the schema explicitly permits a lossy mode.

---

## 3. Standard Quantity Types

All numeric quantities are encoded as JSON objects with fixed field names.

### 3.1 Fixed-point decimal (`Dec`)

Used for money, prices, rates, thresholds, and most measured quantities.

**Encoding**
```json
{ "t": "dec", "m": "12345", "s": 2 }

Where:
	•	t = "dec" (type tag)
	•	m = base-10 signed integer mantissa encoded as a string
	•	s = non-negative integer scale
Value = m × 10^-s

Examples

{ "t": "dec", "m": "1234", "s": 2 }    // 12.34
{ "t": "dec", "m": "-1",   "s": 3 }    // -0.001

Constraints
	•	m MUST match ^-?[0-9]+$
	•	m MUST be minimal:
	•	no leading zeros
	•	"0" is the only zero representation
	•	"-0" is forbidden
	•	s MUST be an integer within the allowed scale range
	•	default maximum: 0..=18
	•	larger values require explicit schema permission
	•	Mantissa length MUST NOT exceed the allowed digit bound
	•	default maximum: 39 decimal digits
	•	Values exceeding scale or mantissa bounds MUST be rejected
(no rounding, truncation, or clipping)

Canonicalization MUST NOT fold trailing zeros into scale unless explicitly required
by schema.

⸻

3.2 Integer (Int)

Used for counts, discrete quantities, and integral values.

Encoding

{ "t": "int", "v": "9223372036854775807" }

Constraints
	•	v MUST match ^-?[0-9]+$
	•	No leading zeros except "0"
	•	"-0" is forbidden
	•	Range MAY be constrained by schema (e.g., 64-bit), but encoding supports big integers

⸻

3.3 Rational (Rat) (optional)

Used for exact ratios or fractions.

Encoding

{ "t": "rat", "n": "1", "d": "3" }

Constraints
	•	d MUST be a positive integer
	•	n and d MUST be reduced (gcd(n, d) = 1)
	•	"-0" is forbidden in n

⸻

3.4 Binary float (F32 / F64) — explicit, lossy, opt-in only

Used only where binary floating-point semantics are inherent
(e.g., model weights, embeddings).

Encoding

{ "t": "f64", "bits": "3ff0000000000000" }

Constraints
	•	Allowed only if schema explicitly permits
	•	bits MUST be the exact IEEE-754 bit pattern encoded as lowercase hex
	•	Canonicalization MUST NOT convert between numeric forms
	•	NaN payloads MAY be allowed only if schema permits

⸻

4. Schema-Level Typing Rules

Protocol fields that represent numeric quantities MUST be declared as one of:
	•	Dec
	•	Int
	•	Rat
	•	F32 / F64 (explicitly lossy)

A field MUST NOT accept both JSON numbers and quantity objects.

If flexibility is required, it MUST be expressed via an explicit tagged union
(e.g., oneOf).

⸻

5. Serialization Rules

5.1 Canonicalization pipeline
	1.	Parse input as a JSON object graph
	2.	Validate structural constraints (UTF-8, no duplicate keys)
	3.	Validate schema typing (no JSON numbers for quantity fields)
	4.	Validate numeric bounds (scale and mantissa limits)
	5.	Serialize using RFC 8785 rules
	6.	Output canonical UTF-8 bytes

5.2 No semantic rewriting

Canonicalization MUST NOT:
	•	Trim or normalize strings
	•	Normalize Unicode (unless a future profile explicitly requires it)
	•	Coerce numeric types
	•	Reorder arrays
	•	Drop unknown fields

Canonicalization exists to produce deterministic bytes, not to clean or reinterpret data.

⸻

6. Deserialization Rules

6.1 Strict mode (default)

Strict mode MUST be used for:
	•	Hashing
	•	Receipts
	•	Verification
	•	Policy evaluation

Rules:
	•	Reject unknown fields unless schema explicitly allows them
	•	Reject JSON numbers for quantity fields
	•	Reject duplicate keys
	•	Reject non-minimal integer strings
	•	Reject -0 encodings
	•	Reject values exceeding scale or mantissa bounds

6.2 Permissive mode (ingestion only)

Permissive mode MAY:
	•	Accept JSON numbers and convert to Dec or Int only if conversion is exact
	•	Emit HygieneStatus::Lossy with appropriate warnings

Canonical bytes used for hashing MUST always come from strict-mode semantics.

⸻

7. Hygiene Report Integration

Canonicalization MUST produce a HygieneReport containing:
	•	status: Ok | Lossy | Ambiguous | Invalid
	•	warnings: stable warning codes
	•	metrics: counts (duplicate keys, numeric coercions, bound violations)
	•	profile_id: hash of this canonicalization profile and numeric rules

Policy gates MAY require status == Ok for cost-bearing or irreversible actions.

⸻

8. Versioning and Compatibility

The canonicalization profile is versioned.

Any change to:
	•	Canonical JSON rules
	•	Numeric encodings
	•	Bounds or normalization behavior
	•	Allowed numeric types

is a breaking change and requires a new profile version and profile_id.

⸻

9. Test Vectors

The repository MUST include golden test vectors covering:
	•	Object key ordering
	•	String escaping
	•	Quantity minimality
	•	Scale and mantissa bound enforcement
	•	Duplicate key rejection
	•	JSON number rejection in strict mode
	•	Floating-point bit determinism (if enabled)

⸻

Notes
	•	Money is a Dec with currency expressed separately
	•	Token counts are Int
	•	Rates, thresholds, and prices are Dec or Rat
	•	Physical quantities are Dec with explicit unit metadata
	•	Floating-point values are allowed only via explicit bit encoding

Core invariant: No silent lossy conversion.