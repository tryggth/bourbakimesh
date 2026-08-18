use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use kernel::state::ProofState;
use kernel::ast::{DeductionStep, Expr};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WasmStepResult {
    pub status: String,
    pub new_hyp: Option<String>,
    pub hyps: HashMap<String, Expr>,
    pub target: Expr,
}

#[wasm_bindgen]
pub struct WasmProofState {
    inner: ProofState,
}

#[wasm_bindgen]
impl WasmProofState {
    #[wasm_bindgen(constructor)]
    pub fn new(initial_hyps_json: &str, target_json: &str) -> Result<WasmProofState, JsValue> {
        let hyps: Vec<(String, Expr)> = serde_json::from_str(initial_hyps_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid hyps: {}", e)))?;
        let target: Expr = serde_json::from_str(target_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid target: {}", e)))?;
        Ok(WasmProofState { inner: ProofState::new(hyps, target) })
    }

    #[wasm_bindgen]
    pub fn apply_step(&mut self, step_json: &str) -> Result<JsValue, JsValue> {
        let step: DeductionStep = serde_json::from_str(step_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid step AST: {}", e)))?;
        match self.inner.apply_step(&step) {
            Ok(new_hyp_opt) => {
                let res = WasmStepResult {
                    status: format!("{:?}", self.inner.status),
                    new_hyp: new_hyp_opt,
                    hyps: self.inner.hyps.clone(),
                    target: self.inner.target.clone(),
                };
                let serializer = serde_wasm_bindgen::Serializer::json_compatible();
                res.serialize(&serializer).map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(err) => Err(JsValue::from_str(&format!("KernelError: {:?}", err)))
        }
    }

    #[wasm_bindgen]
    pub fn get_state_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }
}
