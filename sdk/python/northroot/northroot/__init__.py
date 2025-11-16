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
        record_work_py as _record_work,
        verify_receipt_py as _verify_receipt,
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
        record_work_py as _record_work,
        verify_receipt_py as _verify_receipt,
        receipts,
        delta,
        shapes,
    )


class Client:
    """
    Northroot SDK Client with sync and async support.
    
    This class wraps the Rust-bound sync client with idiomatic async methods.
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
    
    def __init__(self, storage_path: Optional[str] = None):
        """
        Create a new client.
        
        Args:
            storage_path: Optional path for local storage (filesystem-based).
                         If None, receipts are not persisted (can still be created and verified).
        """
        self._client = _SyncClient(storage_path)
    
    def record_work(
        self,
        workload_id: str,
        payload: Dict[str, Any],
        tags: Optional[List[str]] = None,
        trace_id: Optional[str] = None,
        parent_id: Optional[str] = None,
    ):
        """Sync version of record_work. Delegates to Rust client."""
        return self._client.record_work(workload_id, payload, tags, trace_id, parent_id)
    
    def verify_receipt(self, receipt):
        """Sync version of verify_receipt. Delegates to Rust client."""
        return self._client.verify_receipt(receipt)
    
    def store_receipt(self, receipt):
        """Sync version of store_receipt. Delegates to Rust client."""
        return self._client.store_receipt(receipt)
    
    def list_receipts(
        self,
        workload_id: Optional[str] = None,
        trace_id: Optional[str] = None,
        limit: Optional[int] = None,
    ):
        """Sync version of list_receipts. Delegates to Rust client."""
        return self._client.list_receipts(workload_id, trace_id, limit)
    
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
            self._client.record_work,
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
        return await asyncio.to_thread(self._client.verify_receipt, receipt)
    
    async def list_receipts_async(
        self,
        workload_id: Optional[str] = None,
        trace_id: Optional[str] = None,
        limit: Optional[int] = None,
    ):
        """
        Async version of list_receipts.
        
        Lists receipts matching the query criteria asynchronously.
        This method runs the sync Rust implementation in a thread pool.
        
        Args:
            workload_id: Optional filter by workload ID
            trace_id: Optional filter by trace ID
            limit: Optional maximum number of results
        
        Returns:
            List of PyReceipt objects matching the query
        """
        return await asyncio.to_thread(
            self._client.list_receipts,
            workload_id,
            trace_id,
            limit,
        )


# Optional OTEL integration
try:
    from northroot.otel import span_to_receipt, trace_work, OTEL_AVAILABLE
    _all_list = [
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
    _all_list = [
        "Client",
        "receipts",
        "delta",
        "shapes",
    ]

# Optional produce operations helpers
try:
    from northroot.produce import (
        record_harvest_planted,
        record_harvest_executed,
        record_harvest_outcome,
        record_load_prepared,
        record_load_shipped,
        record_load_received,
    )
    _all_list.extend([
        "record_harvest_planted",
        "record_harvest_executed",
        "record_harvest_outcome",
        "record_load_prepared",
        "record_load_shipped",
        "record_load_received",
    ])
except ImportError:
    # Produce module may not be available
    pass

__all__ = _all_list

