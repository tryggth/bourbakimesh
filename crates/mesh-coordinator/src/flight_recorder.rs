//! Server Flight Recorder for Distributed Mesh Telemetry.

use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::diagnostics::FailureClass;

pub fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlightEvent {
    WorkerRegistered {
        worker_id: String,
        model: String,
        vram_limit_mb: u32,
        throughput_tok_s: f64,
    },
    TaskLeased {
        task_id: String,
        node_id: String,
        worker_id: String,
        theorem_name: String,
    },
    ResultSubmitted {
        task_id: String,
        worker_id: String,
        term_json: serde_json::Value,
        genrm_score: f64,
    },
    TermValidated {
        task_id: String,
        worker_id: String,
        theorem_name: String,
        execution_time_us: u64,
        inferred_type: Option<String>,
    },
    TermRejected {
        task_id: String,
        worker_id: String,
        theorem_name: String,
        execution_time_us: u64,
        failure_class: FailureClass,
    },
    LeaseExpired {
        task_id: String,
        worker_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightRecord {
    pub timestamp_us: u64,
    pub event_type: String,
    pub event: FlightEvent,
}

pub struct FlightRecorder {
    file_path: PathBuf,
    file_lock: Mutex<std::fs::File>,
}

impl FlightRecorder {
    pub fn new(log_dir: &Path) -> Result<Self, std::io::Error> {
        create_dir_all(log_dir)?;
        let timestamp = now_micros();
        let file_path = log_dir.join(format!("coordinator_trace_{}.jsonl", timestamp));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        println!("📝 [FlightRecorder] Initialized trace file at {:?}", file_path);

        Ok(Self {
            file_path,
            file_lock: Mutex::new(file),
        })
    }

    pub fn get_path(&self) -> &Path {
        &self.file_path
    }

    pub fn record_event(&self, event: FlightEvent) {
        let event_type = match &event {
            FlightEvent::WorkerRegistered { .. } => "WORKER_REGISTERED",
            FlightEvent::TaskLeased { .. } => "TASK_LEASED",
            FlightEvent::ResultSubmitted { .. } => "RESULT_SUBMITTED",
            FlightEvent::TermValidated { .. } => "TERM_VALIDATED",
            FlightEvent::TermRejected { .. } => "TERM_REJECTED",
            FlightEvent::LeaseExpired { .. } => "LEASE_EXPIRED",
        }
        .to_string();

        let record = FlightRecord {
            timestamp_us: now_micros(),
            event_type,
            event,
        };

        if let Ok(json_line) = serde_json::to_string(&record) {
            if let Ok(mut file) = self.file_lock.lock() {
                let _ = writeln!(file, "{}", json_line);
                let _ = file.flush();
            }
        }
    }
}
