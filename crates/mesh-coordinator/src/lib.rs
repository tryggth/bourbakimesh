//! BourbakiMesh Distributed Proof Coordinator Library.

pub mod dag;
pub mod diagnostics;
pub mod flight_recorder;
pub mod server;

pub use dag::{ProofDag, Task, TaskQueue};
pub use diagnostics::FailureClass;
pub use flight_recorder::{FlightEvent, FlightRecord, FlightRecorder};
pub use server::{CoordinatorState, MeshCoordinatorServer};
