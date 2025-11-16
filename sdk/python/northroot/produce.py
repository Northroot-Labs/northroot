"""
Produce Operations Helper Module

Domain-specific helpers for agricultural supply chain operations.
These are thin wrappers around `record_work()` that provide semantic
workload_id patterns and clean metadata structures.

Goal Grid Tasks:
- P7-T6: Show Python pipeline example
- P6-T6: Document 4th use case (produce operations)

Usage:
    >>> from northroot import Client
    >>> from northroot.produce import record_harvest_planted, record_load_shipped
    >>> 
    >>> client = Client(storage_path="./receipts")
    >>> receipt = record_harvest_planted(
    ...     client,
    ...     field_id="field-123",
    ...     crop="potatoes",
    ...     planned_date="2025-11-20"
    ... )
"""

from typing import Optional, Dict, Any, List
from datetime import datetime, timezone
from northroot import Client


def record_harvest_planted(
    client: Client,
    field_id: str,
    crop: str,
    planned_date: str,
    variety: Optional[str] = None,
    estimated_yield_lbs: Optional[float] = None,
    harvest_method: Optional[str] = None,
    operator: Optional[str] = None,
    trace_id: Optional[str] = None,
    parent_id: Optional[str] = None,
    **kwargs: Any,
) -> Any:  # Returns PyReceipt
    """
    Record a harvest planting intent receipt.
    
    Creates a receipt for the intent to harvest a crop from a field.
    This is the first phase in the harvest lifecycle.
    
    Args:
        client: Northroot Client instance
        field_id: Field identifier
        crop: Crop type (e.g., "potatoes", "corn")
        planned_date: Planned harvest date (ISO format or YYYY-MM-DD)
        variety: Optional crop variety (e.g., "russet", "yukon")
        estimated_yield_lbs: Optional estimated yield in pounds
        harvest_method: Optional harvest method (e.g., "mechanical", "manual")
        operator: Optional operator identifier
        trace_id: Optional trace ID for grouping (auto-generated if None)
        parent_id: Optional parent receipt ID for chaining
        **kwargs: Additional payload fields
    
    Returns:
        PyReceipt object
    
    Example:
        >>> receipt = record_harvest_planted(
        ...     client,
        ...     field_id="field-123",
        ...     crop="potatoes",
        ...     planned_date="2025-11-20",
        ...     variety="russet",
        ...     estimated_yield_lbs=5000
        ... )
    """
    if trace_id is None:
        trace_id = f"harvest-{field_id}-{planned_date}"
    
    payload: Dict[str, Any] = {
        "field_id": field_id,
        "crop": crop,
        "planned_date": planned_date,
    }
    
    if variety:
        payload["variety"] = variety
    if estimated_yield_lbs is not None:
        payload["estimated_yield_lbs"] = estimated_yield_lbs
    if harvest_method:
        payload["harvest_method"] = harvest_method
    if operator:
        payload["operator"] = operator
    
    # Add any additional kwargs
    payload.update(kwargs)
    
    return client.record_work(
        workload_id="harvest_planted",
        payload=payload,
        tags=["agriculture", "harvest", "intent"],
        trace_id=trace_id,
        parent_id=parent_id,
    )


def record_harvest_executed(
    client: Client,
    intent_receipt: Any,  # PyReceipt
    field_id: str,
    crop: str,
    actual_start_time: str,
    actual_end_time: str,
    harvester_id: Optional[str] = None,
    operator_id: Optional[str] = None,
    weather_conditions: Optional[Dict[str, Any]] = None,
    soil_moisture: Optional[float] = None,
    equipment_fuel_used_gallons: Optional[float] = None,
    trace_id: Optional[str] = None,
    **kwargs: Any,
) -> Any:  # Returns PyReceipt
    """
    Record a harvest execution receipt.
    
    Creates a receipt for the actual harvest operation, linked to the intent receipt.
    This is the second phase in the harvest lifecycle.
    
    Args:
        client: Northroot Client instance
        intent_receipt: Previous intent receipt (for chaining)
        field_id: Field identifier
        crop: Crop type
        actual_start_time: Actual harvest start time (ISO format)
        actual_end_time: Actual harvest end time (ISO format)
        harvester_id: Optional harvester equipment ID
        operator_id: Optional operator ID
        weather_conditions: Optional weather data dict
        soil_moisture: Optional soil moisture percentage
        equipment_fuel_used_gallons: Optional fuel consumption
        trace_id: Optional trace ID (uses intent receipt trace_id if None)
        **kwargs: Additional payload fields
    
    Returns:
        PyReceipt object
    
    Example:
        >>> execution_receipt = record_harvest_executed(
        ...     client,
        ...     intent_receipt,
        ...     field_id="field-123",
        ...     crop="potatoes",
        ...     actual_start_time="2025-11-20T08:00:00Z",
        ...     actual_end_time="2025-11-20T14:30:00Z",
        ...     harvester_id="harvester-001"
        ... )
    """
    if trace_id is None:
        # Try to get trace_id from intent receipt
        trace_id = intent_receipt.get_trace_id() if hasattr(intent_receipt, 'get_trace_id') else None
    
    payload: Dict[str, Any] = {
        "field_id": field_id,
        "crop": crop,
        "actual_start_time": actual_start_time,
        "actual_end_time": actual_end_time,
    }
    
    if harvester_id:
        payload["harvester_id"] = harvester_id
    if operator_id:
        payload["operator_id"] = operator_id
    if weather_conditions:
        payload["weather_conditions"] = weather_conditions
    if soil_moisture is not None:
        payload["soil_moisture"] = soil_moisture
    if equipment_fuel_used_gallons is not None:
        payload["equipment_fuel_used_gallons"] = equipment_fuel_used_gallons
    
    # Add any additional kwargs
    payload.update(kwargs)
    
    return client.record_work(
        workload_id="harvest_executed",
        payload=payload,
        tags=["agriculture", "harvest", "execution"],
        trace_id=trace_id,
        parent_id=str(intent_receipt.get_rid()),
    )


def record_harvest_outcome(
    client: Client,
    execution_receipt: Any,  # PyReceipt
    field_id: str,
    crop: str,
    total_yield_lbs: float,
    grade_distribution: Optional[Dict[str, float]] = None,
    quality_metrics: Optional[Dict[str, Any]] = None,
    storage_location: Optional[str] = None,
    batch_id: Optional[str] = None,
    trace_id: Optional[str] = None,
    **kwargs: Any,
) -> Any:  # Returns PyReceipt
    """
    Record a harvest outcome receipt.
    
    Creates a receipt for the harvest results, linked to the execution receipt.
    This is the third phase in the harvest lifecycle.
    
    Args:
        client: Northroot Client instance
        execution_receipt: Previous execution receipt (for chaining)
        field_id: Field identifier
        crop: Crop type
        total_yield_lbs: Total yield in pounds
        grade_distribution: Optional grade distribution dict (e.g., {"us_no1": 4200, "us_no2": 500})
        quality_metrics: Optional quality metrics dict
        storage_location: Optional storage location identifier
        batch_id: Optional batch identifier
        trace_id: Optional trace ID (uses execution receipt trace_id if None)
        **kwargs: Additional payload fields
    
    Returns:
        PyReceipt object
    
    Example:
        >>> outcome_receipt = record_harvest_outcome(
        ...     client,
        ...     execution_receipt,
        ...     field_id="field-123",
        ...     crop="potatoes",
        ...     total_yield_lbs=4850,
        ...     grade_distribution={"us_no1": 4200, "us_no2": 500, "culls": 150}
        ... )
    """
    if trace_id is None:
        # Try to get trace_id from execution receipt
        trace_id = execution_receipt.get_trace_id() if hasattr(execution_receipt, 'get_trace_id') else None
    
    payload: Dict[str, Any] = {
        "field_id": field_id,
        "crop": crop,
        "total_yield_lbs": total_yield_lbs,
        "timestamp": datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
    }
    
    if grade_distribution:
        payload["grade_distribution"] = grade_distribution
    if quality_metrics:
        payload["quality_metrics"] = quality_metrics
    if storage_location:
        payload["storage_location"] = storage_location
    if batch_id:
        payload["batch_id"] = batch_id
    
    # Add any additional kwargs
    payload.update(kwargs)
    
    return client.record_work(
        workload_id="harvest_outcome",
        payload=payload,
        tags=["agriculture", "harvest", "outcome"],
        trace_id=trace_id,
        parent_id=str(execution_receipt.get_rid()),
    )


def record_load_prepared(
    client: Client,
    batch_id: str,
    source_location: str,
    destination: str,
    weight_lbs: float,
    packaging: Optional[str] = None,
    prepared_by: Optional[str] = None,
    preparation_time: Optional[str] = None,
    trace_id: Optional[str] = None,
    parent_id: Optional[str] = None,
    **kwargs: Any,
) -> Any:  # Returns PyReceipt
    """
    Record a load preparation receipt.
    
    Creates a receipt for preparing a load for shipment.
    
    Args:
        client: Northroot Client instance
        batch_id: Batch identifier
        source_location: Source location identifier
        destination: Destination identifier
        weight_lbs: Weight in pounds
        packaging: Optional packaging type
        prepared_by: Optional preparer identifier
        preparation_time: Optional preparation time (ISO format)
        trace_id: Optional trace ID for grouping
        parent_id: Optional parent receipt ID
        **kwargs: Additional payload fields
    
    Returns:
        PyReceipt object
    """
    if trace_id is None:
        trace_id = f"shipment-{batch_id}-{datetime.now(timezone.utc).strftime('%Y-%m-%d')}"
    
    if preparation_time is None:
        preparation_time = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
    
    payload: Dict[str, Any] = {
        "batch_id": batch_id,
        "source_location": source_location,
        "destination": destination,
        "weight_lbs": weight_lbs,
        "preparation_time": preparation_time,
    }
    
    if packaging:
        payload["packaging"] = packaging
    if prepared_by:
        payload["prepared_by"] = prepared_by
    
    # Add any additional kwargs
    payload.update(kwargs)
    
    return client.record_work(
        workload_id="load_prepared",
        payload=payload,
        tags=["agriculture", "shipping", "preparation"],
        trace_id=trace_id,
        parent_id=parent_id,
    )


def record_load_shipped(
    client: Client,
    load_receipt: Any,  # PyReceipt
    batch_id: str,
    truck_id: Optional[str] = None,
    driver_id: Optional[str] = None,
    departure_time: Optional[str] = None,
    estimated_arrival: Optional[str] = None,
    route: Optional[str] = None,
    temperature_controlled: Optional[bool] = None,
    temperature_setpoint_f: Optional[float] = None,
    trace_id: Optional[str] = None,
    **kwargs: Any,
) -> Any:  # Returns PyReceipt
    """
    Record a shipment receipt.
    
    Creates a receipt for shipping a load, linked to the load preparation receipt.
    
    Args:
        client: Northroot Client instance
        load_receipt: Previous load preparation receipt (for chaining)
        batch_id: Batch identifier
        truck_id: Optional truck identifier
        driver_id: Optional driver identifier
        departure_time: Optional departure time (ISO format)
        estimated_arrival: Optional estimated arrival time (ISO format)
        route: Optional route identifier
        temperature_controlled: Optional temperature control flag
        temperature_setpoint_f: Optional temperature setpoint in Fahrenheit
        trace_id: Optional trace ID (uses load receipt trace_id if None)
        **kwargs: Additional payload fields
    
    Returns:
        PyReceipt object
    """
    if trace_id is None:
        # Try to get trace_id from load receipt
        trace_id = load_receipt.get_trace_id() if hasattr(load_receipt, 'get_trace_id') else None
    
    if departure_time is None:
        departure_time = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
    
    payload: Dict[str, Any] = {
        "batch_id": batch_id,
        "departure_time": departure_time,
    }
    
    if truck_id:
        payload["truck_id"] = truck_id
    if driver_id:
        payload["driver_id"] = driver_id
    if estimated_arrival:
        payload["estimated_arrival"] = estimated_arrival
    if route:
        payload["route"] = route
    if temperature_controlled is not None:
        payload["temperature_controlled"] = temperature_controlled
    if temperature_setpoint_f is not None:
        payload["temperature_setpoint_f"] = temperature_setpoint_f
    
    # Add any additional kwargs
    payload.update(kwargs)
    
    return client.record_work(
        workload_id="load_shipped",
        payload=payload,
        tags=["agriculture", "shipping", "execution"],
        trace_id=trace_id,
        parent_id=str(load_receipt.get_rid()),
    )


def record_load_received(
    client: Client,
    shipment_receipt: Any,  # PyReceipt
    batch_id: str,
    destination: str,
    arrival_time: Optional[str] = None,
    received_by: Optional[str] = None,
    weight_lbs: Optional[float] = None,
    condition: Optional[str] = None,
    temperature_actual_f: Optional[float] = None,
    quality_check: Optional[str] = None,
    trace_id: Optional[str] = None,
    **kwargs: Any,
) -> Any:  # Returns PyReceipt
    """
    Record a delivery receipt.
    
    Creates a receipt for receiving a shipment, linked to the shipment receipt.
    
    Args:
        client: Northroot Client instance
        shipment_receipt: Previous shipment receipt (for chaining)
        batch_id: Batch identifier
        destination: Destination identifier
        arrival_time: Optional arrival time (ISO format)
        received_by: Optional receiver identifier
        weight_lbs: Optional received weight in pounds
        condition: Optional condition assessment
        temperature_actual_f: Optional actual temperature in Fahrenheit
        quality_check: Optional quality check result
        trace_id: Optional trace ID (uses shipment receipt trace_id if None)
        **kwargs: Additional payload fields
    
    Returns:
        PyReceipt object
    """
    if trace_id is None:
        # Try to get trace_id from shipment receipt
        trace_id = shipment_receipt.get_trace_id() if hasattr(shipment_receipt, 'get_trace_id') else None
    
    if arrival_time is None:
        arrival_time = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
    
    payload: Dict[str, Any] = {
        "batch_id": batch_id,
        "destination": destination,
        "arrival_time": arrival_time,
    }
    
    if received_by:
        payload["received_by"] = received_by
    if weight_lbs is not None:
        payload["weight_lbs"] = weight_lbs
    if condition:
        payload["condition"] = condition
    if temperature_actual_f is not None:
        payload["temperature_actual_f"] = temperature_actual_f
    if quality_check:
        payload["quality_check"] = quality_check
    
    # Add any additional kwargs
    payload.update(kwargs)
    
    return client.record_work(
        workload_id="load_received",
        payload=payload,
        tags=["agriculture", "shipping", "outcome"],
        trace_id=trace_id,
        parent_id=str(shipment_receipt.get_rid()),
    )


__all__ = [
    "record_harvest_planted",
    "record_harvest_executed",
    "record_harvest_outcome",
    "record_load_prepared",
    "record_load_shipped",
    "record_load_received",
]

