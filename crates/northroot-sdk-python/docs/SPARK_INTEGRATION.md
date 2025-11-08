# Spark Integration

## Overview

Northroot SDK integration with Apache Spark for receipt emission and reuse tracking.

## Usage

```python
from pyspark.sql import SparkSession
from northroot_sdk.spark import DeltaUDF, emit_receipt

spark = SparkSession.builder.appName("DeltaCompute").getOrCreate()

# Register delta-aware UDF
@DeltaUDF(operator="incremental_sum", alpha=0.92)
def sum_values(values):
    return sum(values)

# Use in Spark pipeline
df = spark.read.parquet("input.parquet")
result = df.groupBy("account_id").agg(
    sum_values("amount").alias("total")
)

# Emit receipt for entire Spark job
receipt = emit_receipt(
    execution=result,
    method_ref="acme/cost_attribution@1.0.0",
    policy_ref="acme/reuse_thresholds@1"
)
```

## Status

Implementation pending. Will integrate with Spark job execution lifecycle to emit receipts per job with span commitments.

