//! Replication protocol for safely synchronizing logs.

pub use protocol::{Options, Duplex};

mod replication;
pub use replication::Replication;

mod handle;
pub use handle::{Command, ReplicationHandle};

mod replica_trait;
pub use replica_trait::{ReplicaTrait, Data, Request, DataOrRequest};

mod core_replica;
pub use core_replica::CoreReplica;
