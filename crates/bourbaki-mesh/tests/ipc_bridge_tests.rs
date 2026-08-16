//! Integration tests for asynchronous Tokio IPC server and mesh networking.

use bourbaki_ir::{LogicalPayload, Move, Polarity, StrategyNode, StrategyTree};
use bourbaki_mesh::{
    MeshCoordinator, MeshIpcServer, ProofBlock, ProofLedger, WorkerCommand, WorkerResponse,
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use uuid::Uuid;

#[tokio::test]
async fn test_async_ipc_server_tcp_roundtrip() {
    let mut ledger = ProofLedger::new();
    let genesis = ProofBlock::genesis("True");
    ledger.insert_block(genesis).unwrap();

    let coordinator = Arc::new(Mutex::new(MeshCoordinator::new(ledger)));
    let (server, local_addr) = MeshIpcServer::bind_tcp("127.0.0.1:0", coordinator.clone())
        .await
        .expect("Failed to bind TCP server");

    let stream = TcpStream::connect(&local_addr)
        .await
        .expect("Client failed to connect to IPC server");
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    // 1. Send Heartbeat
    let hb_cmd = WorkerCommand::Heartbeat {
        worker_id: "worker-test-1".into(),
    };
    let mut hb_json = serde_json::to_string(&hb_cmd).unwrap();
    hb_json.push('\n');
    writer.write_all(hb_json.as_bytes()).await.unwrap();

    let resp_line = lines.next_line().await.unwrap().expect("Missing response");
    let resp: WorkerResponse = serde_json::from_str(&resp_line).unwrap();
    assert_eq!(resp, WorkerResponse::Acknowledged);

    // 2. Claim Task
    let task_id = Uuid::new_v4();
    let claim_cmd = WorkerCommand::ClaimTask {
        task_id,
        goal_statement: "A -> A".into(),
        max_simulations: 50,
    };
    let mut claim_json = serde_json::to_string(&claim_cmd).unwrap();
    claim_json.push('\n');
    writer.write_all(claim_json.as_bytes()).await.unwrap();

    let claim_resp_line = lines.next_line().await.unwrap().expect("Missing response");
    let claim_resp: WorkerResponse = serde_json::from_str(&claim_resp_line).unwrap();
    match claim_resp {
        WorkerResponse::TaskAssigned {
            task_id: assigned_id,
            goal_statement,
        } => {
            assert_eq!(task_id, assigned_id);
            assert_eq!(goal_statement, "A -> A");
        }
        other => panic!("Expected TaskAssigned, got {:?}", other),
    }

    // 3. Submit Proof
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

    let proof_cmd = WorkerCommand::SubmitProof { task_id, strategy };
    let mut proof_json = serde_json::to_string(&proof_cmd).unwrap();
    proof_json.push('\n');
    writer.write_all(proof_json.as_bytes()).await.unwrap();

    let proof_resp_line = lines.next_line().await.unwrap().expect("Missing response");
    let proof_resp: WorkerResponse = serde_json::from_str(&proof_resp_line).unwrap();
    match proof_resp {
        WorkerResponse::ProofAccepted {
            task_id: accepted_id,
            block_id,
        } => {
            assert_eq!(task_id, accepted_id);
            let guard = coordinator.lock().await;
            assert!(guard.ledger.contains_block(&block_id));
        }
        other => panic!("Expected ProofAccepted, got {:?}", other),
    }

    server.shutdown();
}
