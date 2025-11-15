"""
OpenTelemetry integration for Northroot.

This module provides utilities to convert OpenTelemetry spans to Northroot receipts,
enabling seamless integration with existing observability infrastructure.

Example:
    >>> from opentelemetry import trace
    >>> from northroot.otel import span_to_receipt
    >>> 
    >>> tracer = trace.get_tracer(__name__)
    >>> with tracer.start_as_current_span("process-data") as span:
    ...     # Your existing code
    ...     result = process_data()
    ...     # Convert span to receipt
    ...     receipt = span_to_receipt(span, {"result": result})
"""

from typing import Optional, Dict, Any, List
import uuid

try:
    from opentelemetry.trace import Span, Status, StatusCode
    from opentelemetry.trace.span import TraceFlags
    OTEL_AVAILABLE = True
except ImportError:
    OTEL_AVAILABLE = False
    # Create dummy types for type hints when OTEL is not available
    Span = Any  # type: ignore
    Status = Any  # type: ignore
    StatusCode = Any  # type: ignore

from northroot import Client


def span_to_receipt(
    span: Span,
    payload: Optional[Dict[str, Any]] = None,
    tags: Optional[List[str]] = None,
    client: Optional[Client] = None,
) -> Any:  # Returns PyReceipt, but type is Any to avoid import issues
    """
    Convert an OpenTelemetry span to a Northroot receipt.
    
    This function extracts relevant information from an OTEL span and creates
    a verifiable receipt. This enables wrapping existing instrumented code
    with minimal changes.
    
    Args:
        span: OpenTelemetry Span object
        payload: Optional additional payload data to include in receipt.
                 If None, span attributes are used as payload.
        tags: Optional tags for categorization. If None, uses span attributes
              with key "tags" or empty list.
        client: Optional Client instance. If None, creates a new client.
    
    Returns:
        PyReceipt object containing the verifiable proof of work
    
    Raises:
        ImportError: If opentelemetry is not installed
        ValueError: If span is invalid or conversion fails
    
    Example:
        >>> from opentelemetry import trace
        >>> from northroot.otel import span_to_receipt
        >>> 
        >>> tracer = trace.get_tracer(__name__)
        >>> with tracer.start_as_current_span("normalize-prices") as span:
        ...     span.set_attribute("input_size", 1000)
        ...     span.set_attribute("output_size", 950)
        ...     # Your processing code here
        ...     receipt = span_to_receipt(span)
        ...     print(f"Receipt ID: {receipt.get_rid()}")
    """
    if not OTEL_AVAILABLE:
        raise ImportError(
            "opentelemetry is not installed. Install it with: pip install opentelemetry-api"
        )
    
    if client is None:
        client = Client()
    
    # Extract workload_id from span name
    workload_id = span.name or "unknown-workload"
    
    # Extract trace_id from span context
    trace_id = None
    if span.context.is_valid:
        trace_id = format(span.context.trace_id, "032x")  # 32 hex chars
    
    # Extract parent_id from span context (if available)
    parent_id = None
    if span.parent and span.parent.is_valid:
        # Use parent span ID as parent_id
        parent_id = format(span.parent.span_id, "016x")
    
    # Build payload from span attributes
    if payload is None:
        payload = {}
    
    # Add span attributes to payload
    if hasattr(span, "attributes") and span.attributes:
        for key, value in span.attributes.items():
            # Skip internal OTEL attributes
            if not key.startswith("otel."):
                payload[key] = value
    
    # Add span metadata
    payload["_span_kind"] = str(span.kind) if hasattr(span, "kind") else None
    payload["_span_status"] = str(span.status.status_code) if hasattr(span, "status") and span.status else None
    
    # Extract tags from span attributes or use provided tags
    if tags is None:
        tags = []
        if hasattr(span, "attributes") and span.attributes:
            # Look for "tags" attribute or common tag patterns
            if "tags" in span.attributes:
                tags_value = span.attributes["tags"]
                if isinstance(tags_value, list):
                    tags = [str(t) for t in tags_value]
                elif isinstance(tags_value, str):
                    tags = [tags_value]
    
    # Create receipt
    receipt = client.record_work(
        workload_id=workload_id,
        payload=payload,
        tags=tags,
        trace_id=trace_id,
        parent_id=parent_id,
    )
    
    return receipt


def trace_work(workload_id: Optional[str] = None, tags: Optional[List[str]] = None):
    """
    Decorator to automatically create receipts from function execution.
    
    This decorator wraps a function and creates a receipt for each execution.
    It integrates with OpenTelemetry if available, otherwise creates standalone receipts.
    
    Args:
        workload_id: Optional workload identifier. If None, uses function name.
        tags: Optional tags for categorization.
    
    Example:
        >>> from northroot.otel import trace_work
        >>> 
        >>> @trace_work(workload_id="normalize-prices", tags=["etl"])
        >>> def normalize_prices(data):
        ...     # Your processing code
        ...     return processed_data
        >>> 
        >>> result = normalize_prices(data)  # Automatically creates receipt
    """
    def decorator(func):
        import functools
        
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            client = Client()
            w_id = workload_id or func.__name__
            
            if OTEL_AVAILABLE:
                # With OTEL: use span context
                from opentelemetry import trace
                
                tracer = trace.get_tracer(__name__)
                with tracer.start_as_current_span(w_id) as span:
                    # Execute function
                    result = func(*args, **kwargs)
                    
                    # Convert span to receipt
                    span_to_receipt(span, tags=tags, client=client)
                    
                    return result
            else:
                # Fallback: create receipt without OTEL
                # Execute function
                result = func(*args, **kwargs)
                
                # Create receipt with function metadata
                payload = {
                    "function": func.__name__,
                    "args_count": len(args),
                    "kwargs_keys": list(kwargs.keys()),
                }
                client.record_work(
                    workload_id=w_id,
                    payload=payload,
                    tags=tags or [],
                )
                
                return result
        
        return wrapper
    return decorator


__all__ = [
    "span_to_receipt",
    "trace_work",
    "OTEL_AVAILABLE",
]

