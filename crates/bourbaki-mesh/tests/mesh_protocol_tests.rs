//! Integration test suite for BourbakiMesh distributed proof DAG, RPC protocol, and task coordinator.

use bourbaki_ir::{LogicalPayload, Move, Polarity, StrategyNode, StrategyTree};
use bourbaki_mesh::{
    BlockId, LedgerError, MeshCoordinator, ProofBlock, ProofLedger, WorkerCommand, WorkerResponse,
};
use uuid::Uuid;

#[test]
fn test_proof_dag_and_integrity_verification() {
    let mut ledger = ProofLedger::new();

    // 1. Insert Genesis block
    let genesis = ProofBlock::genesis("True");
    let genesis_id = ledger
        .insert_block(genesis.clone())
        .expect("Genesis block insertion must succeed");
    assert_eq!(ledger.len(), 1);

    // 2. Insert Theorem 1 referencing Genesis
    let mut strategy1 = StrategyTree::from_root(Move::root_goal("A -> A"));
    strategy1
        .root
        .as_mut()
        .unwrap()
        .add_child(StrategyNode::new(Move::answer(
            1,
            Polarity::Proponent,
            0,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        )));

    let thm1 = ProofBlock::new(
        vec![genesis_id],
        "Theorem.Identity".into(),
        "A -> A".into(),
        Some(strategy1),
        None,
        true,
        1700000001,
    );
    let thm1_id = ledger
        .insert_block(thm1.clone())
        .expect("Theorem 1 insertion must succeed");
    assert_eq!(ledger.len(), 2);

    // 3. Insert Theorem 2 referencing Theorem 1
    let thm2 = ProofBlock::new(
        vec![thm1_id],
        "Theorem.ModusPonens".into(),
        "A -> (A -> B) -> B".into(),
        None,
        None,
        true,
        1700000002,
    );
    let _thm2_id = ledger
        .insert_block(thm2)
        .expect("Theorem 2 insertion must succeed");
    assert_eq!(ledger.len(), 3);

    // 4. Verify chain integrity
    assert!(ledger.verify_chain_integrity().is_ok());

    // 5. Test parent not found error
    let orphan = ProofBlock::new(
        vec![BlockId::from_bytes([0xff; 32])],
        "Theorem.Orphan".into(),
        "False".into(),
        None,
        None,
        true,
        1700000003,
    );
    let err = ledger.insert_block(orphan);
    assert!(matches!(err, Err(LedgerError::ParentNotFound(_))));
}

#[test]
fn test_worker_task_dispatch_and_proof_acceptance() {
    let mut ledger = ProofLedger::new();
    let genesis = ProofBlock::genesis("True");
    ledger.insert_block(genesis).unwrap();

    let mut coordinator = MeshCoordinator::new(ledger);

    // 1. Dispatch task
    let task_id = coordinator.dispatch_task("A -> A");
    assert_eq!(coordinator.active_tasks.len(), 1);

    // 2. Construct valid winning strategy tree for Identity
    let mut strategy = StrategyTree::from_root(Move::root_goal("A -> A"));
    strategy
        .root
        .as_mut()
        .unwrap()
        .add_child(StrategyNode::new(Move::answer(
            1,
            Polarity::Proponent,
            0,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        )));

    // 3. Worker submits proof
    let cmd = WorkerCommand::SubmitProof {
        task_id,
        strategy: strategy.clone(),
    };
    let resp = coordinator.handle_command(cmd);

    match resp {
        WorkerResponse::ProofAccepted {
            task_id: resp_id,
            block_id,
        } => {
            assert_eq!(task_id, resp_id);
            assert!(coordinator.ledger.contains_block(&block_id));
            let block = coordinator.ledger.get_block(&block_id).unwrap();
            assert_eq!(block.statement, "A -> A");
            assert!(block.certified);
            assert_eq!(coordinator.active_tasks.len(), 0);
        }
        other => panic!("Expected ProofAccepted, got {:?}", other),
    }
}

#[test]
fn test_worker_invalid_strategy_rejection() {
    let ledger = ProofLedger::new();
    let mut coordinator = MeshCoordinator::new(ledger);

    let task_id = coordinator.dispatch_task("Goal");

    // Construct empty strategy (missing root)
    let bad_strategy = StrategyTree::new();

    let cmd = WorkerCommand::SubmitProof {
        task_id,
        strategy: bad_strategy,
    };
    let resp = coordinator.handle_command(cmd);

    match resp {
        WorkerResponse::ProofRejected {
            task_id: resp_id,
            reason,
        } => {
            assert_eq!(task_id, resp_id);
            assert!(reason.contains("Strategy extraction compiler error"));
            assert_eq!(coordinator.ledger.len(), 0);
        }
        other => panic!("Expected ProofRejected, got {:?}", other),
    }
}

#[test]
fn test_mesh_serialization_round_trip() {
    // 1. BlockId serialization
    let block_id = BlockId::from_bytes([42u8; 32]);
    let hex = block_id.to_hex();
    let decoded_id = BlockId::from_hex(&hex).expect("Hex decoding failed");
    assert_eq!(block_id, decoded_id);

    // 2. WorkerCommand serialization
    let cmd = WorkerCommand::ClaimTask {
        task_id: Uuid::new_v4(),
        goal_statement: "A /\\ B -> B /\\ A".into(),
        max_simulations: 1000,
    };
    let json_cmd = serde_json::to_string(&cmd).unwrap();
    let decoded_cmd: WorkerCommand = serde_json::from_str(&json_cmd).unwrap();
    assert_eq!(cmd, decoded_cmd);

    // 3. ProofBlock serialization
    let block = ProofBlock::genesis("Root Theory");
    let bincode_bytes = bincode::serialize(&block).unwrap();
    let decoded_block: ProofBlock = bincode::deserialize(&bincode_bytes).unwrap();
    assert_eq!(block, decoded_block);
}
