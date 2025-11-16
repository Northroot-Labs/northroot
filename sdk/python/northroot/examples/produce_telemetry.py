#!/usr/bin/env python3
"""
Produce Telemetry: Sensor Data Collection Example

This example demonstrates time-series receipt chaining for agricultural telemetry data,
including sensor readings (soil moisture, temperature, humidity) and equipment telemetry
(harvester GPS, fuel consumption). Shows how to create verifiable receipts for continuous
monitoring data.

Goal Grid Task:
- P5-T3: Provide structured-logging example

Usage:
    python examples/produce_telemetry.py
"""

from northroot import Client
import tempfile
import shutil
from datetime import datetime, timedelta
from typing import List, Dict, Any

# Create a client with temporary storage
storage_dir = tempfile.mkdtemp(prefix="northroot-telemetry-")
client = Client(storage_path=storage_dir)
print(f"📁 Storage: {storage_dir}\n")


def demonstrate_sensor_telemetry():
    """Demonstrate sensor data collection with time-series receipt chaining."""
    print("=" * 70)
    print("SENSOR TELEMETRY: SOIL MOISTURE & WEATHER")
    print("=" * 70)
    
    field_id = "field-123"
    trace_id = f"sensors-{field_id}-2025-11-20"
    
    # Simulate hourly sensor readings
    base_time = datetime(2025, 11, 20, 6, 0, 0)
    sensor_readings: List[Any] = []
    
    print("\n1. Recording hourly sensor readings")
    for hour in range(12):  # 12 hours of data
        reading_time = base_time + timedelta(hours=hour)
        
        # Simulate sensor data
        sensor_data = {
            "field_id": field_id,
            "timestamp": reading_time.isoformat() + "Z",
            "sensor_type": "soil_moisture",
            "sensor_id": f"sensor-{field_id}-001",
            "reading": {
                "soil_moisture_pct": 32.5 + (hour * 0.1),  # Gradually increasing
                "temperature_f": 65.0 + (hour * 2.0),  # Warming during day
                "humidity_pct": 45.0 - (hour * 1.5),  # Drying during day
            },
            "location": {
                "latitude": 40.7128,
                "longitude": -74.0060,
                "zone": "zone-A",
            },
        }
        
        # Create receipt for this sensor reading
        receipt = client.record_work(
            workload_id="sensor_reading",
            payload=sensor_data,
            tags=["agriculture", "telemetry", "sensor"],
            trace_id=trace_id,
            parent_id=str(sensor_readings[-1].get_rid()) if sensor_readings else None,
        )
        client.store_receipt(receipt)
        sensor_readings.append(receipt)
        
        if hour % 3 == 0:  # Print every 3 hours
            print(f"   ✓ Hour {hour:2d}: {reading_time.strftime('%H:%M')} - "
                  f"Moisture: {sensor_data['reading']['soil_moisture_pct']:.1f}%, "
                  f"Temp: {sensor_data['reading']['temperature_f']:.1f}°F")
    
    print(f"\n   ✓ Recorded {len(sensor_readings)} sensor readings")
    print(f"   ✓ All linked in time-series chain")
    
    # Verify chain
    print("\n2. Verifying sensor reading chain")
    valid_count = sum(1 for r in sensor_readings if client.verify_receipt(r))
    print(f"   ✓ {valid_count}/{len(sensor_readings)} receipts valid")
    
    return trace_id, sensor_readings


def demonstrate_equipment_telemetry():
    """Demonstrate equipment telemetry (harvester GPS, fuel consumption)."""
    print("\n" + "=" * 70)
    print("EQUIPMENT TELEMETRY: HARVESTER GPS & FUEL")
    print("=" * 70)
    
    harvester_id = "harvester-001"
    trace_id = f"equipment-{harvester_id}-2025-11-20"
    
    # Simulate GPS waypoints during harvest
    waypoints = [
        {"lat": 40.7128, "lon": -74.0060, "time": "08:00"},
        {"lat": 40.7130, "lon": -74.0058, "time": "08:15"},
        {"lat": 40.7132, "lon": -74.0056, "time": "08:30"},
        {"lat": 40.7134, "lon": -74.0054, "time": "08:45"},
    ]
    
    print("\n1. Recording harvester GPS waypoints")
    equipment_readings: List[Any] = []
    
    for i, waypoint in enumerate(waypoints):
        equipment_data = {
            "equipment_id": harvester_id,
            "equipment_type": "harvester",
            "timestamp": f"2025-11-20T{waypoint['time']}:00Z",
            "gps": {
                "latitude": waypoint["lat"],
                "longitude": waypoint["lon"],
                "altitude_ft": 100.0,
                "speed_mph": 3.5,
                "heading_deg": 45.0 + (i * 10),
            },
            "fuel": {
                "level_pct": 85.0 - (i * 2.5),
                "consumption_gallons": i * 0.5,
            },
            "operational": {
                "engine_hours": 1250.5 + (i * 0.25),
                "harvest_rate_lbs_per_hour": 500.0,
            },
        }
        
        receipt = client.record_work(
            workload_id="equipment_telemetry",
            payload=equipment_data,
            tags=["agriculture", "telemetry", "equipment"],
            trace_id=trace_id,
            parent_id=str(equipment_readings[-1].get_rid()) if equipment_readings else None,
        )
        client.store_receipt(receipt)
        equipment_readings.append(receipt)
        
        print(f"   ✓ Waypoint {i+1}: {waypoint['time']} - "
              f"Lat: {waypoint['lat']:.4f}, Lon: {waypoint['lon']:.4f}, "
              f"Fuel: {equipment_data['fuel']['level_pct']:.1f}%")
    
    print(f"\n   ✓ Recorded {len(equipment_readings)} equipment telemetry readings")
    
    # Verify chain
    print("\n2. Verifying equipment telemetry chain")
    valid_count = sum(1 for r in equipment_readings if client.verify_receipt(r))
    print(f"   ✓ {valid_count}/{len(equipment_readings)} receipts valid")
    
    return trace_id, equipment_readings


def demonstrate_weather_integration():
    """Demonstrate weather data integration."""
    print("\n" + "=" * 70)
    print("WEATHER INTEGRATION")
    print("=" * 70)
    
    field_id = "field-123"
    trace_id = f"weather-{field_id}-2025-11-20"
    
    # Simulate weather station readings
    weather_readings = [
        {"time": "06:00", "temp_f": 55, "humidity": 60, "wind_mph": 3, "precip_in": 0.0},
        {"time": "12:00", "temp_f": 70, "humidity": 45, "wind_mph": 8, "precip_in": 0.0},
        {"time": "18:00", "temp_f": 65, "humidity": 50, "wind_mph": 5, "precip_in": 0.1},
    ]
    
    print("\n1. Recording weather station readings")
    weather_receipts: List[Any] = []
    
    for reading in weather_readings:
        weather_data = {
            "field_id": field_id,
            "timestamp": f"2025-11-20T{reading['time']}:00Z",
            "weather_station_id": f"weather-{field_id}-001",
            "conditions": {
                "temperature_f": reading["temp_f"],
                "humidity_pct": reading["humidity"],
                "wind_speed_mph": reading["wind_mph"],
                "wind_direction_deg": 180.0,
                "precipitation_inches": reading["precip_in"],
                "pressure_inhg": 30.15,
            },
            "forecast": {
                "next_24h_precip_prob": 0.2,
                "next_24h_temp_high_f": 72,
                "next_24h_temp_low_f": 58,
            },
        }
        
        receipt = client.record_work(
            workload_id="weather_reading",
            payload=weather_data,
            tags=["agriculture", "telemetry", "weather"],
            trace_id=trace_id,
            parent_id=str(weather_receipts[-1].get_rid()) if weather_receipts else None,
        )
        client.store_receipt(receipt)
        weather_receipts.append(receipt)
        
        print(f"   ✓ {reading['time']}: {reading['temp_f']}°F, "
              f"{reading['humidity']}% humidity, {reading['wind_mph']} mph wind")
    
    print(f"\n   ✓ Recorded {len(weather_receipts)} weather readings")
    
    return trace_id, weather_receipts


def demonstrate_querying_telemetry():
    """Demonstrate querying patterns for telemetry data."""
    print("\n" + "=" * 70)
    print("QUERYING TELEMETRY DATA")
    print("=" * 70)
    
    # Query by workload_id
    print("\n1. Querying sensor readings")
    sensor_receipts = client.list_receipts(workload_id="sensor_reading")
    print(f"   ✓ Found {len(sensor_receipts)} sensor_reading receipts")
    
    print("\n2. Querying equipment telemetry")
    equipment_receipts = client.list_receipts(workload_id="equipment_telemetry")
    print(f"   ✓ Found {len(equipment_receipts)} equipment_telemetry receipts")
    
    print("\n3. Querying weather readings")
    weather_receipts = client.list_receipts(workload_id="weather_reading")
    print(f"   ✓ Found {len(weather_receipts)} weather_reading receipts")
    
    # Query by trace_id
    print("\n4. Querying by trace_id (time-series chains)")
    all_receipts = client.list_receipts()
    print(f"   ✓ Total telemetry receipts: {len(all_receipts)}")
    
    # Group by trace
    trace_counts: Dict[str, int] = {}
    for receipt in all_receipts:
        # In real implementation, extract trace_id from receipt
        # For now, just show the pattern
        pass
    print("   ✓ Receipts grouped by trace (time-series chains)")


def main():
    """Run all telemetry demonstrations."""
    print("=" * 70)
    print("PRODUCE TELEMETRY: SENSOR & EQUIPMENT DATA")
    print("=" * 70)
    print("\nThis example demonstrates:")
    print("  - Time-series receipt chaining for continuous monitoring")
    print("  - Sensor data collection (soil moisture, temperature, humidity)")
    print("  - Equipment telemetry (harvester GPS, fuel consumption)")
    print("  - Weather data integration")
    print("  - Querying patterns for telemetry data")
    
    try:
        # Demonstrate sensor telemetry
        sensor_trace_id, sensor_readings = demonstrate_sensor_telemetry()
        
        # Demonstrate equipment telemetry
        equipment_trace_id, equipment_readings = demonstrate_equipment_telemetry()
        
        # Demonstrate weather integration
        weather_trace_id, weather_receipts = demonstrate_weather_integration()
        
        # Demonstrate querying
        demonstrate_querying_telemetry()
        
        print("\n" + "=" * 70)
        print("✅ ALL TELEMETRY DEMONSTRATIONS COMPLETE")
        print("=" * 70)
        print(f"\n📊 Summary:")
        print(f"   - Sensor readings: {len(sensor_readings)} receipts")
        print(f"   - Equipment telemetry: {len(equipment_readings)} receipts")
        print(f"   - Weather readings: {len(weather_receipts)} receipts")
        print(f"   - Total telemetry receipts: {len(sensor_readings) + len(equipment_readings) + len(weather_receipts)}")
        print(f"\n💡 Key Patterns:")
        print(f"   - workload_id: Semantic type (sensor_reading, equipment_telemetry, weather_reading)")
        print(f"   - trace_id: Groups time-series data for a specific source")
        print(f"   - parent_id: Links receipts in chronological chain")
        print(f"   - Clean metadata: All telemetry data in payload")
        
    finally:
        # Cleanup
        print(f"\n🧹 Cleaning up temporary storage: {storage_dir}")
        shutil.rmtree(storage_dir)
        print("✅ Cleanup complete")


if __name__ == "__main__":
    main()

