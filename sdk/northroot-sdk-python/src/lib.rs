//! Northroot Python SDK
//!
//! This module provides Python bindings to the Northroot proof algebra engine.
//! It exposes core functionality for delta compute, receipt generation, and reuse decisions.
//!
//! ## Architecture Boundaries
//!
//! - **This crate**: Python bindings and high-level Python API
//! - **northroot-engine**: Core Rust engine (proof algebra, delta compute)
//! - **northroot-receipts**: Receipt structure and validation
//! - **northroot-storage**: Storage backends (optional)
//!
//! The SDK provides a Python-friendly interface over the Rust engine, maintaining
//! clear separation: SDK = language bindings, Engine = core logic.

use pyo3::prelude::*;

/// Client module (thin client wrapper)
mod client;

/// Delta compute module
mod delta;

/// Error types module
mod errors;

/// Receipts module
mod receipts;

/// Shapes module
mod shapes;

/// Python module definition
#[pymodule]
fn northroot_sdk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register error classes
    errors::register_errors(m)?;

    // Register Client class
    m.add_class::<client::Client>()?;

    // Register module-level functions for ergonomic usage
    m.add_function(pyo3::wrap_pyfunction!(client::record_work_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(client::verify_receipt_py, m)?)?;

    // Register submodules (for advanced usage)
    delta::register_module(m)?;
    receipts::register_module(m)?;
    shapes::register_module(m)?;

    Ok(())
}

