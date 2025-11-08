# Operator Manifest v1

**Purpose:** Declaratively define an atomic, deterministic transformation unit.

An operator manifest describes:
- **tool_id / tool_version** — stable identity.
- **param_schema** — schema for configurable parameters.
- **input_shape / output_shape** — JSON shapes for I/O contracts.
- **x_numeric / x_io** — numeric and I/O policy pins.
- **checker_refs** — optional verification references.

Example:
```json
{
  "schema_version": "operator.v1",
  "tool_id": "acme.frame.inc_sum",
  "tool_version": "1",
  "param_schema": { "type": "object", "properties": { "column": {"type":"string"} } },
  "input_shape": {...},
  "output_shape": {...}
}