//! WB supplies cluster: FBS list / FBO list / info / goods / warehouses.

pub mod api;
pub mod fbo;
pub mod goods;
pub mod info;
pub mod list;
pub mod warehouses;

pub use fbo::SuppliesFboScript;
pub use goods::SuppliesGoodsScript;
pub use info::SupplyInfoScript;
pub use list::SuppliesScript;
pub use warehouses::SuppliesWarehousesScript;
