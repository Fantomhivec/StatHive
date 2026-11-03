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
use crate::core::i18n::t;
use crate::templates::model::{
    normalize_marketplace, Template, TemplateGroup, TemplatesFile, TemplatesSnapshot,
};
use chrono::Utc;

pub fn snapshot(marketplace: &str) -> Result<TemplatesSnapshot, String> {
    with_file(|file| {
        let m = normalize_marketplace(marketplace);
        let bundle = file.market(&m);
        let tab = bundle.selected_tab.clone();
        let templates: Vec<crate::templates::TemplateCard> = bundle
            .templates
            .iter()
            .filter(|t| t.tab_name == tab)
            .map(crate::templates::TemplateCard::from)
            .collect();
        let groups: Vec<TemplateGroup> = bundle
            .groups
            .iter()
            .filter(|g| g.tab_name == tab)
            .cloned()
            .collect();
        let in_group: std::collections::HashSet<&str> = groups
            .iter()
            .flat_map(|g| g.template_ids.iter().map(|s| s.as_str()))
            .collect();
        let standalone: Vec<crate::templates::TemplateCard> = templates
            .iter()
            .filter(|t| !in_group.contains(t.id.as_str()))
            .cloned()
            .collect();
        TemplatesSnapshot {
            marketplace: m,
            tabs: bundle.tabs.clone(),
            selected_tab: bundle.selected_tab.clone(),
            default_tab: bundle.default_tab.clone(),
            templates,
            groups,
            standalone,
        }
    })
}

pub fn get(id: &str) -> Result<Option<Template>, String> {
    with_file(|file| {
        file.wb
            .templates
            .iter()
            .chain(file.ozon.templates.iter())
            .find(|t| t.id == id)
            .cloned()
    })
}

pub(crate) fn find_market_for_template(file: &TemplatesFile, id: &str) -> Option<String> {
    if file.wb.templates.iter().any(|t| t.id == id) {
        return Some("wb".into());
    }
    if file.ozon.templates.iter().any(|t| t.id == id) {
        return Some("ozon".into());
    }
    None
}

/// Снять «Выгрузка в Excel», залипшую от старого force-save без Google.
/// Только шаблоны с непустым `spreadsheet_id` (цель — Google); осознанный Excel
/// без таблицы не трогаем.
pub fn clear_stuck_excel_exports() -> Result<usize, String> {
    with_file_mut(|file| {
        let mut n = 0usize;
        for t in file
            .wb
            .templates
            .iter_mut()
            .chain(file.ozon.templates.iter_mut())
        {
            if t.export_to_excel && !t.spreadsheet_id.trim().is_empty() {
                t.export_to_excel = false;
                n += 1;
            }
        }
        Ok((n, n > 0))
    })
}

/// Один раз за процесс (участник после HasOauth) — не трогаем каждый snapshot.
pub fn clear_stuck_excel_exports_once() -> Result<usize, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return Ok(0);
    }
    clear_stuck_excel_exports()
}

pub fn upsert(mut tpl: Template) -> Result<Template, String> {
    tpl.touch();
    tpl.normalize();
    tpl.marketplace = normalize_marketplace(&tpl.marketplace);
    if tpl.tab_name.trim().is_empty() {
        tpl.tab_name = t("templates.defaultTab");
    }
    with_file_mut(|file| {
        let market = tpl.marketplace.clone();
        let bundle = file.market_mut(&market);
        if !bundle.tabs.contains(&tpl.tab_name) {
            bundle.tabs.push(tpl.tab_name.clone());
        }
        if let Some(slot) = bundle.templates.iter_mut().find(|t| t.id == tpl.id) {
            *slot = tpl.clone();
        } else {
            file.wb.templates.retain(|t| t.id != tpl.id);
            file.ozon.templates.retain(|t| t.id != tpl.id);
            if tpl.created_at.is_empty() {
                tpl.created_at = tpl.updated_at.clone();
            }
            file.market_mut(&market).templates.push(tpl.clone());
        }
        Ok((tpl, true))
    })
}

pub fn delete(id: &str) -> Result<bool, String> {
    with_file_mut(|file| {
        let before = file.wb.templates.len() + file.ozon.templates.len();
        file.wb.templates.retain(|t| t.id != id);
        file.ozon.templates.retain(|t| t.id != id);
        for g in file.wb.groups.iter_mut().chain(file.ozon.groups.iter_mut()) {
            g.template_ids.retain(|tid| tid != id);
        }
        let after = file.wb.templates.len() + file.ozon.templates.len();
        let changed = after != before;
        Ok((changed, changed))
    })
}

pub fn set_last_run(id: &str, status: &str) -> Result<(), String> {
    with_file_mut(|file| {
        let now = Utc::now().to_rfc3339();
        let mut found = false;
        for t in file
            .wb
            .templates
            .iter_mut()
            .chain(file.ozon.templates.iter_mut())
        {
            if t.id == id {
                t.last_run_status = status.to_string();
                t.last_run_at = now;
                t.touch();
                found = true;
                break;
            }
        }
        Ok(((), found))
    })
}

/// Дописать пустые spreadsheet / лист / название документа (после fallback на app.config).
pub fn backfill_spreadsheet_target(
    id: &str,
    spreadsheet_id: &str,
    sheet_name: &str,
    spreadsheet_title: &str,
) -> Result<bool, String> {
    backfill_spreadsheet_targets(&[(id, spreadsheet_id, sheet_name, spreadsheet_title)])
}

/// Один load/save на пачку backfill (карточки шаблонов после googleListSheets).
pub fn backfill_spreadsheet_targets(
    updates: &[(&str, &str, &str, &str)],
) -> Result<bool, String> {
    if updates.is_empty() {
        return Ok(false);
    }
    with_file_mut(|file| {
        let mut changed = false;
        for &(id, spreadsheet_id, sheet_name, spreadsheet_title) in updates {
            let sid = spreadsheet_id.trim();
            let sheet = sheet_name.trim();
            let title = spreadsheet_title.trim();
            if id.trim().is_empty() || (sid.is_empty() && sheet.is_empty() && title.is_empty()) {
                continue;
            }
            for t in file
                .wb
                .templates
                .iter_mut()
                .chain(file.ozon.templates.iter_mut())
            {
                if t.id != id {
                    continue;
                }
                let mut local = false;
                if t.spreadsheet_id.trim().is_empty() && !sid.is_empty() {
                    t.spreadsheet_id = sid.to_string();
                    local = true;
                }
                if t.sheet_name.trim().is_empty() && !sheet.is_empty() {
                    t.sheet_name = sheet.to_string();
                    local = true;
                }
                if t.spreadsheet_title.trim().is_empty() && !title.is_empty() {
                    t.spreadsheet_title = title.to_string();
                    local = true;
                }
                if local {
                    t.touch();
                    changed = true;
                }
                break;
            }
        }
        Ok((changed, changed))
    })
}
