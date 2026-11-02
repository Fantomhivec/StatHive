//! WB paid storage scripts (create / status / download / auto).

pub mod api;
pub mod auto;
pub mod create;
pub mod download;
pub mod status;

pub use auto::PaidStorageAutoScript;
pub use create::PaidStorageCreateTaskScript;
pub use download::PaidStorageDownloadScript;
pub use status::PaidStorageStatusScript;
