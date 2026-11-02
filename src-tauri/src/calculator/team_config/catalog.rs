//! Каталог конфигураций калькулятора — лист `CalcTeam` в workspace (SA).

use crate::calculator::profile::{self, CalculatorProfile};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};


use super::types::{
    chunk_json, new_id, new_revision, save_stamp, spreadsheet_id_ws,
    TeamConfigCatalog, TeamConfigEntry, TeamConfigListItem, CALC_TEAM_SHEET,
};

async fn read_raw_payload(client: &GoogleSheetsClient) -> Result<Option<String>, String> {
    let sid = spreadsheet_id_ws()?;
    client.ensure_sheet_exists(&sid, CALC_TEAM_SHEET).await?;
    let rows = client
        .read_values(&sid, &format!("'{CALC_TEAM_SHEET}'!A:A"))
        .await
        .map_err(|e| tf("errors.calc.teamRead", &[("error", &e.to_string())]))?;
    let mut parts: Vec<String> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let cell = row.first().map(|s| s.as_str()).unwrap_or("").trim();
        if i == 0 && (cell.eq_ignore_ascii_case("payload") || cell.is_empty()) {
            continue;
        }
        if cell.is_empty() {
            continue;
        }
        parts.push(cell.to_string());
    }
    if parts.is_empty() {
        return Ok(None);
    }
    Ok(Some(parts.join("")))
}

async fn write_raw_payload(client: &GoogleSheetsClient, raw: &str) -> Result<(), String> {
    let sid = spreadsheet_id_ws()?;
    client.ensure_sheet_exists(&sid, CALC_TEAM_SHEET).await?;
    let chunks = chunk_json(raw);
    client
        .clear_range(&sid, &format!("'{CALC_TEAM_SHEET}'!A:A"))
        .await
        .map_err(|e| tf("errors.calc.teamWrite", &[("error", &e.to_string())]))?;
    let mut rows: Vec<Vec<String>> = vec![vec!["payload".into()]];
    for c in chunks {
        rows.push(vec![c]);
    }
    client
        .write_values(&sid, &format!("'{CALC_TEAM_SHEET}'!A1"), rows)
        .await
        .map_err(|e| tf("errors.calc.teamWrite", &[("error", &e.to_string())]))?;
    Ok(())
}

pub(crate) fn parse_catalog(raw: &str) -> Result<TeamConfigCatalog, String> {
    let v: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| tf("errors.calc.teamParse", &[("error", &e.to_string())]))?;
    if v.get("configs").is_some() || v.get("defaultId").is_some() || v.get("default_id").is_some()
    {
        let mut cat: TeamConfigCatalog = serde_json::from_value(v)
            .map_err(|e| tf("errors.calc.teamParse", &[("error", &e.to_string())]))?;
        if cat.default_id.trim().is_empty() {
            if let Some(first) = cat.configs.first() {
                cat.default_id = first.id.clone();
            }
        }
        for e in &mut cat.configs {
            if e.revision.trim().is_empty() {
                e.revision = new_revision();
            }
            if e.id.trim().is_empty() {
                e.id = new_id();
            }
            if e.name.trim().is_empty() {
                e.name = e
                    .profile
                    .calculator_name
                    .trim()
                    .to_string();
                if e.name.is_empty() {
                    e.name = "Конфигурация".into();
                }
            }
        }
        return Ok(cat);
    }
    // Legacy: голый CalculatorProfile.
    let profile: CalculatorProfile = serde_json::from_value(v)
        .map_err(|e| tf("errors.calc.teamParse", &[("error", &e.to_string())]))?;
    let id = new_id();
    let name = {
        let n = profile.calculator_name.trim();
        if n.is_empty() {
            "Основная".into()
        } else {
            n.to_string()
        }
    };
    Ok(TeamConfigCatalog {
        default_id: id.clone(),
        configs: vec![TeamConfigEntry {
            id,
            name,
            revision: new_revision(),
            profile,
        }],
    })
}

pub async fn load_catalog(client: &GoogleSheetsClient) -> Result<TeamConfigCatalog, String> {
    let Some(raw) = read_raw_payload(client).await? else {
        return Ok(TeamConfigCatalog::default());
    };
    parse_catalog(&raw)
}

async fn save_catalog(
    client: &GoogleSheetsClient,
    catalog: &TeamConfigCatalog,
) -> Result<(), String> {
    let raw = serde_json::to_string(catalog)
        .map_err(|e| tf("errors.calc.teamSerialize", &[("error", &e.to_string())]))?;
    write_raw_payload(client, &raw).await
}

pub fn list_items(catalog: &TeamConfigCatalog) -> Vec<TeamConfigListItem> {
    catalog
        .configs
        .iter()
        .map(|e| TeamConfigListItem {
            id: e.id.clone(),
            name: e.name.clone(),
            revision: e.revision.clone(),
            is_default: e.id.eq_ignore_ascii_case(catalog.default_id.trim()),
        })
        .collect()
}

pub fn find_by_id<'a>(catalog: &'a TeamConfigCatalog, id: &str) -> Option<&'a TeamConfigEntry> {
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    catalog
        .configs
        .iter()
        .find(|e| e.id.eq_ignore_ascii_case(id))
}

pub fn find_by_name<'a>(catalog: &'a TeamConfigCatalog, name: &str) -> Option<&'a TeamConfigEntry> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    catalog
        .configs
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(name))
}

pub fn resolve_entry<'a>(
    catalog: &'a TeamConfigCatalog,
    config_id: &str,
) -> Option<&'a TeamConfigEntry> {
    let id = config_id.trim();
    if !id.is_empty() {
        if let Some(e) = find_by_id(catalog, id) {
            return Some(e);
        }
    }
    let def = catalog.default_id.trim();
    if !def.is_empty() {
        return find_by_id(catalog, def);
    }
    catalog.configs.first()
}

/// Admin: сохранить локальный профиль под именем.
pub async fn publish(
    client: &GoogleSheetsClient,
    name: &str,
    make_default: bool,
) -> Result<TeamConfigEntry, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(t("errors.calc.teamNameRequired"));
    }
    // В каталог попадает профиль (формулы, ссылки, названия кабинетов). Ключи `.env` не читаются.
    let profile = profile::load();
    let mut catalog = load_catalog(client).await?;
    let revision = new_revision();
    let entry = if let Some(idx) = catalog
        .configs
        .iter()
        .position(|e| e.name.eq_ignore_ascii_case(name))
    {
        let id = catalog.configs[idx].id.clone();
        catalog.configs[idx] = TeamConfigEntry {
            id: id.clone(),
            name: name.to_string(),
            revision: revision.clone(),
            profile: profile.clone(),
        };
        catalog.configs[idx].clone()
    } else {
        let entry = TeamConfigEntry {
            id: new_id(),
            name: name.to_string(),
            revision: revision.clone(),
            profile: profile.clone(),
        };
        catalog.configs.push(entry.clone());
        entry
    };
    if make_default || catalog.default_id.trim().is_empty() {
        catalog.default_id = entry.id.clone();
    }
    save_catalog(client, &catalog).await?;
    Ok(entry)
}

pub async fn set_default(client: &GoogleSheetsClient, id: &str) -> Result<(), String> {
    let mut catalog = load_catalog(client).await?;
    let entry = find_by_id(&catalog, id).ok_or_else(|| t("errors.calc.teamNotFound"))?;
    catalog.default_id = entry.id.clone();
    save_catalog(client, &catalog).await
}

pub async fn delete_config(client: &GoogleSheetsClient, id: &str) -> Result<(), String> {
    let mut catalog = load_catalog(client).await?;
    let before = catalog.configs.len();
    catalog
        .configs
        .retain(|e| !e.id.eq_ignore_ascii_case(id.trim()));
    if catalog.configs.len() == before {
        return Err(t("errors.calc.teamNotFound"));
    }
    if catalog.default_id.eq_ignore_ascii_case(id.trim()) {
        catalog.default_id = catalog
            .configs
            .first()
            .map(|e| e.id.clone())
            .unwrap_or_default();
    }
    save_catalog(client, &catalog).await
}

/// Записать снимок в локальный профиль и метку.
pub fn apply_entry_locally(entry: &TeamConfigEntry) -> Result<(), String> {
    profile::save(&entry.profile)?;
    save_stamp(&entry.id, &entry.revision)?;
    crate::calculator::article_index::clear();
    Ok(())
}

/// Pull по id (пусто → default). `Ok(None)` — каталог пуст.
pub async fn pull_by_id(
    client: &GoogleSheetsClient,
    config_id: &str,
) -> Result<Option<TeamConfigEntry>, String> {
    let catalog = load_catalog(client).await?;
    let Some(entry) = resolve_entry(&catalog, config_id).cloned() else {
        return Ok(None);
    };
    apply_entry_locally(&entry)?;
    Ok(Some(entry))
}
