#![forbid(unsafe_code)]
//! In-process `libgit2` repository, commit, and credentialed transport operations.

pub mod commit;
pub mod repo;
pub mod transport;

pub use commit::LibGit2Commit;
pub use repo::LibGit2Repo;
pub use transport::LibGit2Transport;
