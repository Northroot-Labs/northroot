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


# Add async methods to Client class via monkey-patching
# This keeps Rust code simple and avoids complex type conversions
def _add_async_methods():
    """Add async methods to Client class"""
    async def record_work_async(self, workload_id, payload, tags=None, trace_id=None, parent_id=None):
        """Async version of record_work"""
        return await asyncio.to_thread(
            self.record_work,
            workload_id,
            payload,
            tags,
            trace_id,
            parent_id,
        )
    
    async def verify_receipt_async(self, receipt):
        """Async version of verify_receipt"""
        return await asyncio.to_thread(self.verify_receipt, receipt)
    
    # Monkey-patch the methods onto Client class
    Client.record_work_async = record_work_async
    Client.verify_receipt_async = verify_receipt_async

_add_async_methods()


__all__ = [
    "Client",
    "receipts",
    "delta",
    "shapes",
]

# Module-level functions are kept internal for Client to use
# Public API is via Client class only

