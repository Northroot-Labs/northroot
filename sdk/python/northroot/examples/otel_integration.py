#!/usr/bin/env python3
"""
OpenTelemetry Integration Example

This example demonstrates how to integrate Northroot receipts with OpenTelemetry spans,
enabling seamless observability for existing workflows.

This aligns with Goal Grid P5-T1: Build an OTEL span → proof transformer.
"""

try:
    from opentelemetry import trace
    from opentelemetry.sdk.trace import TracerProvider
    from opentelemetry.sdk.trace.export import ConsoleSpanExporter, BatchSpanProcessor
    OTEL_AVAILABLE = True
except ImportError:
    print("⚠️  OpenTelemetry not installed. Install with: pip install opentelemetry-api opentelemetry-sdk")
    print("   This example will use a simplified fallback mode.")
    OTEL_AVAILABLE = False

from northroot import Client
from northroot.otel import span_to_receipt, trace_work, OTEL_AVAILABLE


def example_1_basic_span_to_receipt():
    """Example 1: Convert an OTEL span to a receipt"""
    print("=" * 60)
    print("Example 1: Basic Span → Receipt Conversion")
    print("=" * 60)
    
    if not OTEL_AVAILABLE:
        print("Skipping: OpenTelemetry not available")
        return
    
    # Setup OTEL
    trace.set_tracer_provider(TracerProvider())
    tracer = trace.get_tracer(__name__)
    
    # Create a span (simulating existing instrumented code)
    with tracer.start_as_current_span("normalize-prices") as span:
        span.set_attribute("input_size", 1000)
        span.set_attribute("output_size", 950)
        span.set_attribute("processing_time_ms", 42)
        
        # Simulate work
        print("Processing data...")
        
        # Convert span to receipt
        receipt = span_to_receipt(span, tags=["etl", "batch"])
        print(f"✅ Created receipt: {receipt.get_rid()}")
        print(f"   Hash: {receipt.get_hash()}")
        print(f"   Workload: normalize-prices")


def example_2_trace_with_parent_child():
    """Example 2: Trace with parent-child relationships"""
    print("\n" + "=" * 60)
    print("Example 2: Parent-Child Trace Relationships")
    print("=" * 60)
    
    if not OTEL_AVAILABLE:
        print("Skipping: OpenTelemetry not available")
        return
    
    trace.set_tracer_provider(TracerProvider())
    tracer = trace.get_tracer(__name__)
    client = Client()
    
    # Parent span
    with tracer.start_as_current_span("etl-pipeline") as parent_span:
        parent_span.set_attribute("pipeline", "daily-etl")
        parent_receipt = span_to_receipt(parent_span, tags=["etl", "pipeline"])
        print(f"Parent receipt: {parent_receipt.get_rid()}")
        
        # Child span
        with tracer.start_as_current_span("normalize-prices") as child_span:
            child_span.set_attribute("step", "normalization")
            child_receipt = span_to_receipt(
                child_span,
                tags=["etl", "normalize"],
                client=client,
            )
            print(f"Child receipt: {child_receipt.get_rid()}")
            print(f"Parent receipt: {parent_receipt.get_rid()}")


def example_3_decorator_pattern():
    """Example 3: Using the @trace_work decorator"""
    print("\n" + "=" * 60)
    print("Example 3: Decorator Pattern")
    print("=" * 60)
    
    @trace_work(workload_id="normalize-prices", tags=["etl", "decorator"])
    def normalize_prices(data_size: int):
        """Simulate price normalization"""
        print(f"Normalizing {data_size} prices...")
        return data_size * 0.95  # Simulate reduction
    
    # Call function - receipt is created automatically
    result = normalize_prices(1000)
    print(f"✅ Function executed, result: {result}")
    print("   (Receipt created automatically via decorator)")


def example_4_fallback_without_otel():
    """Example 4: Fallback mode without OTEL"""
    print("\n" + "=" * 60)
    print("Example 4: Fallback Mode (No OTEL)")
    print("=" * 60)
    
    if OTEL_AVAILABLE:
        print("OTEL is available, showing fallback behavior...")
    
    # Use decorator without OTEL - it will still work
    @trace_work(workload_id="simple-task", tags=["fallback"])
    def simple_task():
        print("Executing simple task...")
        return "done"
    
    result = simple_task()
    print(f"✅ Task completed: {result}")


if __name__ == "__main__":
    print("Northroot OpenTelemetry Integration Examples")
    print("=" * 60)
    print()
    
    example_1_basic_span_to_receipt()
    example_2_trace_with_parent_child()
    example_3_decorator_pattern()
    example_4_fallback_without_otel()
    
    print("\n" + "=" * 60)
    print("✅ All examples complete!")
    print("=" * 60)
    print("\nNext steps:")
    print("1. Install OpenTelemetry: pip install opentelemetry-api opentelemetry-sdk")
    print("2. Integrate with your existing OTEL instrumentation")
    print("3. Use span_to_receipt() or @trace_work decorator")
    print("4. Receipts are automatically linked via trace_id and parent_id")

