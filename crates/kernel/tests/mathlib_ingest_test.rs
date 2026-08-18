//! Integration and Ingestion Test Suite for Lean 4 Mathlib Export Bridge & ι-Reduction.

use std::fs;
use std::path::Path;
use std::time::Instant;
use serde::Deserialize;
use kernel::cic::{
    Expr, Environment, LocalContext, check_type, whnf,
};

#[derive(Debug, Clone, Deserialize)]
struct ExportedTheorem {
    name: String,
    #[serde(rename = "type")]
    ty: Expr,
    value: Expr,
}

#[test]
fn test_primitive_iota_reduction_recursors() {
    let env = Environment::default_with_logic();
    let ctx = LocalContext::new();

    // 1. Test Bool.rec on Bool.false and Bool.true
    // Bool.rec {motive} (minor_false : motive false) (minor_true : motive true) Bool.false => minor_false
    let motive_bool = Expr::lam("b", Expr::const_term("Bool", vec![]), Expr::prop());
    let minor_false = Expr::prop();
    let minor_true = Expr::type_0();

    let bool_false_app = Expr::mk_app(
        Expr::const_term("Bool.rec", vec![]),
        vec![
            motive_bool.clone(),
            minor_false.clone(),
            minor_true.clone(),
            Expr::const_term("Bool.false", vec![]),
        ],
    );

    let reduced_false = whnf(&bool_false_app, &env, &ctx);
    assert_eq!(reduced_false, minor_false, "Bool.rec on false should reduce to minor_false");

    let bool_true_app = Expr::mk_app(
        Expr::const_term("Bool.rec", vec![]),
        vec![
            motive_bool,
            minor_false,
            minor_true.clone(),
            Expr::const_term("Bool.true", vec![]),
        ],
    );
    let reduced_true = whnf(&bool_true_app, &env, &ctx);
    assert_eq!(reduced_true, minor_true, "Bool.rec on true should reduce to minor_true");

    // 2. Test Nat.rec on Nat.zero and Nat.succ
    let motive_nat = Expr::lam("n", Expr::const_term("Nat", vec![]), Expr::const_term("Nat", vec![]));
    let minor_zero = Expr::const_term("Nat.zero", vec![]);
    let minor_succ = Expr::lam(
        "n",
        Expr::const_term("Nat", vec![]),
        Expr::lam("ih", Expr::const_term("Nat", vec![]), Expr::BVar(0)),
    );

    let nat_zero_app = Expr::mk_app(
        Expr::const_term("Nat.rec", vec![]),
        vec![
            motive_nat.clone(),
            minor_zero.clone(),
            minor_succ.clone(),
            Expr::const_term("Nat.zero", vec![]),
        ],
    );
    let reduced_nat_zero = whnf(&nat_zero_app, &env, &ctx);
    assert_eq!(reduced_nat_zero, minor_zero, "Nat.rec on zero should reduce to minor_zero");

    let one = Expr::mk_app(Expr::const_term("Nat.succ", vec![]), vec![Expr::const_term("Nat.zero", vec![])]);
    let nat_succ_app = Expr::mk_app(
        Expr::const_term("Nat.rec", vec![]),
        vec![
            motive_nat,
            minor_zero,
            minor_succ,
            one,
        ],
    );
    let reduced_nat_succ = whnf(&nat_succ_app, &env, &ctx);
    assert_eq!(reduced_nat_succ, Expr::const_term("Nat.zero", vec![]));
}

#[test]
fn test_ingest_and_verify_all_mathlib_exports() {
    let env = Environment::default_with_logic();
    let ctx = LocalContext::new();

    let artifact_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("artifacts");

    assert!(artifact_dir.exists(), "Artifacts directory {:?} must exist", artifact_dir);

    let export_files = vec![
        "exported_id_prop.json",
        "exported_k_comb.json",
        "exported_modus_ponens_thm.json",
        "exported_and_intro_thm.json",
        "exported_trans_impl_thm.json",
        "exported_And.swap.json",
        "exported_Or.swap.json",
        "exported_Eq.symm.json",
    ];

    let mut total_latency_us = 0.0;
    let mut verified_count = 0;

    for filename in export_files {
        let file_path = artifact_dir.join(filename);
        if !file_path.exists() {
            println!("Skipping non-existent artifact: {}", filename);
            continue;
        }

        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", filename));

        let thm: ExportedTheorem = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse JSON in {}: {:?}", filename, e));

        // Warm-up run
        let check_res = check_type(&thm.value, &thm.ty, &env, &ctx);
        assert!(
            check_res.is_ok(),
            "Verification failed for {} ({:?}): {:?}",
            thm.name,
            filename,
            check_res
        );

        // Benchmark iterations
        let iters = 100;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = check_type(&thm.value, &thm.ty, &env, &ctx);
        }
        let elapsed = start.elapsed();
        let elapsed_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iters as f64);

        println!(
            " Verified Mathlib export {:<28} in {:>6.2} µs",
            thm.name, elapsed_us
        );

        total_latency_us += elapsed_us;
        verified_count += 1;
    }

    assert!(verified_count >= 5, "Should have verified at least 5 exported theorems");
    let avg_latency = total_latency_us / verified_count as f64;
    println!(
        "\n✅ Mathlib Ingestion Summary: Verified {} theorems. Avg latency: {:.2} µs (Ceiling < 50.0 µs)",
        verified_count, avg_latency
    );
    assert!(
        avg_latency < 50.0,
        "Average verification latency ({:.2} µs) must be < 50.0 µs",
        avg_latency
    );
}
