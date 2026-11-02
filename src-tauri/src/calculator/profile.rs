//! Профиль калькулятора: `wb_calculator_settings.json`.

use crate::core::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WbKgtRules {
    #[serde(default = "default_wb_side")]
    pub max_one_side_cm: f64,
    #[serde(default = "default_gte")]
    pub one_side_cmp: String,
    #[serde(default = "default_wb_sum")]
    pub max_sum_sides_cm: f64,
    #[serde(default = "default_gte")]
    pub sum_sides_cmp: String,
    #[serde(default = "default_wb_weight")]
    pub max_weight_kg: f64,
    #[serde(default = "default_gte")]
    pub weight_cmp: String,
}

fn default_wb_side() -> f64 {
    120.0
}
fn default_wb_sum() -> f64 {
    200.0
}
fn default_wb_weight() -> f64 {
    25.0
}
fn default_gte() -> String {
    "gte".into()
}
fn default_gt() -> String {
    "gt".into()
}

impl Default for WbKgtRules {
    fn default() -> Self {
        Self {
            max_one_side_cm: 120.0,
            one_side_cmp: "gte".into(),
            max_sum_sides_cm: 200.0,
            sum_sides_cmp: "gte".into(),
            max_weight_kg: 25.0,
            weight_cmp: "gte".into(),
        }
    }
}

impl WbKgtRules {
    pub fn is_sgt(&self, len: f64, wid: f64, hei: f64, weight_kg: f64) -> bool {
        let max_side = len.max(wid).max(hei);
        let sum = len + wid + hei;
        kgt_meets(max_side, self.max_one_side_cm, &self.one_side_cmp, "gte")
            || kgt_meets(sum, self.max_sum_sides_cm, &self.sum_sides_cmp, "gte")
            || kgt_meets(weight_kg, self.max_weight_kg, &self.weight_cmp, "gte")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OzKgtRules {
    #[serde(default = "default_oz_sum")]
    pub max_sum_sides: f64,
    #[serde(default = "default_gt")]
    pub sum_sides_cmp: String,
    #[serde(default = "default_oz_weight")]
    pub max_weight: f64,
    #[serde(default = "default_gt")]
    pub weight_cmp: String,
    #[serde(default = "default_oz_volume")]
    pub max_volume: f64,
    #[serde(default = "default_gt")]
    pub volume_cmp: String,
    #[serde(default = "default_oz_bonus")]
    pub bonus: f64,
}

fn default_oz_sum() -> f64 {
    200.0
}
fn default_oz_weight() -> f64 {
    35.0
}
fn default_oz_volume() -> f64 {
    500.0
}
fn default_oz_bonus() -> f64 {
    280.0
}

impl Default for OzKgtRules {
    fn default() -> Self {
        Self {
            max_sum_sides: 200.0,
            sum_sides_cmp: "gt".into(),
            max_weight: 35.0,
            weight_cmp: "gt".into(),
            max_volume: 500.0,
            volume_cmp: "gt".into(),
            bonus: 280.0,
        }
    }
}

impl OzKgtRules {
    pub fn is_kgt(&self, len: f64, wid: f64, hei: f64, weight: f64, volume: f64) -> bool {
        if kgt_meets(weight, self.max_weight, &self.weight_cmp, "gt") {
            return true;
        }
        if len > 0.0
            && wid > 0.0
            && hei > 0.0
            && kgt_meets(len + wid + hei, self.max_sum_sides, &self.sum_sides_cmp, "gt")
        {
            return true;
        }
        kgt_meets(volume, self.max_volume, &self.volume_cmp, "gt")
    }
}

pub fn kgt_normalize(raw: &str, fallback: &str) -> String {
    let s = raw.trim().to_ascii_lowercase();
    if s == "gt" || s == "gte" {
        s
    } else {
        fallback.to_string()
    }
}

pub fn kgt_meets(value: f64, threshold: f64, cmp: &str, fallback: &str) -> bool {
    let mode = kgt_normalize(cmp, fallback);
    if mode == "gte" {
        value >= threshold
    } else {
        value > threshold
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalculatorColumnBinding {
    #[serde(default)]
    pub metric_key: String,
    #[serde(default)]
    pub spreadsheet_url: String,
    #[serde(default)]
    pub sheet_name: String,
    #[serde(default)]
    pub column_header: String,
    #[serde(default = "default_article_header")]
    pub article_column_header: String,
}

fn default_article_header() -> String {
    "Артикул ВБ".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalculatorTriggerSettings {
    #[serde(default)]
    pub table_key: String,
    #[serde(default)]
    pub display_name: String,
    /// None | Once | Daily | Weekly | Interval
    #[serde(default = "default_trigger_mode")]
    pub mode: String,
    #[serde(default = "default_trigger_time")]
    pub time: String,
    #[serde(default)]
    pub interval_hours: i32,
    #[serde(default)]
    pub interval_minutes: i32,
    #[serde(default)]
    pub next_execution: String,
    /// Once: ISO local datetime string.
    #[serde(default)]
    pub scheduled_date_time: String,
    #[serde(default)]
    pub weekly_day: String,
    #[serde(default)]
    pub weekly_time: String,
}

fn default_trigger_mode() -> String {
    "None".into()
}
fn default_trigger_time() -> String {
    "09:00".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculatorProfile {
    #[serde(default = "new_bundle_id")]
    pub bundle_id: String,
    #[serde(default = "default_calc_name")]
    pub calculator_name: String,
    #[serde(default = "default_data_source")]
    pub data_source_mode: String,
    #[serde(default = "default_marketplace")]
    pub marketplace: String,
    #[serde(default)]
    pub triggers_enabled: bool,
    #[serde(default)]
    pub triggers: Vec<CalculatorTriggerSettings>,
    #[serde(default)]
    pub table_modes: HashMap<String, String>,
    #[serde(default)]
    pub selected_cabinets: Vec<String>,
    #[serde(default)]
    pub cabinet_marketplaces: HashMap<String, String>,
    #[serde(default)]
    pub table_links: HashMap<String, String>,
    #[serde(default)]
    pub last_updated: HashMap<String, String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub tax_percent: f64,
    #[serde(default)]
    pub acquiring_percent: f64,
    #[serde(default)]
    pub adv_percent: f64,
    #[serde(default)]
    pub extra_expenses_percent: f64,
    #[serde(default)]
    pub ozon_tax_percent: f64,
    #[serde(default)]
    pub ozon_acquiring_percent: f64,
    #[serde(default)]
    pub ozon_adv_percent: f64,
    #[serde(default)]
    pub ozon_extra_expenses_percent: f64,
    #[serde(default)]
    pub profit_formula: String,
    #[serde(default)]
    pub ozon_profit_formula: String,
    #[serde(default)]
    pub wb_kgt: WbKgtRules,
    #[serde(default)]
    pub oz_kgt: OzKgtRules,
    #[serde(default)]
    pub column_bindings: Vec<CalculatorColumnBinding>,
    #[serde(default)]
    pub template_ids_by_table_key: HashMap<String, String>,
}

fn new_bundle_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
fn default_calc_name() -> String {
    "калькулятор StatHive".into()
}
fn default_data_source() -> String {
    "auto".into()
}
fn default_marketplace() -> String {
    "wb".into()
}

impl Default for CalculatorProfile {
    fn default() -> Self {
        Self {
            bundle_id: new_bundle_id(),
            calculator_name: default_calc_name(),
            data_source_mode: default_data_source(),
            marketplace: default_marketplace(),
            triggers_enabled: false,
            triggers: Vec::new(),
            table_modes: HashMap::new(),
            selected_cabinets: Vec::new(),
            cabinet_marketplaces: HashMap::new(),
            table_links: HashMap::new(),
            last_updated: HashMap::new(),
            folder_id: None,
            tax_percent: 0.0,
            acquiring_percent: 0.0,
            adv_percent: 0.0,
            extra_expenses_percent: 0.0,
            ozon_tax_percent: 0.0,
            ozon_acquiring_percent: 0.0,
            ozon_adv_percent: 0.0,
            ozon_extra_expenses_percent: 0.0,
            profit_formula: String::new(),
            ozon_profit_formula: String::new(),
            wb_kgt: WbKgtRules::default(),
            oz_kgt: OzKgtRules::default(),
            column_bindings: Vec::new(),
            template_ids_by_table_key: HashMap::new(),
        }
    }
}

pub fn settings_path() -> PathBuf {
    AppConfig::config_dir().join("wb_calculator_settings.json")
}

pub fn load() -> CalculatorProfile {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => CalculatorProfile::default(),
    }
}

pub fn save(profile: &CalculatorProfile) -> Result<(), String> {
    let dir = AppConfig::config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    fs::write(settings_path(), raw).map_err(|e| e.to_string())
}
