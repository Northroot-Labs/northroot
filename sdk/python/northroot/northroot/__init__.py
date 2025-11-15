"""
Northroot Python SDK

A thin client for the Northroot proof algebra system.
"""

# Import the Rust extension module (built by maturin)
try:
    from _northroot import (
        Client,
        record_work,
        verify_receipt,
        # Submodules
        receipts,
        delta,
        shapes,
    )
except ImportError:
    # Fallback for development - try importing from parent
    import sys
    import os
    # Add parent directory to path for development
    sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
    from _northroot import (
        Client,
        record_work,
        verify_receipt,
        receipts,
        delta,
        shapes,
    )

# Async wrappers using asyncio.to_thread
import asyncio
from functools import partial


async def record_work_async(workload_id, payload, tags=None, trace_id=None, parent_id=None):
    """
    Async version of record_work.
    
    Runs the synchronous record_work in a thread pool to avoid blocking the event loop.
    
    Args:
        workload_id: Identifier for this unit of work (required)
        payload: Work payload as a dictionary (required)
        tags: Optional tags for categorization
        trace_id: Optional trace ID for grouping related work units
        parent_id: Optional parent receipt ID for DAG composition
    
    Returns:
        A PyReceipt containing the verifiable proof of work.
    
    Example:
        >>> import northroot as nr
        >>> receipt = await nr.record_work_async(
        ...     workload_id="normalize-prices",
        ...     payload={"input": "data"},
        ...     tags=["etl"]
        ... )
    """
    return await asyncio.to_thread(
        record_work,
        workload_id,
        payload,
        tags,
        trace_id,
        parent_id,
    )


async def verify_receipt_async(receipt):
    """
    Async version of verify_receipt.
    
    Runs the synchronous verify_receipt in a thread pool to avoid blocking the event loop.
    
    Args:
        receipt: The receipt to verify
    
    Returns:
        True if the receipt is valid, False if invalid.
    
    Example:
        >>> import northroot as nr
        >>> is_valid = await nr.verify_receipt_async(receipt)
    """
    return await asyncio.to_thread(verify_receipt, receipt)


__all__ = [
    "Client",
    "record_work",
    "verify_receipt",
    "record_work_async",
    "verify_receipt_async",
    "receipts",
    "delta",
    "shapes",
]

