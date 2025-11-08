# Northroot Python SDK

Python SDK for Northroot delta compute and verifiable receipt system.

## Installation

```bash
pip install northroot-sdk
```

## Usage

### Decorator Pattern

```python
from northroot_sdk import delta_compute

@delta_compute(
    operator="partition_rows",
    alpha=0.95,
    cost_model={"c_id": 0.1, "c_comp": 10.0}
)
def process_partition(data: pd.DataFrame) -> pd.DataFrame:
    # Existing processing logic
    return transformed_data

# Receipt automatically emitted with spend.justification
```

### Context Manager Pattern

```python
from northroot_sdk import DeltaContext

with DeltaContext(
    operator="group_by_account",
    policy_ref="acme/reuse_thresholds@1"
) as ctx:
    result = df.groupby("account_id").sum()
    ctx.emit_receipt(result)
```

### Operator Wrapper Pattern

```python
from northroot_sdk import DeltaOperator

partition_op = DeltaOperator(
    name="partition_rows",
    alpha=0.95,
    strategy="partition"
)

result, receipt = partition_op.execute(
    input_data=df,
    prev_state=prev_state_hash
)
```

## Integration

- **Spark**: Custom UDFs with receipt emission
- **Dagster**: Asset materialization hooks
- **Pandas/Dask**: Decorator-based instrumentation

## Status

This SDK is in development. The Rust engine provides the core functionality, and this SDK will provide Python bindings via PyO3 or CFFI.

