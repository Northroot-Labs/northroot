"""
Northroot Python SDK

A thin client for the Northroot proof algebra system.
"""

import asyncio
from typing import Optional, Dict, Any, List

# Import the Rust extension module (built by maturin)
try:
    from _northroot import (
        Client as _SyncClient,
        record_work as _record_work,
        verify_receipt as _verify_receipt,
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
        Client as _SyncClient,
        record_work as _record_work,
        verify_receipt as _verify_receipt,
        receipts,
        delta,
        shapes,
    )


class Client(_SyncClient):
    """
    Northroot SDK Client with sync and async support.
    
    This class extends the Rust-bound sync client with idiomatic async methods.
    All async operations use asyncio.to_thread to run sync Rust code in a thread pool.
    
    Example:
        >>> from northroot import Client
        >>> 
        >>> # Sync usage
        >>> client = Client()
        >>> receipt = client.record_work("workload-id", {"data": "value"})
        >>> 
        >>> # Async usage
        >>> async def example():
        ...     client = Client()
        ...     receipt = await client.record_work_async("workload-id", {"data": "value"})
        ...     is_valid = await client.verify_receipt_async(receipt)
    """
    
    async def record_work_async(
        self,
        workload_id: str,
        payload: Dict[str, Any],
        tags: Optional[List[str]] = None,
        trace_id: Optional[str] = None,
        parent_id: Optional[str] = None,
    ):
        """
        Async version of record_work.
        
        Records a unit of work and produces a verifiable receipt asynchronously.
        This method runs the sync Rust implementation in a thread pool.
        
        Args:
            workload_id: Identifier for this unit of work
            payload: Work payload as a dictionary
            tags: Optional tags for categorization
            trace_id: Optional trace ID for grouping related work units
            parent_id: Optional parent receipt ID for DAG composition
        
        Returns:
            PyReceipt object containing the verifiable proof of work
        """
        return await asyncio.to_thread(
            self.record_work,
            workload_id,
            payload,
            tags,
            trace_id,
            parent_id,
        )
    
    async def verify_receipt_async(self, receipt):
        """
        Async version of verify_receipt.
        
        Verifies receipt integrity and hash correctness asynchronously.
        This method runs the sync Rust implementation in a thread pool.
        
        Args:
            receipt: PyReceipt object to verify
        
        Returns:
            True if receipt is valid, False if invalid
        """
        return await asyncio.to_thread(self.verify_receipt, receipt)


# Optional OTEL integration
try:
    from northroot.otel import span_to_receipt, trace_work, OTEL_AVAILABLE
    __all__ = [
        "Client",
        "receipts",
        "delta",
        "shapes",
        "span_to_receipt",
        "trace_work",
        "OTEL_AVAILABLE",
    ]
except ImportError:
    # OTEL module may not be available if dependencies are missing
    __all__ = [
        "Client",
        "receipts",
        "delta",
        "shapes",
    ]

