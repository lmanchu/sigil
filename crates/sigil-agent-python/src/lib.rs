use pyo3::prelude::*;

/// Python bindings for Sigil Agent SDK
/// Usage: pip install sigil-agent
///
/// ```python
/// from sigil_agent import SigilAgent
/// agent = SigilAgent("my-agent", ["wss://relay.damus.io"])
/// ```
#[pymodule]
fn sigil_agent(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySigilAgent>()?;
    Ok(())
}

#[pyclass]
#[pyo3(name = "SigilAgent")]
struct PySigilAgent {
    name: String,
    relays: Vec<String>,
}

#[pymethods]
impl PySigilAgent {
    #[new]
    fn new(name: String, relays: Vec<String>) -> Self {
        Self { name, relays }
    }

    /// Get agent's npub (public key in bech32)
    fn npub(&self) -> String {
        // TODO: bridge to sigil-core agent
        format!("npub_placeholder_{}", self.name)
    }
}
