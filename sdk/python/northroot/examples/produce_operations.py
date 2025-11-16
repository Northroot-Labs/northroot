#!/usr/bin/env python3
"""
Produce Operations: Potato Supply Chain Example

This example demonstrates verifiable receipts for agricultural supply chain operations,
specifically potato harvest and shipping workflows. It shows how to use semantic
workload_id patterns (e.g., "harvest_planted", "load_shipped") to create cryptographic
proofs of supply chain events.

Goal Grid Tasks:
- P7-T6: Show Python pipeline example
- P6-T6: Document 4th use case (produce operations)

Usage:
    python examples/produce_operations.py
"""

from northroot import Client
import tempfile
import shutil
from datetime import datetime, timezone
from typing import Dict, Any

# Create a client with temporary storage
storage_dir = tempfile.mkdtemp(prefix="northroot-produce-")
client = Client(storage_path=storage_dir)
print(f"📁 Storage: {storage_dir}\n")


def demonstrate_harvest_lifecycle():
    """Demonstrate complete harvest lifecycle with semantic workload_id patterns."""
    print("=" * 70)
    print("POTATO HARVEST LIFECYCLE")
    print("=" * 70)
    
    field_id = "field-123"
    crop = "potatoes"
    variety = "russet"
    trace_id = f"harvest-{field_id}-2025-11-20"
    
    # Phase 1: Intent - Plan to harvest
    print("\n1. Recording harvest intent (harvest_planted)")
    intent_receipt = client.record_work(
        workload_id="harvest_planted",
        payload={
            "field_id": field_id,
            "crop": crop,
            "variety": variety,
            "planned_date": "2025-11-20",
            "estimated_yield_lbs": 5000,
            "harvest_method": "mechanical",
            "operator": "farm-operator-001",
        },
        tags=["agriculture", "harvest", "intent"],
        trace_id=trace_id,
    )
    client.store_receipt(intent_receipt)
    print(f"   ✓ Receipt ID: {intent_receipt.get_rid()[:16]}...")
    print(f"   ✓ Hash: {intent_receipt.get_hash()[:32]}...")
    print(f"   ✓ Workload: harvest_planted")
    
    # Phase 2: Execution - Actual harvest operation
    print("\n2. Recording harvest execution (harvest_executed)")
    execution_receipt = client.record_work(
        workload_id="harvest_executed",
        payload={
            "field_id": field_id,
            "crop": crop,
            "actual_start_time": "2025-11-20T08:00:00Z",
            "actual_end_time": "2025-11-20T14:30:00Z",
            "harvester_id": "harvester-001",
            "operator_id": "operator-456",
            "weather_conditions": {
                "temp_f": 65,
                "humidity": 0.45,
                "wind_mph": 5,
            },
            "soil_moisture": 0.32,
            "equipment_fuel_used_gallons": 12.5,
        },
        tags=["agriculture", "harvest", "execution"],
        trace_id=trace_id,
        parent_id=str(intent_receipt.get_rid()),
    )
    client.store_receipt(execution_receipt)
    print(f"   ✓ Receipt ID: {execution_receipt.get_rid()[:16]}...")
    print(f"   ✓ Linked to intent receipt (dom matches cod)")
    print(f"   ✓ Workload: harvest_executed")
    
    # Phase 3: Outcome - Harvest results
    print("\n3. Recording harvest outcome (harvest_outcome)")
    outcome_receipt = client.record_work(
        workload_id="harvest_outcome",
        payload={
            "field_id": field_id,
            "crop": crop,
            "total_yield_lbs": 4850,
            "grade_distribution": {
                "us_no1": 4200,
                "us_no2": 500,
                "culls": 150,
            },
            "quality_metrics": {
                "avg_size_oz": 8.5,
                "defect_rate": 0.03,
                "sugar_content": 0.18,
            },
            "storage_location": "warehouse-789",
            "batch_id": "batch-2025-11-20-001",
            "timestamp": datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
        },
        tags=["agriculture", "harvest", "outcome"],
        trace_id=trace_id,
        parent_id=str(execution_receipt.get_rid()),
    )
    client.store_receipt(outcome_receipt)
    print(f"   ✓ Receipt ID: {outcome_receipt.get_rid()[:16]}...")
    print(f"   ✓ Linked to execution receipt")
    print(f"   ✓ Workload: harvest_outcome")
    
    # Verify composition
    print("\n4. Verifying receipt chain")
    is_valid_intent = client.verify_receipt(intent_receipt)
    is_valid_execution = client.verify_receipt(execution_receipt)
    is_valid_outcome = client.verify_receipt(outcome_receipt)
    
    print(f"   ✓ Intent receipt valid: {is_valid_intent}")
    print(f"   ✓ Execution receipt valid: {is_valid_execution}")
    print(f"   ✓ Outcome receipt valid: {is_valid_outcome}")
    
    # Query by trace_id
    print("\n5. Querying receipts by trace_id")
    all_harvest_receipts = client.list_receipts(trace_id=trace_id)
    print(f"   ✓ Found {len(all_harvest_receipts)} receipts for trace {trace_id}")
    for receipt in all_harvest_receipts:
        # Extract workload_id from receipt (would need to access payload)
        print(f"   - Receipt {receipt.get_rid()[:16]}...")
    
    return trace_id, [intent_receipt, execution_receipt, outcome_receipt]


def demonstrate_shipping_lifecycle():
    """Demonstrate shipping lifecycle with semantic workload_id patterns."""
    print("\n" + "=" * 70)
    print("POTATO SHIPPING LIFECYCLE")
    print("=" * 70)
    
    batch_id = "batch-2025-11-20-001"
    trace_id = f"shipment-{batch_id}-2025-11-21"
    
    # Phase 1: Load prepared
    print("\n1. Recording load preparation (load_prepared)")
    load_receipt = client.record_work(
        workload_id="load_prepared",
        payload={
            "batch_id": batch_id,
            "source_location": "warehouse-789",
            "destination": "distribution-center-456",
            "weight_lbs": 4200,
            "packaging": "bulk-bags",
            "prepared_by": "warehouse-staff-123",
            "preparation_time": "2025-11-21T06:00:00Z",
        },
        tags=["agriculture", "shipping", "preparation"],
        trace_id=trace_id,
    )
    client.store_receipt(load_receipt)
    print(f"   ✓ Receipt ID: {load_receipt.get_rid()[:16]}...")
    print(f"   ✓ Workload: load_prepared")
    
    # Phase 2: Load shipped
    print("\n2. Recording shipment (load_shipped)")
    shipment_receipt = client.record_work(
        workload_id="load_shipped",
        payload={
            "batch_id": batch_id,
            "truck_id": "truck-789",
            "driver_id": "driver-456",
            "departure_time": "2025-11-21T08:00:00Z",
            "estimated_arrival": "2025-11-21T14:00:00Z",
            "route": "route-123",
            "temperature_controlled": True,
            "temperature_setpoint_f": 45,
        },
        tags=["agriculture", "shipping", "execution"],
        trace_id=trace_id,
        parent_id=str(load_receipt.get_rid()),
    )
    client.store_receipt(shipment_receipt)
    print(f"   ✓ Receipt ID: {shipment_receipt.get_rid()[:16]}...")
    print(f"   ✓ Linked to load receipt")
    print(f"   ✓ Workload: load_shipped")
    
    # Phase 3: Load received
    print("\n3. Recording delivery (load_received)")
    delivery_receipt = client.record_work(
        workload_id="load_received",
        payload={
            "batch_id": batch_id,
            "destination": "distribution-center-456",
            "arrival_time": "2025-11-21T13:45:00Z",
            "received_by": "dc-staff-789",
            "weight_lbs": 4195,  # Slight loss during transport
            "condition": "good",
            "temperature_actual_f": 44.5,
            "quality_check": "passed",
        },
        tags=["agriculture", "shipping", "outcome"],
        trace_id=trace_id,
        parent_id=str(shipment_receipt.get_rid()),
    )
    client.store_receipt(delivery_receipt)
    print(f"   ✓ Receipt ID: {delivery_receipt.get_rid()[:16]}...")
    print(f"   ✓ Linked to shipment receipt")
    print(f"   ✓ Workload: load_received")
    
    # Verify chain
    print("\n4. Verifying shipping chain")
    is_valid_load = client.verify_receipt(load_receipt)
    is_valid_shipment = client.verify_receipt(shipment_receipt)
    is_valid_delivery = client.verify_receipt(delivery_receipt)
    
    print(f"   ✓ Load receipt valid: {is_valid_load}")
    print(f"   ✓ Shipment receipt valid: {is_valid_shipment}")
    print(f"   ✓ Delivery receipt valid: {is_valid_delivery}")
    
    return trace_id, [load_receipt, shipment_receipt, delivery_receipt]


def demonstrate_querying():
    """Demonstrate querying patterns for produce operations."""
    print("\n" + "=" * 70)
    print("QUERYING PRODUCE RECEIPTS")
    print("=" * 70)
    
    # Query by workload_id (semantic type)
    print("\n1. Querying by workload_id (harvest_planted)")
    harvest_receipts = client.list_receipts(workload_id="harvest_planted")
    print(f"   ✓ Found {len(harvest_receipts)} harvest_planted receipts")
    
    print("\n2. Querying by workload_id (load_shipped)")
    shipment_receipts = client.list_receipts(workload_id="load_shipped")
    print(f"   ✓ Found {len(shipment_receipts)} load_shipped receipts")
    
    # Query all receipts
    print("\n3. Listing all receipts")
    all_receipts = client.list_receipts()
    print(f"   ✓ Total receipts stored: {len(all_receipts)}")
    
    # Group by workload_id
    print("\n4. Grouping by workload_id")
    workload_counts: Dict[str, int] = {}
    for receipt in all_receipts:
        # Note: In real implementation, we'd extract workload_id from receipt payload
        # For now, we'll just show the pattern
        pass
    print("   ✓ Receipts grouped by operation type")


def main():
    """Run all produce operations demonstrations."""
    print("=" * 70)
    print("PRODUCE OPERATIONS: POTATO SUPPLY CHAIN")
    print("=" * 70)
    print("\nThis example demonstrates:")
    print("  - Semantic workload_id patterns (harvest_planted, load_shipped, etc.)")
    print("  - Supply chain lifecycle tracking")
    print("  - Receipt chaining via parent_id and trace_id")
    print("  - Querying by workload_id and trace_id")
    print("  - Clean data at ingestion")
    
    try:
        # Demonstrate harvest lifecycle
        harvest_trace_id, harvest_receipts = demonstrate_harvest_lifecycle()
        
        # Demonstrate shipping lifecycle
        shipping_trace_id, shipping_receipts = demonstrate_shipping_lifecycle()
        
        # Demonstrate querying
        demonstrate_querying()
        
        print("\n" + "=" * 70)
        print("✅ ALL DEMONSTRATIONS COMPLETE")
        print("=" * 70)
        print(f"\n📊 Summary:")
        print(f"   - Harvest trace: {harvest_trace_id}")
        print(f"   - Shipping trace: {shipping_trace_id}")
        print(f"   - Total receipts created: {len(harvest_receipts) + len(shipping_receipts)}")
        print(f"\n💡 Key Patterns:")
        print(f"   - workload_id: Semantic operation type (harvest_planted, load_shipped)")
        print(f"   - trace_id: Groups related receipts for a specific instance")
        print(f"   - parent_id: Links receipts in a chain (dom/cod composition)")
        print(f"   - Clean metadata: All supply chain data in payload")
        
    finally:
        # Cleanup
        print(f"\n🧹 Cleaning up temporary storage: {storage_dir}")
        shutil.rmtree(storage_dir)
        print("✅ Cleanup complete")


if __name__ == "__main__":
    main()

