use crate::core::i18n::t;
use serde::{Deserialize, Serialize};


pub(crate) fn default_marketplace() -> String {
    "wb".into()
}

pub(crate) fn default_tab_name() -> String {
    t("templates.defaultTab")
}

pub(crate) fn calculator_tab_name() -> String {
    t("templates.calculatorTab")
}

/// Системные вкладки: нельзя переименовать / удалить.
pub(crate) fn is_system_tab(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty()
        && (n == default_tab_name() || n == calculator_tab_name())
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_tabs() -> Vec<String> {
    vec![default_tab_name(), calculator_tab_name()]
}

pub(crate) fn default_subjects_locale() -> String {
    "ru".into()
}

pub(crate) fn default_write_sort_direction() -> String {
    "asc".into()
}

pub(crate) fn default_report_period() -> String {
    "weekly".into()
}

pub(crate) fn default_subjects_limit() -> Option<i32> {
    Some(30)
}

pub(crate) fn default_subjects_offset() -> Option<i32> {
    Some(0)
}

/// Фильтры списка предметов WB ( `SubjectsListSettings`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectsListFilters {
    #[serde(default = "default_subjects_locale")]
    pub locale: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_subjects_limit")]
    pub limit: Option<i32>,
    #[serde(default = "default_subjects_offset")]
    pub offset: Option<i32>,
    #[serde(default)]
    pub parent_id: Option<i32>,
}

impl Default for SubjectsListFilters {
    fn default() -> Self {
        Self {
            locale: default_subjects_locale(),
            name: String::new(),
            limit: Some(30),
            offset: Some(0),
            parent_id: None,
        }
    }
}

/// Настройки инкрементальной записи ( `ExportSyncSettings`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportSyncSettings {
    #[serde(
        default,
        alias = "EnableChangeSync",
        alias = "enableChangeSync"
    )]
    pub enable_change_sync: bool,
    #[serde(
        default,
        alias = "KeyField1",
        alias = "keyField1"
    )]
    pub key_field_1: String,
    #[serde(
        default,
        alias = "KeyField2",
        alias = "keyField2"
    )]
    pub key_field_2: String,
    #[serde(
        default,
        alias = "DoNotDeleteOldRows",
        alias = "doNotDeleteOldRows"
    )]
    pub do_not_delete_old_rows: bool,
}

