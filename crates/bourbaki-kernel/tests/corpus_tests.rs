//! Integration tests for Mathlib batch corpus decompiler.

use bourbaki_kernel::ast::{Term, Universe};
use bourbaki_kernel::corpus::{CorpusDataset, CorpusDecompiler, RawTheorem};
use bourbaki_kernel::extractor::StrategyExtractor;

#[test]
fn test_batch_corpus_decompilation_and_reextraction() {
    let prop = Term::sort(Universe::Zero);

    let id_thm = RawTheorem {
        name: "id_prop".to_string(),
        type_expr: "A -> A".to_string(),
        prop_type: prop.clone(),
        proof_term: Term::lam("a", prop.clone(), Term::var("a")),
        dependency_count: 0,
    };

    let k_thm = RawTheorem {
        name: "k_comb".to_string(),
        type_expr: "A -> B -> A".to_string(),
        prop_type: prop.clone(),
        proof_term: Term::lam(
            "a",
            prop.clone(),
            Term::lam("b", prop.clone(), Term::var("a")),
        ),
        dependency_count: 1,
    };

    let mp_thm = RawTheorem {
        name: "modus_ponens".to_string(),
        type_expr: "A -> (A -> B) -> B".to_string(),
        prop_type: prop.clone(),
        proof_term: Term::lam(
            "a",
            prop.clone(),
            Term::lam("f", prop.clone(), Term::app(Term::var("f"), Term::var("a"))),
        ),
        dependency_count: 2,
    };

    let raw_batch = vec![id_thm, k_thm, mp_thm];
    let dataset =
        CorpusDecompiler::decompile_batch(&raw_batch).expect("Batch decompilation failed");

    assert_eq!(dataset.theorems.len(), 3);
    assert!(dataset.total_nodes >= 6);

    for thm in &dataset.theorems {
        assert!(thm.trace_depth >= 1);
        assert!(thm.branch_count >= 1);

        // Verify that extracted StrategyTree is valid and can be re-compiled to CIC
        let recompiled = StrategyExtractor::compile_strategy(&thm.strategy)
            .expect("Re-extraction to CIC term failed");
        assert!(!format!("{:?}", recompiled).is_empty());
    }

    // JSON round-trip test
    let json_str = dataset.to_json_string().expect("JSON export failed");
    let loaded_json = CorpusDataset::from_json_str(&json_str).expect("JSON import failed");
    assert_eq!(loaded_json.theorems.len(), 3);
    assert_eq!(loaded_json.total_nodes, dataset.total_nodes);
}

#[test]
fn test_corpus_bincode_roundtrip() {
    let prop = Term::sort(Universe::Zero);
    let id_thm = RawTheorem {
        name: "id_prop".to_string(),
        type_expr: "A -> A".to_string(),
        prop_type: prop.clone(),
        proof_term: Term::lam("a", prop.clone(), Term::var("a")),
        dependency_count: 0,
    };

    let dataset = CorpusDecompiler::decompile_batch(&[id_thm]).unwrap();
    let temp_dir = std::env::temp_dir();
    let bincode_path = temp_dir.join("bourbaki_mathlib_corpus_test.bin");

    dataset
        .save_bincode(&bincode_path)
        .expect("Save bincode failed");
    let loaded = CorpusDataset::load_bincode(&bincode_path).expect("Load bincode failed");

    assert_eq!(loaded.theorems.len(), 1);
    assert_eq!(loaded.theorems[0].name, "id_prop");

    if bincode_path.exists() {
        let _ = std::fs::remove_file(bincode_path);
    }
}
