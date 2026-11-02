use crate::templates::trigger::{normalize_triggers, TimeTrigger};
use serde::{Deserialize, Serialize};


use super::filters::{default_marketplace, default_tab_name, default_tabs, default_true};
use super::template::{Template, TemplateCard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateGroup {
    pub id: String,
    pub name: String,
    #[serde(default = "default_marketplace")]
    pub marketplace: String,
    #[serde(default = "default_tab_name")]
    pub tab_name: String,
    #[serde(default)]
    pub template_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub is_expanded: bool,
    #[serde(default)]
    pub last_run_status: String,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default, alias = "TimeTriggers", alias = "timeTriggers")]
    pub time_triggers: Vec<TimeTrigger>,
}

impl TemplateGroup {
    pub fn normalize(&mut self) {
        self.time_triggers = normalize_triggers(std::mem::take(&mut self.time_triggers));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceBundle {
    #[serde(default = "default_tabs")]
    pub tabs: Vec<String>,
    #[serde(default = "default_tab_name")]
    pub selected_tab: String,
    #[serde(default = "default_tab_name")]
    pub default_tab: String,
    #[serde(default)]
    pub templates: Vec<Template>,
    #[serde(default)]
    pub groups: Vec<TemplateGroup>,
}

impl Default for MarketplaceBundle {
    fn default() -> Self {
        let tabs = default_tabs();
        let tab = tabs[0].clone();
        Self {
            tabs,
            selected_tab: tab.clone(),
            default_tab: tab,
            templates: Vec::new(),
            groups: Vec::new(),
        }
    }
}

impl MarketplaceBundle {
    pub fn ensure_tabs(&mut self) {
        use super::filters::{calculator_tab_name, default_tab_name};
        let main = default_tab_name();
        let calc = calculator_tab_name();
        // Системные вкладки всегда впереди, в фиксированном порядке.
        let mut rest: Vec<String> = self
            .tabs
            .drain(..)
            .filter(|t| t != &main && t != &calc)
            .collect();
        self.tabs = vec![main.clone(), calc];
        self.tabs.append(&mut rest);
        if self.default_tab.trim().is_empty() || !self.tabs.contains(&self.default_tab) {
            self.default_tab = main;
        }
        if self.selected_tab.trim().is_empty() || !self.tabs.contains(&self.selected_tab) {
            self.selected_tab = self.default_tab.clone();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplatesFile {
    #[serde(default)]
    pub wb: MarketplaceBundle,
    #[serde(default)]
    pub ozon: MarketplaceBundle,
    /// Legacy flat list (pre-marketplace); migrated into `wb` on load.
    #[serde(default, skip_serializing)]
    pub templates: Vec<Template>,
}

impl TemplatesFile {
    pub fn normalize(&mut self) {
        if !self.templates.is_empty() && self.wb.templates.is_empty() {
            for mut t in self.templates.drain(..) {
                if t.marketplace.trim().is_empty() {
                    t.marketplace = "wb".into();
                }
                if t.tab_name.trim().is_empty() {
                    t.tab_name = default_tab_name();
                }
                if !self.wb.tabs.contains(&t.tab_name) {
                    self.wb.tabs.push(t.tab_name.clone());
                }
                self.wb.templates.push(t);
            }
        }
        self.wb.ensure_tabs();
        self.ozon.ensure_tabs();
    }

    pub fn market_mut(&mut self, marketplace: &str) -> &mut MarketplaceBundle {
        match normalize_marketplace(marketplace).as_str() {
            "ozon" => &mut self.ozon,
            _ => &mut self.wb,
        }
    }

    pub fn market(&self, marketplace: &str) -> &MarketplaceBundle {
        match normalize_marketplace(marketplace).as_str() {
            "ozon" => &self.ozon,
            _ => &self.wb,
        }
    }
}

pub fn normalize_marketplace(m: &str) -> String {
    match m.trim().to_ascii_lowercase().as_str() {
        "ozon" => "ozon".into(),
        _ => "wb".into(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplatesSnapshot {
    pub marketplace: String,
    pub tabs: Vec<String>,
    pub selected_tab: String,
    pub default_tab: String,
    pub templates: Vec<TemplateCard>,
    pub groups: Vec<TemplateGroup>,
    pub standalone: Vec<TemplateCard>,
}
