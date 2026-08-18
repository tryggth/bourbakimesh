use kernel::ast::{DeductionStep, Expr, ProofStatus};
use mesh_coordinator::dag::{ProofDag, TaskQueue};

#[test]
fn test_proof_dag_lifecycle_and_kernel_transition() {
    let mut dag = ProofDag::new();

    // Goal: AndComm (h0: A ∧ B ⊢ B ∧ A)
    let initial_hyps = vec![("h0".to_string(), Expr::And(Box::new(Expr::Prop("A".to_string())), Box::new(Expr::Prop("B".to_string()))))];
    let target = Expr::And(Box::new(Expr::Prop("B".to_string())), Box::new(Expr::Prop("A".to_string())));

    let root_id = dag.insert_root("AndComm", initial_hyps, target);
    assert_eq!(dag.roots.len(), 1);

    // Step 1: AndElimR(h0) -> creates child node1 with h1: B
    let step1 = DeductionStep::AndElimR { hyp: "h0".to_string() };
    let (node1_id, status1) = dag.apply_step_and_branch(&root_id, &step1).expect("Step 1 failed");
    assert_eq!(status1, ProofStatus::Open);
    assert_ne!(node1_id, root_id);

    // Step 2: AndElimL(h0) -> creates child node2 with h2: A
    let step2 = DeductionStep::AndElimL { hyp: "h0".to_string() };
    let (node2_id, status2) = dag.apply_step_and_branch(&node1_id, &step2).expect("Step 2 failed");
    assert_eq!(status2, ProofStatus::Open);

    // Step 3: AndIntro(h1, h2) -> derives h3: And(B, A) matching target, immediately Proven
    let step3 = DeductionStep::AndIntro { left: "h1".to_string(), right: "h2".to_string() };
    let (node3_id, status3) = dag.apply_step_and_branch(&node2_id, &step3).expect("Step 3 failed");
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

    let task_id = queue.push_task("node-1".to_string(), "TestTheorem".to_string(), hyps, target, 10);
    assert_eq!(queue.len(), 1);

    let leased = queue.lease_next_task("worker-1", 30).expect("Should lease task");
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
