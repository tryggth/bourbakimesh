use kernel::ast::{DeductionStep, Expr, ProofStatus};
use mesh_coordinator::dag::{ProofDag, TaskQueue};

#[test]
fn test_proof_dag_lifecycle_and_kernel_transition() {
    let mut dag = ProofDag::new();

    // Goal: AndComm (h0: A ∧ B ⊢ B ∧ A)
    let initial_hyps = vec![(
        "h0".to_string(),
        Expr::And(
            Box::new(Expr::Prop("A".to_string())),
            Box::new(Expr::Prop("B".to_string())),
        ),
    )];
    let target = Expr::And(
        Box::new(Expr::Prop("B".to_string())),
        Box::new(Expr::Prop("A".to_string())),
    );

    let root_id = dag.insert_root("AndComm", initial_hyps, target);
    assert_eq!(dag.roots.len(), 1);

    // Step 1: AndElimR(h0) -> creates child node1 with h1: B
    let step1 = DeductionStep::AndElimR {
        hyp: "h0".to_string(),
    };
    let (node1_id, status1) = dag
        .apply_step_and_branch(&root_id, &step1)
        .expect("Step 1 failed");
    assert_eq!(status1, ProofStatus::Open);
    assert_ne!(node1_id, root_id);

    // Step 2: AndElimL(h0) -> creates child node2 with h2: A
    let step2 = DeductionStep::AndElimL {
        hyp: "h0".to_string(),
    };
    let (node2_id, status2) = dag
        .apply_step_and_branch(&node1_id, &step2)
        .expect("Step 2 failed");
    assert_eq!(status2, ProofStatus::Open);

    // Step 3: AndIntro(h1, h2) -> derives h3: And(B, A) matching target, immediately Proven
    let step3 = DeductionStep::AndIntro {
        left: "h1".to_string(),
        right: "h2".to_string(),
    };
    let (node3_id, status3) = dag
        .apply_step_and_branch(&node2_id, &step3)
        .expect("Step 3 failed");
    assert_eq!(status3, ProofStatus::Proven);
    assert_eq!(node3_id, node2_id);

    // Verify root status is updated to Proven
    let root_node = dag.get_node(&root_id).unwrap();
    assert_eq!(root_node.status, ProofStatus::Proven);
}

#[test]
fn test_task_queue_lease_and_expiration() {
    let mut queue = TaskQueue::new();
    let hyps = std::collections::HashMap::new();
    let target = Expr::Prop("B".to_string());

    let task_id = queue.push_task(
        "node-1".to_string(),
        "TestTheorem".to_string(),
        hyps,
        target,
        10,
    );
    assert_eq!(queue.len(), 1);

    let leased = queue
        .lease_next_task("worker-1", 30)
        .expect("Should lease task");
    assert_eq!(leased.task_id, task_id);
    assert_eq!(leased.lease_worker, Some("worker-1".to_string()));

    // When empty, returns None
    let next = queue.lease_next_task("worker-2", 30);
    assert!(next.is_none());

    // Complete task
    let completed = queue.complete_task(&task_id);
    assert!(completed.is_some());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_cic_target_and_failure_attribution() {
    use kernel::cic::expr::Expr as CicExpr;
    use kernel::cic::typecheck::TypeError;
    use mesh_coordinator::diagnostics::FailureClass;
    use mesh_coordinator::flight_recorder::{FlightEvent, FlightRecorder};

    // Test FailureClass mapping
    let mismatch_err = TypeError::TypeMismatch {
        expected: CicExpr::Sort(kernel::cic::expr::Level::Zero),
        got: CicExpr::Sort(kernel::cic::expr::Level::Succ(Box::new(
            kernel::cic::expr::Level::Zero,
        ))),
    };
    let failure_class = FailureClass::from_type_error(&mismatch_err);
    assert!(matches!(failure_class, FailureClass::TypeMismatch { .. }));

    let unbound_err = TypeError::LooseBVar(5);
    let unbound_class = FailureClass::from_type_error(&unbound_err);
    assert!(matches!(
        unbound_class,
        FailureClass::UnboundDeBruijnIndex { index: 5, .. }
    ));

    // Test FlightRecorder
    let temp_dir = std::env::temp_dir().join(format!("test_flight_{}", uuid::Uuid::new_v4()));
    let recorder = FlightRecorder::new(&temp_dir).expect("Recorder init failed");

    recorder.record_event(FlightEvent::WorkerRegistered {
        worker_id: "worker-test".to_string(),
        model: "gemma-4-2b".to_string(),
        vram_limit_mb: 4096,
        throughput_tok_s: 60.0,
    });

    recorder.record_event(FlightEvent::TermRejected {
        task_id: "task-1".to_string(),
        worker_id: "worker-test".to_string(),
        theorem_name: "test_thm".to_string(),
        execution_time_us: 42,
        failure_class: failure_class.clone(),
    });

    use mesh_coordinator::flight_recorder::{
        iso8601_now, ProofSubmissionRecord, SolverTelemetry, SERVER_GIT_COMMIT,
    };

    recorder.record_submission(&ProofSubmissionRecord {
        timestamp: iso8601_now(),
        session_id: recorder.session_id.clone(),
        server_commit: SERVER_GIT_COMMIT.to_string(),
        client_commit: Some("a1b2c3d".to_string()),
        event_type: "PROOF_SUBMISSION_ACCEPTED".to_string(),
        worker_id: "worker-test".to_string(),
        task_id: "task-1".to_string(),
        theorem_name: "test_thm".to_string(),
        term_ast: serde_json::json!({"BVar": 0}),
        thinking_trace: "λ a => a".to_string(),
        genrm_score: 0.99,
        wasm_latency_us: Some(150),
        server_validation_latency_us: 35,
        solver_telemetry: Some(SolverTelemetry {
            tier: "tier1_symbolic".to_string(),
            fallback_reason: None,
            nodes_explored: 1,
            depth_reached: 2,
            tier1_duration_us: 450,
            tier2_duration_us: 0,
        }),
        failure_class: None,
        client_metadata: Some(serde_json::json!({"provider": "webgpu", "vram_allocated_mb": 1850})),
    });

    assert!(recorder.get_path().exists());
    assert!(recorder.get_timestamped_path().exists());

    let content = std::fs::read_to_string(recorder.get_path()).expect("Failed to read trace file");
    assert!(content.contains("PROOF_SUBMISSION_ACCEPTED"));
    assert!(content.contains("worker-test"));
    assert!(content.contains("tier1_symbolic"));
    assert!(content.contains("server_commit"));
    assert!(content.contains("a1b2c3d"));
    assert!(content.contains("webgpu"));

    let _ = std::fs::remove_dir_all(temp_dir);
}
