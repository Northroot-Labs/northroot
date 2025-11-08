# Dagster Integration

## Overview

Northroot SDK integration with Dagster for asset materialization hooks and automatic receipt emission.

## Usage

```python
from dagster import asset
from northroot_sdk.dagster import NorthrootAsset

@asset
@NorthrootAsset(
    policy_ref="acme/reuse_thresholds@1",
    alpha=0.75
)
def my_partitioned_asset(context):
    # Asset materialization logic
    return processed_data
```

## Status

Implementation pending. Will wrap Dagster assets with Northroot instrumentation to emit receipts on materialization and track partition-level reuse.

