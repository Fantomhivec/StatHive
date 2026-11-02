//! Заказы FBS marketplace v3 (`orders_fbs`, `orders_fbs_status`).

pub mod fields;
pub mod orders;
pub mod status;

pub use orders::OrdersFbsScript;
pub use status::OrdersFbsStatusScript;
