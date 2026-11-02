//! Лист «Пресеты» в workspace spreadsheet — единственный каталог прав (кроме builtin Admin).

use crate::core::access::set_presets_cache;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use std::collections::HashMap;

/// Лист Users — remap AccessLevel при смене id пресетов (без зависимости на auth).
const USERS_SHEET: &str = "Пользователи";

use super::flags::{
    app_access_cell, calc_access_cell, calc_cell, calc_from_cell, calculator_bool_from_flags, cell,
    parse_app_access_cell, parse_calc_access_cell, parse_scripts_cell, scripts_cell, PresetDef,
    ADMIN_ROLE_ID, ALL_CALC_FLAGS, HEADER, PRESETS_SHEET,
};

fn parse_preset_row(row: &[String]) -> Option<PresetDef> {
    let id = cell(row, 0);
    if id.is_empty() {
        return None;
    }
    let calculator_legacy = calc_from_cell(&cell(row, 2));
    let calc_access = parse_calc_access_cell(&cell(row, 4), calculator_legacy);
    let calculator = calculator_bool_from_flags(&calc_access);
    Some(PresetDef {
        id,
        title: {
            let t = cell(row, 1);
            if t.is_empty() {
                cell(row, 0)
            } else {
                t
            }
        },
        calculator,
        scripts: parse_scripts_cell(&cell(row, 3)),
        calc_access,
        app_access: parse_app_access_cell(&cell(row, 5)),
    })
}

fn preset_to_row(p: &PresetDef) -> Vec<String> {
    let flags = if p.calc_access.is_empty() && p.calculator {
        ALL_CALC_FLAGS.iter().map(|s| (*s).to_string()).collect()
    } else {
        p.calc_access.clone()
    };
    vec![
        p.id.clone(),
        p.title.clone(),
        calc_cell(calculator_bool_from_flags(&flags)),
        scripts_cell(&p.scripts),
        calc_access_cell(&flags),
        app_access_cell(&p.app_access),
    ]
}

fn is_numeric_preset_id(id: &str) -> bool {
    let t = id.trim();
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
}

fn needs_numeric_migrate(presets: &[PresetDef]) -> bool {
    presets.iter().any(|p| !is_numeric_preset_id(&p.id))
}

fn next_numeric_preset_id(existing: &[PresetDef]) -> String {
    let mut max = 0u32;
    for p in existing {
        if let Ok(n) = p.id.trim().parse::<u32>() {
            max = max.max(n);
        }
    }
    let mut n = max.saturating_add(1).max(1);
    loop {
        let cand = n.to_string();
        if !cand.eq_ignore_ascii_case(ADMIN_ROLE_ID)
            && !existing.iter().any(|p| p.id == cand)
        {
            return cand;
        }
        n = n.saturating_add(1);
    }
}

async fn write_header(client: &GoogleSheetsClient, sid: &str, sheet: &str) -> Result<(), String> {
    let range = format!("'{sheet}'!A1:F1");
    client
        .write_values(
            sid,
            &range,
            vec![HEADER.iter().map(|s| (*s).to_string()).collect()],
        )
        .await
}

async fn read_data_rows(
    client: &GoogleSheetsClient,
    sid: &str,
    sheet: &str,
) -> Result<Vec<Vec<String>>, String> {
    let range = format!("'{sheet}'!A2:F500");
    client.read_values(sid, &range).await.map_err(|e| {
        tf(
            "errors.access.presetReadFailed",
            &[("sheet", sheet), ("error", &e.to_string())],
        )
    })
}

async fn rewrite_sheet(
    client: &GoogleSheetsClient,
    sid: &str,
    sheet: &str,
    rows: Vec<Vec<String>>,
) -> Result<(), String> {
    write_header(client, sid, sheet).await?;
    let clear_rows = 200usize.max(rows.len() + 5);
    let empty: Vec<Vec<String>> = (0..clear_rows)
        .map(|_| {
            vec![
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]
        })
        .collect();
    client
        .write_values(sid, &format!("'{sheet}'!A2:F{}", clear_rows + 1), empty)
        .await?;
    if rows.is_empty() {
        return Ok(());
    }
    let end = rows.len() + 1;
    client
        .write_values(sid, &format!("'{sheet}'!A2:F{end}"), rows)
        .await
}

/// Перенумеровать slug-id → 1..n и обновить AccessLevel у пользователей.
async fn migrate_presets_to_numeric(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    presets: Vec<PresetDef>,
) -> Result<Vec<PresetDef>, String> {
    if !needs_numeric_migrate(&presets) {
        return Ok(presets);
    }
    let mut map: HashMap<String, String> = HashMap::new();
    let mut renumbered = Vec::with_capacity(presets.len());
    for (i, mut p) in presets.into_iter().enumerate() {
        let new_id = (i as u32 + 1).to_string();
        map.insert(p.id.clone(), new_id.clone());
        // case-insensitive lookup helper
        map.insert(p.id.to_ascii_lowercase(), new_id.clone());
        p.id = new_id;
        renumbered.push(p);
    }
    rewrite_sheet(
        client,
        spreadsheet_id,
        PRESETS_SHEET,
        renumbered.iter().map(preset_to_row).collect(),
    )
    .await?;
    remap_users_access_levels(client, spreadsheet_id, &map).await?;
    Ok(renumbered)
}

async fn remap_users_access_levels(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    map: &HashMap<String, String>,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET}'!D2:D1000");
    let rows = match client.read_values(spreadsheet_id, &range).await {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for (i, row) in rows.iter().enumerate() {
        let level = row.first().map(|s| s.trim()).unwrap_or("");
        if level.is_empty() || level.eq_ignore_ascii_case(ADMIN_ROLE_ID) {
            continue;
        }
        let new_id = map
            .get(level)
            .or_else(|| map.get(&level.to_ascii_lowercase()));
        let Some(new_id) = new_id else {
            continue;
        };
        if new_id == level {
            continue;
        }
        let row_index = (i as u32) + 2;
        let cell_range = format!("'{USERS_SHEET}'!D{row_index}");
        client
            .write_values(spreadsheet_id, &cell_range, vec![vec![new_id.clone()]])
            .await?;
    }
    Ok(())
}

/// Лист «Пресеты» + кэш. При тёплом кэше — без Sheets (анти-429).
pub async fn ensure_access_sheets(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
) -> Result<(), String> {
    if crate::core::access::presets_cache_ready() {
        return Ok(());
    }
    client
        .ensure_sheet_exists(spreadsheet_id, PRESETS_SHEET)
        .await?;
    let data = read_data_rows(client, spreadsheet_id, PRESETS_SHEET).await?;
    let has = data.iter().any(|r| !cell(r, 0).is_empty());
    if !has {
        write_header(client, spreadsheet_id, PRESETS_SHEET).await?;
    }
    reload_presets_cache(client, spreadsheet_id).await?;
    Ok(())
}

pub async fn reload_presets_cache(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
) -> Result<Vec<PresetDef>, String> {
    let presets = list_presets(client, spreadsheet_id).await?;
    set_presets_cache(presets.clone());
    Ok(presets)
}

pub async fn list_presets(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
) -> Result<Vec<PresetDef>, String> {
    let data = read_data_rows(client, spreadsheet_id, PRESETS_SHEET).await?;
    let presets: Vec<PresetDef> = data.iter().filter_map(|r| parse_preset_row(r)).collect();
    migrate_presets_to_numeric(client, spreadsheet_id, presets).await
}

pub async fn save_preset(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    preset: PresetDef,
) -> Result<PresetDef, String> {
    let title = preset.title.trim().to_string();
    if title.is_empty() {
        return Err(t("errors.access.presetTitleRequired"));
    }
    let all = list_presets(client, spreadsheet_id).await?;
    let mut id = preset.id.trim().to_string();
    if id.is_empty() {
        id = next_numeric_preset_id(&all);
    } else {
        if !is_numeric_preset_id(&id) {
            return Err(t("errors.access.presetIdInvalid"));
        }
        if id.eq_ignore_ascii_case(ADMIN_ROLE_ID) {
            return Err(t("errors.access.presetAdminIdReserved"));
        }
    }
    // Empty calc_access on save = no calculator. Do not expand to ALL_CALC_FLAGS
    // Пустой CalcAccess при записи = без калькулятора (старые строки листа читаются иначе).
    let calc_access = preset.calc_access;
    let saved = PresetDef {
        id: id.clone(),
        title,
        calculator: calculator_bool_from_flags(&calc_access),
        scripts: preset.scripts,
        calc_access,
        app_access: preset.app_access,
    };
    let mut all = all;
    if let Some(p) = all.iter_mut().find(|p| p.id.eq_ignore_ascii_case(&saved.id)) {
        *p = saved.clone();
    } else {
        all.push(saved.clone());
    }
    rewrite_sheet(
        client,
        spreadsheet_id,
        PRESETS_SHEET,
        all.iter().map(preset_to_row).collect(),
    )
    .await?;
    reload_presets_cache(client, spreadsheet_id).await?;
    Ok(saved)
}

pub async fn delete_preset(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    preset_id: &str,
) -> Result<(), String> {
    let id = preset_id.trim();
    if id.eq_ignore_ascii_case(ADMIN_ROLE_ID) {
        return Err(t("errors.access.cannotDeleteAdmin"));
    }
    let all: Vec<PresetDef> = list_presets(client, spreadsheet_id)
        .await?
        .into_iter()
        .filter(|p| !p.id.eq_ignore_ascii_case(id))
        .collect();
    rewrite_sheet(
        client,
        spreadsheet_id,
        PRESETS_SHEET,
        all.iter().map(preset_to_row).collect(),
    )
    .await?;
    reload_presets_cache(client, spreadsheet_id).await?;
    Ok(())
}

/// Проверить, что пресет есть; вернуть канонический id.
pub async fn resolve_preset_id(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    preset_id: &str,
) -> Result<String, String> {
    let id = preset_id.trim();
    if id.is_empty() {
        return Err(t("errors.access.presetRequired"));
    }
    if id.eq_ignore_ascii_case(ADMIN_ROLE_ID) {
        return Err(t("errors.access.presetCannotBeAdmin"));
    }
    if let Some(p) = crate::core::access::find_preset(id) {
        return Ok(p.id);
    }
    let _ = ensure_access_sheets(client, spreadsheet_id).await;
    if let Some(p) = crate::core::access::find_preset(id) {
        return Ok(p.id);
    }
    let presets = list_presets(client, spreadsheet_id).await?;
    presets
        .into_iter()
        .find(|p| p.id.eq_ignore_ascii_case(id))
        .map(|p| p.id)
        .ok_or_else(|| tf("errors.access.presetNotFound", &[("id", id)]))
}
