//! Delta compute Python bindings
//!
//! This module exposes delta compute functionality to Python:
//! - Reuse decision logic
//! - Jaccard similarity computation
//! - Overlap estimation
//! - Cost model evaluation

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pymodule;
use std::collections::HashSet;

use northroot_engine::delta::{decide_reuse, economic_delta, jaccard_similarity};
use northroot_policy::CostModel;

/// Convert Python dict to CostModel
///
/// Accepts a dict with keys: "c_id", "c_comp", "alpha"
/// Each value can be:
/// - A float (constant cost)
/// - A dict with "type": "constant", "value": float
/// - A dict with "type": "linear", "per_row": float, "base": float
fn dict_to_cost_model(dict: &Bound<'_, PyDict>) -> PyResult<CostModel> {
    let c_id = parse_cost_value(dict.get_item("c_id")?, "c_id")?;
    let c_comp = parse_cost_value(dict.get_item("c_comp")?, "c_comp")?;
    let alpha = parse_cost_value(dict.get_item("alpha")?, "alpha")?;

    Ok(CostModel { c_id, c_comp, alpha })
}

/// Parse a cost value from Python object
fn parse_cost_value(obj: Option<Bound<'_, PyAny>>, field_name: &str) -> PyResult<northroot_policy::CostValue> {
    use northroot_policy::CostValue;

    let obj = obj.ok_or_else(|| PyValueError::new_err(format!("Missing required field: {}", field_name)))?;

    // If it's a float, treat as constant
    if let Ok(value) = obj.extract::<f64>() {
        return Ok(CostValue::Constant { value });
    }

    // If it's a dict, parse the type
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let type_str = dict
            .get_item("type")?
            .and_then(|t| t.extract::<String>().ok())
            .ok_or_else(|| PyValueError::new_err(format!("{}: missing 'type' field", field_name)))?;

        match type_str.as_str() {
            "constant" => {
                let value = dict
                    .get_item("value")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyValueError::new_err(format!("{}: missing 'value' field", field_name)))?;
                Ok(CostValue::Constant { value })
            }
            "linear" => {
                let per_row = dict
                    .get_item("per_row")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyValueError::new_err(format!("{}: missing 'per_row' field", field_name)))?;
                let base = dict
                    .get_item("base")?
                    .and_then(|v| v.extract::<f64>().ok())
                    .ok_or_else(|| PyValueError::new_err(format!("{}: missing 'base' field", field_name)))?;
                Ok(CostValue::Linear { per_row, base })
            }
            _ => Err(PyValueError::new_err(format!(
                "{}: invalid type '{}', expected 'constant' or 'linear'",
                field_name, type_str
            ))),
        }
    } else {
        Err(PyValueError::new_err(format!(
            "{}: expected float or dict, got {}",
            field_name,
            obj.get_type().name()?
        )))
    }
}

/// Decide whether to reuse based on overlap and cost model.
///
/// Implements the reuse rule: Reuse iff J > C_id / (α · C_comp)
///
/// Args:
///     overlap_j: Jaccard overlap [0,1] between prior and current chunk sets
///     cost_model: Cost model dict with keys: c_id, c_comp, alpha
///     row_count: Optional row count for linear cost calculation
///
/// Returns:
///     Tuple of (decision, justification) where:
///     - decision: "reuse", "recompute", or "hybrid"
///     - justification: Dict with overlap_j, alpha, c_id, c_comp, decision
#[pyfunction]
#[pyo3(signature = (overlap_j, cost_model, row_count = None))]
fn decide_reuse_py(
    overlap_j: f64,
    cost_model: &Bound<'_, PyDict>,
    row_count: Option<usize>,
) -> PyResult<PyObject> {
    let cost_model_rust = dict_to_cost_model(cost_model)?;
    let (decision, justification) = decide_reuse(overlap_j, &cost_model_rust, row_count);

    Python::with_gil(|py| {
        let decision_str = match decision {
            northroot_engine::delta::ReuseDecision::Reuse => "reuse",
            northroot_engine::delta::ReuseDecision::Recompute => "recompute",
            northroot_engine::delta::ReuseDecision::Hybrid => "hybrid",
        };

        let justification_dict = PyDict::new_bound(py);
        if let Some(overlap) = justification.overlap_j {
            justification_dict.set_item("overlap_j", overlap)?;
        }
        if let Some(alpha) = justification.alpha {
            justification_dict.set_item("alpha", alpha)?;
        }
        if let Some(c_id) = justification.c_id {
            justification_dict.set_item("c_id", c_id)?;
        }
        if let Some(c_comp) = justification.c_comp {
            justification_dict.set_item("c_comp", c_comp)?;
        }
        if let Some(decision_str_val) = justification.decision {
            justification_dict.set_item("decision", decision_str_val)?;
        }

        let result = PyDict::new_bound(py);
        result.set_item("decision", decision_str)?;
        result.set_item("justification", justification_dict)?;

        Ok(result.to_object(py))
    })
}

/// Compute economic delta (savings estimate) from reuse decision.
///
/// Economic delta is defined as: ΔC ≈ α · C_comp · J - C_id
/// Positive values indicate savings from reuse.
///
/// Args:
///     overlap_j: Jaccard overlap [0,1]
///     cost_model: Cost model dict with keys: c_id, c_comp, alpha
///     row_count: Optional row count for linear cost calculation
///
/// Returns:
///     Economic delta (positive = savings, negative = cost)
#[pyfunction]
#[pyo3(signature = (overlap_j, cost_model, row_count = None))]
fn economic_delta_py(
    overlap_j: f64,
    cost_model: &Bound<'_, PyDict>,
    row_count: Option<usize>,
) -> PyResult<f64> {
    let cost_model_rust = dict_to_cost_model(cost_model)?;
    Ok(economic_delta(overlap_j, &cost_model_rust, row_count))
}

/// Compute Jaccard similarity between two sets.
///
/// Jaccard similarity is defined as: J(U,V) = |U ∩ V| / |U ∪ V|
///
/// Args:
///     set1: First set of chunk identifiers (list of strings)
///     set2: Second set of chunk identifiers (list of strings)
///
/// Returns:
///     Jaccard similarity in [0, 1]
#[pyfunction]
fn jaccard_similarity_py(set1: Vec<String>, set2: Vec<String>) -> PyResult<f64> {
    let set1_rust: HashSet<String> = set1.into_iter().collect();
    let set2_rust: HashSet<String> = set2.into_iter().collect();
    Ok(jaccard_similarity(&set1_rust, &set2_rust))
}

/// Python bindings for delta compute operations
#[pymodule]
fn delta(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decide_reuse_py, m)?)?;
    m.add_function(wrap_pyfunction!(economic_delta_py, m)?)?;
    m.add_function(wrap_pyfunction!(jaccard_similarity_py, m)?)?;
    Ok(())
}

/// Register delta module
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_wrapped(wrap_pymodule!(delta))?;
    Ok(())
}

