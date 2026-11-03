// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::io::{with_file, with_file_mut};
use super::templates::find_market_for_template;
use crate::core::i18n::t;
use crate::templates::model::{normalize_marketplace, TemplateGroup};
use chrono::Utc;

pub fn upsert_group(mut group: TemplateGroup) -> Result<TemplateGroup, String> {
    group.marketplace = normalize_marketplace(&group.marketplace);
    group.normalize();
    if group.tab_name.trim().is_empty() {
        group.tab_name = t("templates.defaultTab");
    }
    if group.name.trim().is_empty() {
        return Err(t("errors.templates.groupNameRequired"));
    }
    with_file_mut(|file| {
        let market = group.marketplace.clone();
        let bundle = file.market_mut(&market);
        if !bundle.tabs.contains(&group.tab_name) {
            bundle.tabs.push(group.tab_name.clone());
        }
        let valid: std::collections::HashSet<String> = bundle
            .templates
            .iter()
            .filter(|t| t.tab_name == group.tab_name)
            .map(|t| t.id.clone())
            .collect();
        group.template_ids.retain(|id| valid.contains(id));
        if let Some(slot) = bundle.groups.iter_mut().find(|g| g.id == group.id) {
            *slot = group.clone();
        } else {
            file.wb.groups.retain(|g| g.id != group.id);
            file.ozon.groups.retain(|g| g.id != group.id);
            file.market_mut(&market).groups.push(group.clone());
        }
        Ok((group, true))
    })
}

pub fn delete_group(id: &str) -> Result<bool, String> {
    with_file_mut(|file| {
        let before = file.wb.groups.len() + file.ozon.groups.len();
        file.wb.groups.retain(|g| g.id != id);
        file.ozon.groups.retain(|g| g.id != id);
        let after = file.wb.groups.len() + file.ozon.groups.len();
        let changed = after != before;
        Ok((changed, changed))
    })
}

pub fn get_group(id: &str) -> Result<Option<TemplateGroup>, String> {
    with_file(|file| {
        file.wb
            .groups
            .iter()
            .chain(file.ozon.groups.iter())
            .find(|g| g.id == id)
            .cloned()
    })
}

pub fn set_group_expanded(id: &str, expanded: bool) -> Result<(), String> {
    with_file_mut(|file| {
        for g in file.wb.groups.iter_mut().chain(file.ozon.groups.iter_mut()) {
            if g.id == id {
                g.is_expanded = expanded;
                return Ok(((), true));
            }
        }
        Err(t("errors.templates.groupNotFound"))
    })
}

pub fn add_to_group(template_id: &str, group_id: &str) -> Result<(), String> {
    with_file_mut(|file| {
        let market = find_market_for_template(file, template_id)
            .ok_or_else(|| t("errors.templates.templateNotFound"))?;
        let bundle = file.market_mut(&market);
        let tab = bundle
            .templates
            .iter()
            .find(|t| t.id == template_id)
            .map(|t| t.tab_name.clone())
            .ok_or_else(|| t("errors.templates.templateNotFound"))?;
        for g in &mut bundle.groups {
            g.template_ids.retain(|id| id != template_id);
        }
        let group = bundle
            .groups
            .iter_mut()
            .find(|g| g.id == group_id)
            .ok_or_else(|| t("errors.templates.groupNotFound"))?;
        if group.tab_name != tab {
            return Err(t("errors.templates.groupWrongTab"));
        }
        if !group.template_ids.iter().any(|id| id == template_id) {
            group.template_ids.push(template_id.to_string());
        }
        Ok(((), true))
    })
}

pub fn set_group_last_run(id: &str, status: &str) -> Result<(), String> {
    with_file_mut(|file| {
        let now = Utc::now().to_rfc3339();
        for g in file.wb.groups.iter_mut().chain(file.ozon.groups.iter_mut()) {
            if g.id == id {
                g.last_run_status = status.to_string();
                g.last_run_at = now;
                return Ok(((), true));
            }
        }
        Ok(((), false))
    })
}

pub fn selected_tab_for(marketplace: &str) -> Result<String, String> {
    with_file(|file| file.market(marketplace).selected_tab.clone())
}
