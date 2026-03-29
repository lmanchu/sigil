use nostr_sdk::prelude::ToBech32;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use sigil_core::agent::SigilAgent as CoreAgent;
use sigil_core::qr::AgentQrData;
use sigil_core::tui::{ButtonStyle, TuiButton, TuiMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Python bindings for Sigil Agent SDK
///
/// ```python
/// from sigil_agent import SigilAgent
///
/// agent = SigilAgent("my-agent", ["wss://relay.damus.io"])
/// print(agent.npub)
/// print(agent.qr_uri)
/// ```
#[pymodule]
fn sigil_agent(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySigilAgent>()?;
    m.add_class::<PyTuiButtons>()?;
    m.add_class::<PyTuiCard>()?;
    Ok(())
}

#[pyclass]
#[pyo3(name = "SigilAgent")]
struct PySigilAgent {
    inner: Arc<RwLock<CoreAgent>>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PySigilAgent {
    #[new]
    #[pyo3(signature = (name, relays, secret_key=None))]
    fn new(name: String, relays: Vec<String>, secret_key: Option<String>) -> PyResult<Self> {
        let agent = match secret_key {
            Some(key) => CoreAgent::from_key(&name, &key, relays)
                .map_err(|e| PyRuntimeError::new_err(format!("Key error: {}", e)))?,
            None => CoreAgent::new(&name, relays),
        };

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Runtime error: {}", e)))?;

        Ok(Self {
            inner: Arc::new(RwLock::new(agent)),
            runtime,
        })
    }

    /// Get agent's public key in bech32 (npub) format
    #[getter]
    fn npub(&self) -> String {
        self.runtime
            .block_on(async { self.inner.read().await.npub() })
    }

    /// Get agent's secret key in bech32 (nsec) format
    #[getter]
    fn nsec(&self) -> PyResult<String> {
        self.runtime.block_on(async {
            let agent = self.inner.read().await;
            agent
                .keys
                .secret_key()
                .to_bech32()
                .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
        })
    }

    /// Get QR code URI for agent onboarding
    #[getter]
    fn qr_uri(&self) -> String {
        self.runtime.block_on(async {
            let agent = self.inner.read().await;
            let relay = agent.relays.first().cloned().unwrap_or_default();
            AgentQrData {
                npub: agent.npub(),
                relay,
                name: agent.profile.name.clone(),
                capabilities: vec![],
            }
            .to_uri()
        })
    }

    /// Generate QR code as SVG string
    fn qr_svg(&self) -> PyResult<String> {
        self.runtime.block_on(async {
            let agent = self.inner.read().await;
            let relay = agent.relays.first().cloned().unwrap_or_default();
            AgentQrData {
                npub: agent.npub(),
                relay,
                name: agent.profile.name.clone(),
                capabilities: vec![],
            }
            .to_qr_svg()
            .map_err(|e| PyRuntimeError::new_err(format!("QR error: {}", e)))
        })
    }

    /// Send a plain text message to a recipient npub
    fn send(&self, to_npub: String, content: String) -> PyResult<()> {
        self.runtime.block_on(async {
            let agent = self.inner.read().await;
            let to = nostr_sdk::PublicKey::parse(&to_npub)
                .map_err(|e| PyRuntimeError::new_err(format!("Invalid npub: {}", e)))?;
            agent
                .send(to, &content)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Send error: {}", e)))
        })
    }

    /// Send TUI buttons message
    fn send_buttons(
        &self,
        to_npub: String,
        text: String,
        buttons: Vec<(String, String)>,
    ) -> PyResult<()> {
        let tui = TuiMessage::Buttons {
            text: Some(text),
            items: buttons
                .into_iter()
                .map(|(id, label)| TuiButton {
                    id,
                    label,
                    style: Some(ButtonStyle::Primary),
                })
                .collect(),
        };
        let json = tui
            .to_json()
            .map_err(|e| PyRuntimeError::new_err(format!("JSON error: {}", e)))?;
        self.send(to_npub, json)
    }

    fn __repr__(&self) -> String {
        let npub = self
            .runtime
            .block_on(async { self.inner.read().await.npub() });
        format!("SigilAgent(npub={})", &npub[..20.min(npub.len())])
    }
}

// TUI helper classes for Python

#[pyclass]
#[pyo3(name = "TuiButtons")]
struct PyTuiButtons;

#[pymethods]
impl PyTuiButtons {
    #[staticmethod]
    fn create(text: String, buttons: Vec<(String, String)>) -> PyResult<String> {
        let tui = TuiMessage::Buttons {
            text: Some(text),
            items: buttons
                .into_iter()
                .map(|(id, label)| TuiButton {
                    id,
                    label,
                    style: Some(ButtonStyle::Primary),
                })
                .collect(),
        };
        tui.to_json()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
}

#[pyclass]
#[pyo3(name = "TuiCard")]
struct PyTuiCard;

#[pymethods]
impl PyTuiCard {
    #[staticmethod]
    fn create(title: String, description: Option<String>) -> PyResult<String> {
        let tui = TuiMessage::Card {
            title,
            description,
            image_url: None,
            actions: None,
        };
        tui.to_json()
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }
}
