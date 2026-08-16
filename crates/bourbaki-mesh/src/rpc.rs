//! Backwards-compatible RPC module exporting protocol message types.

pub use crate::protocol::{WorkerCommand, WorkerResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_rpc_serialization() {
        let cmd = WorkerCommand::Heartbeat {
            worker_id: "worker-node-42".into(),
        };
        let json = serde_json::to_string(&cmd).expect("JSON serialization failed");
        let deserialized: WorkerCommand =
            serde_json::from_str(&json).expect("JSON deserialization failed");
        assert_eq!(cmd, deserialized);

        let resp = WorkerResponse::TaskAssigned {
            task_id: Uuid::new_v4(),
            goal_statement: "A -> A".into(),
        };
        let bytes = bincode::serialize(&resp).expect("Bincode serialization failed");
        let deserialized_resp: WorkerResponse =
            bincode::deserialize(&bytes).expect("Bincode deserialization failed");
        assert_eq!(resp, deserialized_resp);
    }
}
