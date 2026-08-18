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

#[derive(Serialize, Deserialize)]
pub struct WasmMathlibVerifyResult {
    pub name: String,
    pub valid: bool,
    pub inferred_type: String,
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

/// Validates that a Calculus of Inductive Constructions (CIC) proof term has the expected goal type.
#[wasm_bindgen]
pub fn check_cic_term(
    context_json: &str,
    proof_term_json: &str,
    goal_type_json: &str,
) -> Result<bool, JsValue> {
    let hyps: Vec<(String, kernel::cic::Expr)> = serde_json::from_str(context_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid context JSON: {}", e)))?;
    let proof_term: kernel::cic::Expr = serde_json::from_str(proof_term_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid proof term JSON: {}", e)))?;
    let goal_type: kernel::cic::Expr = serde_json::from_str(goal_type_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid goal type JSON: {}", e)))?;

    let env = kernel::cic::Environment::default_with_logic();
    let mut ctx = kernel::cic::LocalContext::new();
    for (id, ty) in hyps {
        ctx = ctx.extend(&id, &id, ty);
    }

    match kernel::cic::check_type(&proof_term, &goal_type, &env, &ctx) {
        Ok(()) => Ok(true),
        Err(err) => Err(JsValue::from_str(&format!("TypeError: {:?}", err))),
    }
}

/// Infers the CIC type of a term under a local context.
#[wasm_bindgen]
pub fn infer_cic_type(
    context_json: &str,
    term_json: &str,
) -> Result<String, JsValue> {
    let hyps: Vec<(String, kernel::cic::Expr)> = serde_json::from_str(context_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid context JSON: {}", e)))?;
    let term: kernel::cic::Expr = serde_json::from_str(term_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid term JSON: {}", e)))?;

    let env = kernel::cic::Environment::default_with_logic();
    let mut ctx = kernel::cic::LocalContext::new();
    for (id, ty) in hyps {
        ctx = ctx.extend(&id, &id, ty);
    }

    match kernel::cic::infer_type(&term, &env, &ctx) {
        Ok(inferred) => serde_json::to_string(&inferred)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        Err(err) => Err(JsValue::from_str(&format!("TypeError: {:?}", err))),
    }
}

/// Verifies a complete Lean 4 / Mathlib exported theorem JSON payload.
#[wasm_bindgen]
pub fn verify_mathlib_export(export_json: &str) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct Payload {
        name: String,
        #[serde(rename = "type")]
        ty: kernel::cic::Expr,
        value: kernel::cic::Expr,
    }

    let payload: Payload = serde_json::from_str(export_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid export JSON: {}", e)))?;

    let env = kernel::cic::Environment::default_with_logic();
    let ctx = kernel::cic::LocalContext::new();

    match kernel::cic::check_type(&payload.value, &payload.ty, &env, &ctx) {
        Ok(()) => {
            let inferred = kernel::cic::infer_type(&payload.value, &env, &ctx)
                .map(|t| serde_json::to_string(&t).unwrap_or_default())
                .unwrap_or_default();
            let res = WasmMathlibVerifyResult {
                name: payload.name,
                valid: true,
                inferred_type: inferred,
            };
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            res.serialize(&serializer).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        Err(err) => Err(JsValue::from_str(&format!("TypeError: {:?}", err))),
    }
}
