use crate::core::config::AppConfig;
use crate::core::i18n::{t, tf};
use crate::templates::model::{
    normalize_marketplace, Template, TemplateGroup, TemplatesFile, TemplatesSnapshot,
};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub fn store_path() -> PathBuf {
    AppConfig::config_dir().join("templates.json")
}

fn cache() -> &'static Mutex<Option<TemplatesFile>> {
    static CACHE: OnceLock<Mutex<Option<TemplatesFile>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn load_from_disk() -> Result<TemplatesFile, String> {
    let path = store_path();
    if !path.is_file() {
        return Ok(TemplatesFile::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut file: TemplatesFile =
        serde_json::from_str(&raw).map_err(|e| format!("templates.json: {e}"))?;
    file.normalize();
    Ok(file)
}

fn lock_cache() -> Result<std::sync::MutexGuard<'static, Option<TemplatesFile>>, String> {
    cache()
        .lock()
        .map_err(|_| "templates cache lock".to_string())
}

fn ensure_cached(slot: &mut Option<TemplatesFile>) -> Result<&mut TemplatesFile, String> {
    if slot.is_none() {
        *slot = Some(load_from_disk()?);
    }
    Ok(slot.as_mut().expect("cache filled"))
}

/// Чтение без глубокого clone всего файла.
pub(crate) fn with_file<R>(f: impl FnOnce(&TemplatesFile) -> R) -> Result<R, String> {
    let mut guard = lock_cache()?;
    let file = ensure_cached(&mut guard)?;
    Ok(f(file))
}

/// Мутация на месте; `dirty == true` → запись на диск (кэш уже обновлён).
pub(crate) fn with_file_mut<T>(
    f: impl FnOnce(&mut TemplatesFile) -> Result<(T, bool), String>,
) -> Result<T, String> {
    let mut guard = lock_cache()?;
    let file = ensure_cached(&mut guard)?;
    let (out, dirty) = f(file)?;
    if dirty {
        write_disk(file)?;
    }
    Ok(out)
}

fn write_disk(file: &TemplatesFile) -> Result<(), String> {
    let dir = AppConfig::config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    let compact = super::json_compact::compact_value(value);
    let raw = serde_json::to_string_pretty(&compact).map_err(|e| e.to_string())?;
    fs::write(store_path(), raw).map_err(|e| e.to_string())?;
    Ok(())
}

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

pub fn select_tab(marketplace: &str, tab: &str) -> Result<TemplatesSnapshot, String> {
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if !bundle.tabs.iter().any(|t| t == tab) {
            return Err(tf("errors.templates.tabNotFound", &[("tab", tab)]));
        }
        bundle.selected_tab = tab.to_string();
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

pub fn add_tab(marketplace: &str, name: &str) -> Result<TemplatesSnapshot, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(t("errors.templates.tabNameRequired"));
    }
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if bundle.tabs.iter().any(|t| t == name) {
            return Err(tf("errors.templates.tabAlreadyExists", &[("name", name)]));
        }
        if bundle.tabs.len() >= 20 {
            return Err(t("errors.templates.tooManyTabs"));
        }
        bundle.tabs.push(name.to_string());
        bundle.selected_tab = name.to_string();
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

pub fn rename_tab(marketplace: &str, from: &str, to: &str) -> Result<TemplatesSnapshot, String> {
    let to = to.trim();
    if to.is_empty() {
        return Err(t("errors.templates.newNameRequired"));
    }
    if crate::templates::model::is_system_tab(from) {
        return Err(t("errors.templates.cannotRenameSystemTab"));
    }
    if crate::templates::model::is_system_tab(to) {
        return Err(tf("errors.templates.tabAlreadyExists", &[("name", to)]));
    }
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if !bundle.tabs.iter().any(|t| t == from) {
            return Err(tf("errors.templates.tabNotFound", &[("tab", from)]));
        }
        if from != to && bundle.tabs.iter().any(|t| t == to) {
            return Err(tf("errors.templates.tabAlreadyExists", &[("name", to)]));
        }
        for t in &mut bundle.tabs {
            if t == from {
                *t = to.to_string();
            }
        }
        if bundle.selected_tab == from {
            bundle.selected_tab = to.to_string();
        }
        if bundle.default_tab == from {
            bundle.default_tab = to.to_string();
        }
        for t in &mut bundle.templates {
            if t.tab_name == from {
                t.tab_name = to.to_string();
            }
        }
        for g in &mut bundle.groups {
            if g.tab_name == from {
                g.tab_name = to.to_string();
            }
        }
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

pub fn delete_tab(marketplace: &str, name: &str) -> Result<TemplatesSnapshot, String> {
    if crate::templates::model::is_system_tab(name) {
        return Err(t("errors.templates.cannotDeleteSystemTab"));
    }
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if bundle.tabs.len() <= 1 {
            return Err(t("errors.templates.cannotDeleteLastTab"));
        }
        if name == bundle.default_tab {
            return Err(t("errors.templates.cannotDeleteDefaultTab"));
        }
        if !bundle.tabs.iter().any(|t| t == name) {
            return Err(tf("errors.templates.tabNotFound", &[("tab", name)]));
        }
        bundle.tabs.retain(|t| t != name);
        bundle.templates.retain(|t| t.tab_name != name);
        bundle.groups.retain(|g| g.tab_name != name);
        if bundle.selected_tab == name {
            bundle.selected_tab = bundle.default_tab.clone();
        }
        bundle.ensure_tabs();
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

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
