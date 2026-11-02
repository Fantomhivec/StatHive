//! Метрики карточки / свода (паритет CalculatorCatalog.Metrics).

mod oz;
mod wb;

use crate::calculator::catalog::MetricDef;
use std::sync::OnceLock;

pub use oz::OZ_METRICS;
pub use wb::WB_METRICS;

/// Объединённый срез WB+Ozon (как прежний `METRICS`).
pub fn metrics() -> &'static [MetricDef] {
    static CELL: OnceLock<Vec<MetricDef>> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut v = Vec::with_capacity(WB_METRICS.len() + OZ_METRICS.len());
        v.extend_from_slice(WB_METRICS);
        v.extend_from_slice(OZ_METRICS);
        v
    })
    .as_slice()
}
