//! Server Flight Recorder for Distributed Mesh Telemetry.

use crate::diagnostics::FailureClass;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SERVER_GIT_COMMIT: &str = env!("GIT_COMMIT");

pub fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

pub fn iso8601_now() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;

    let mut year = 1970;
    let mut day_count = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if day_count < days_in_year {
            break;
        }
        day_count -= days_in_year;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &d in &month_days {
        if day_count < d {
            break;
        }
        day_count -= d;
        month += 1;
    }
    let day = day_count + 1;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, h, m, s, millis
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverTelemetry {
    pub tier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub nodes_explored: u32,
    pub depth_reached: u32,
    pub tier1_duration_us: u64,
    pub tier2_duration_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSubmissionRecord {
    pub timestamp: String,
    pub session_id: String,
    pub server_commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_commit: Option<String>,
    pub event_type: String, // "PROOF_SUBMISSION_ACCEPTED" | "PROOF_SUBMISSION_REJECTED"
    pub worker_id: String,
    pub task_id: String,
    pub theorem_name: String,
    pub term_ast: serde_json::Value,
    pub thinking_trace: String,
    pub genrm_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_latency_us: Option<u64>,
    pub server_validation_latency_us: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_telemetry: Option<SolverTelemetry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<FailureClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlightEvent {
    WorkerRegistered {
        worker_id: String,
        model: String,
        vram_limit_mb: u32,
        throughput_tok_s: f64,
    },
    WorkerUnregistered {
        worker_id: String,
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
    pub session_id: String,
    file_path: PathBuf,
    canonical_path: PathBuf,
    files: Mutex<(std::fs::File, std::fs::File)>,
}

impl FlightRecorder {
    pub fn new(log_dir: &Path) -> Result<Self, std::io::Error> {
        create_dir_all(log_dir)?;
        let timestamp = now_micros();
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let file_path = log_dir.join(format!("coordinator_trace_{}.jsonl", timestamp));
        let canonical_path = log_dir.join("coordinator_trace.jsonl");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        let canonical_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&canonical_path)?;

        println!(
            "📝 [FlightRecorder] Initialized session {} at {:?} and {:?}",
            session_id, file_path, canonical_path
        );

        Ok(Self {
            session_id,
            file_path,
            canonical_path,
            files: Mutex::new((file, canonical_file)),
        })
    }

    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    pub fn get_path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn get_timestamped_path(&self) -> &Path {
        &self.file_path
    }

    pub fn record_submission(&self, record: &ProofSubmissionRecord) {
        if let Ok(json_line) = serde_json::to_string(record) {
            if let Ok(mut files) = self.files.lock() {
                let _ = writeln!(files.0, "{}", json_line);
                let _ = files.0.flush();
                let _ = writeln!(files.1, "{}", json_line);
                let _ = files.1.flush();
            }
        }
    }

    pub fn record_event(&self, event: FlightEvent) {
        let event_type = match &event {
            FlightEvent::WorkerRegistered { .. } => "WORKER_REGISTERED",
            FlightEvent::WorkerUnregistered { .. } => "WORKER_UNREGISTERED",
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
            if let Ok(mut files) = self.files.lock() {
                let _ = writeln!(files.0, "{}", json_line);
                let _ = files.0.flush();
                let _ = writeln!(files.1, "{}", json_line);
                let _ = files.1.flush();
            }
        }
    }
}
