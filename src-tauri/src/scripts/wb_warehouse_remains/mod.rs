//! WB warehouse remains: create / status / download / auto.

pub mod api;
pub mod auto;
pub mod create;
pub mod download;
pub mod status;

pub use auto::WarehouseRemainsAutoScript;
pub use create::WarehouseRemainsCreateTaskScript;
pub use download::WarehouseRemainsDownloadScript;
pub use status::WarehouseRemainsStatusScript;
