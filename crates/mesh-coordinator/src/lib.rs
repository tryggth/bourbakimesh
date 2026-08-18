//! BourbakiMesh Distributed Proof Coordinator Library.

pub mod dag;
pub mod server;

pub use dag::{ProofDag, Task, TaskQueue};
pub use server::{MeshCoordinatorServer, CoordinatorState};
