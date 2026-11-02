//! Snapshot / save settings / table links / cabinets.

use crate::calculator::catalog::{self, tables_for_marketplace};
use crate::calculator::profile::{
    self, kgt_normalize, CalculatorProfile, OzKgtRules, WbKgtRules,
};
use crate::core::env_keys;
use crate::core::i18n::t;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

/// Кабинеты из `.env` за одно чтение файла.
struct EnvCabinets {
    wb: Vec<String>,
    ozon: Vec<String>,
    both: Vec<String>,
}

fn load_env_cabinets() -> EnvCabinets {
    let path = env_keys::default_path();
    let Ok(snap) = env_keys::list_rows(&path) else {
        return EnvCabinets {
            wb: Vec::new(),
            ozon: Vec::new(),
            both: Vec::new(),
        };
    };
    let mut wb = Vec::new();
    let mut ozon = Vec::new();
    let mut both = Vec::new();
    for r in snap.rows {
        let name = r.cabinet;
        if name.trim().is_empty() {
            continue;
        }
        both.push(name.clone());
        if r.marketplace.eq_ignore_ascii_case("WB") {
            wb.push(name);
        } else if r.marketplace.eq_ignore_ascii_case("OZON") {
            ozon.push(name);
        }
    }
    sort_dedup_cabinets(&mut wb);
    sort_dedup_cabinets(&mut ozon);
    sort_dedup_cabinets(&mut both);
    EnvCabinets { wb, ozon, both }
}

fn sort_dedup_cabinets(names: &mut Vec<String>) {
    names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
}

pub fn get_snapshot() -> Value {
    build_snapshot(true, "full")
}

/// Снимок уже под view (не полный JSON + retain).
pub fn get_snapshot_for_view(local_keys: bool, view: &str) -> Value {
    build_snapshot(local_keys, view)
}

fn build_snapshot(local_keys: bool, view: &str) -> Value {
    let view = view.trim().to_ascii_lowercase();
    let full = view.is_empty() || view == "full";
    let need_rates = full || view == "view" || view == "settings";
    let need_tables = full || view == "tables";
    let need_kgt = full || view == "kgt";

    let mut profile = profile::load();
    let env = if local_keys {
        load_env_cabinets()
    } else {
        EnvCabinets {
            wb: Vec::new(),
            ozon: Vec::new(),
            both: Vec::new(),
        }
    };
    let mut dirty = false;
    dirty |= ensure_bundle_id(&mut profile);
    dirty |= ensure_table_modes(&mut profile);
    if local_keys {
        dirty |= ensure_cabinet_marketplaces_with(&mut profile, &env);
    }
    if dirty {
        let _ = profile::save(&profile);
    }

    let tables_mp = resolve_tables_marketplace_with(&profile, &env);
    let tables = tables_for_marketplace(&tables_mp);

    let selected = profile.selected_cabinets.clone();
    let available = if local_keys {
        env.both.clone()
    } else {
        selected.clone()
    };
    let available_set: HashSet<String> = available.iter().map(|c| c.to_ascii_lowercase()).collect();
    let missing: Vec<String> = if local_keys {
        selected
            .iter()
            .filter(|c| !available_set.contains(&c.to_ascii_lowercase()))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let mut cabinet_chips =
        build_cabinet_chips(&selected, &available, &profile.cabinet_marketplaces, &env);
    if !local_keys {
        for chip in &mut cabinet_chips {
            if let Some(obj) = chip.as_object_mut() {
                obj.insert("hasKey".into(), json!(true));
            }
        }
    }

    let mut out = json!({
        "bundleId": profile.bundle_id,
        "calculatorName": if profile.calculator_name.trim().is_empty() {
            "калькулятор StatHive".to_string()
        } else {
            profile.calculator_name.clone()
        },
        "marketplace": if profile.marketplace.trim().is_empty() { "wb" } else { &profile.marketplace },
        "tablesMarketplace": tables_mp,
        "selectedCabinets": selected,
        "availableCabinets": available,
        "missingKeyCabinets": missing,
        "cabinetMarketplaces": profile.cabinet_marketplaces,
        "cabinetChips": cabinet_chips,
        "usesLocalKeys": local_keys
    });
    let obj = out.as_object_mut().expect("snapshot object");

    if need_rates {
        obj.insert("triggersEnabled".into(), json!(profile.triggers_enabled));
        obj.insert("taxPercent".into(), json!(profile.tax_percent));
        obj.insert("acquiringPercent".into(), json!(profile.acquiring_percent));
        obj.insert("advPercent".into(), json!(profile.adv_percent));
        obj.insert("extraExpensesPercent".into(), json!(profile.extra_expenses_percent));
        obj.insert("ozonTaxPercent".into(), json!(profile.ozon_tax_percent));
        obj.insert("ozonAcquiringPercent".into(), json!(profile.ozon_acquiring_percent));
        obj.insert("ozonAdvPercent".into(), json!(profile.ozon_adv_percent));
        obj.insert(
            "ozonExtraExpensesPercent".into(),
            json!(profile.ozon_extra_expenses_percent),
        );
        obj.insert("profitFormula".into(), json!(profile.profit_formula));
        obj.insert("ozonProfitFormula".into(), json!(profile.ozon_profit_formula));
    }

    if need_tables {
        let mut table_modes = Map::new();
        for t in &tables {
            let mode = if catalog::is_table_manual(&profile.table_modes, t.key) {
                "manual"
            } else {
                "auto"
            };
            table_modes.insert(t.key.to_string(), json!(mode));
        }
        let metrics: Vec<Value> = catalog::metrics_for_marketplace(
            obj.get("tablesMarketplace")
                .and_then(|v| v.as_str())
                .unwrap_or("wb"),
        )
        .into_iter()
        .map(|m| {
            json!({
                "key": m.key,
                "label": m.label_tr(),
                "marketplace": m.marketplace,
                "tableKey": m.default_table_key
            })
        })
        .collect();
        let table_keys: Vec<Value> = tables
            .iter()
            .map(|t| {
                json!({
                    "key": t.key,
                    "label": t.label_tr(),
                    "marketplace": t.marketplace
                })
            })
            .collect();
        let column_bindings: Vec<Value> = profile
            .column_bindings
            .iter()
            .map(|b| {
                json!({
                    "metricKey": b.metric_key,
                    "spreadsheetUrl": b.spreadsheet_url,
                    "sheetName": b.sheet_name,
                    "columnHeader": b.column_header,
                    "articleColumnHeader": b.article_column_header
                })
            })
            .collect();
        let create_label = if tables.iter().any(|t| {
            !catalog::is_table_manual(&profile.table_modes, t.key)
                && profile
                    .table_links
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(t.key))
                    .map(|(_, u)| u.trim().is_empty())
                    .unwrap_or(true)
        }) {
            t("calc.createTables")
        } else {
            t("calc.refreshAllTables")
        };
        obj.insert("triggersEnabled".into(), json!(profile.triggers_enabled));
        obj.insert("tableLinks".into(), json!(profile.table_links));
        obj.insert("tableModes".into(), json!(table_modes));
        obj.insert("lastUpdated".into(), json!(profile.last_updated));
        obj.insert("columnBindings".into(), json!(column_bindings));
        obj.insert("metrics".into(), json!(metrics));
        obj.insert(
            "triggers".into(),
            json!(crate::calculator::triggers::build_trigger_rows(&profile)),
        );
        obj.insert("tableKeys".into(), json!(table_keys));
        obj.insert(
            "templateIdsByTableKey".into(),
            json!(profile.template_ids_by_table_key),
        );
        obj.insert("createButtonLabel".into(), json!(create_label));
    }

    if need_kgt {
        obj.insert("wbKgt".into(), snapshot_wb_kgt(&profile.wb_kgt));
        obj.insert("ozKgt".into(), snapshot_oz_kgt(&profile.oz_kgt));
    }

    out
}

pub fn resolve_tables_marketplace(profile: &CalculatorProfile) -> String {
    resolve_tables_marketplace_with(profile, &load_env_cabinets())
}

fn resolve_tables_marketplace_with(profile: &CalculatorProfile, env: &EnvCabinets) -> String {
    let selected = &profile.selected_cabinets;
    let has = |mp: &str| {
        let matched = resolve_cabinets_for_marketplace(profile, selected, mp, env);
        !matched.is_empty()
            && !(matched.len() == 1 && matched[0].eq_ignore_ascii_case("__none__"))
    };
    let wb = has("wb");
    let oz = has("ozon");
    if wb && oz {
        "both".into()
    } else if oz {
        "ozon".into()
    } else if wb {
        "wb".into()
    } else {
        // без выбранных кабинетов — none (пустой каталог таблиц).
        "none".into()
    }
}

fn resolve_cabinets_for_marketplace(
    profile: &CalculatorProfile,
    selected: &[String],
    marketplace: &str,
    env: &EnvCabinets,
) -> Vec<String> {
    let want_ozon = marketplace.eq_ignore_ascii_case("ozon");
    let env_list = if want_ozon { &env.ozon } else { &env.wb };
    let env_set: HashSet<String> = env_list.iter().map(|c| c.to_ascii_lowercase()).collect();
    selected
        .iter()
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            if env_set.contains(&lower) {
                return true;
            }
            if let Some(mp) = profile
                .cabinet_marketplaces
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
            {
                let is_oz = mp.to_ascii_lowercase().contains("ozon");
                return is_oz == want_ozon;
            }
            false
        })
        .cloned()
        .collect()
}

fn build_cabinet_chips(
    selected: &[String],
    available: &[String],
    cabinet_mps: &HashMap<String, String>,
    env: &EnvCabinets,
) -> Vec<Value> {
    let available_set: HashSet<String> = available.iter().map(|c| c.to_ascii_lowercase()).collect();
    let wb_env: HashSet<String> = env.wb.iter().map(|c| c.to_ascii_lowercase()).collect();
    let oz_env: HashSet<String> = env.ozon.iter().map(|c| c.to_ascii_lowercase()).collect();
    let mut names: Vec<String> = available
        .iter()
        .chain(selected.iter())
        .cloned()
        .collect();
    sort_dedup_cabinets(&mut names);
    names
        .into_iter()
        .filter(|n| !n.trim().is_empty())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            let mp = cabinet_mps
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&name))
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| {
                    // Ozon раньше WB — кабинет только в OZON_KEY_* не должен стать «wb».
                    if oz_env.contains(&lower) {
                        "ozon".into()
                    } else if wb_env.contains(&lower) {
                        "wb".into()
                    } else {
                        String::new()
                    }
                });
            json!({
                "name": name,
                "marketplace": mp,
                "hasKey": available_set.contains(&lower)
            })
        })
        .collect()
}

/// Возвращает `true`, если профиль изменился.
pub(crate) fn ensure_bundle_id(profile: &mut CalculatorProfile) -> bool {
    if profile.bundle_id.trim().is_empty() {
        profile.bundle_id = uuid::Uuid::new_v4().simple().to_string();
        true
    } else {
        false
    }
}

/// Возвращает `true`, если профиль изменился.
pub(crate) fn ensure_table_modes(profile: &mut CalculatorProfile) -> bool {
    let mut dirty = false;
    for t in catalog::WB_TABLES.iter().chain(catalog::OZON_TABLES.iter()) {
        if profile
            .table_modes
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case(t.key))
        {
            continue;
        }
        let mode = if t.script_key.trim().is_empty() {
            "manual"
        } else {
            "auto"
        };
        profile.table_modes.insert(t.key.to_string(), mode.into());
        dirty = true;
    }
    dirty
}

/// Возвращает `true`, если профиль изменился.
pub(crate) fn ensure_cabinet_marketplaces(profile: &mut CalculatorProfile) -> bool {
    ensure_cabinet_marketplaces_with(profile, &load_env_cabinets())
}

fn ensure_cabinet_marketplaces_with(profile: &mut CalculatorProfile, env: &EnvCabinets) -> bool {
    let wb_env: HashSet<String> = env.wb.iter().map(|c| c.to_ascii_lowercase()).collect();
    let oz_env: HashSet<String> = env.ozon.iter().map(|c| c.to_ascii_lowercase()).collect();
    let mut names: Vec<String> = env.both.clone();
    names.extend(profile.selected_cabinets.iter().cloned());
    sort_dedup_cabinets(&mut names);
    let mut dirty = false;
    for name in names {
        if profile
            .cabinet_marketplaces
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case(&name))
        {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let mp = if oz_env.contains(&lower) {
            "ozon"
        } else if wb_env.contains(&lower) {
            "wb"
        } else {
            "wb"
        };
        profile.cabinet_marketplaces.insert(name, mp.into());
        dirty = true;
    }
    dirty
}

fn snapshot_wb_kgt(r: &WbKgtRules) -> Value {
    json!({
        "maxOneSideCm": r.max_one_side_cm,
        "oneSideCmp": kgt_normalize(&r.one_side_cmp, "gte"),
        "maxSumSidesCm": r.max_sum_sides_cm,
        "sumSidesCmp": kgt_normalize(&r.sum_sides_cmp, "gte"),
        "maxWeightKg": r.max_weight_kg,
        "weightCmp": kgt_normalize(&r.weight_cmp, "gte"),
    })
}

fn snapshot_oz_kgt(r: &OzKgtRules) -> Value {
    json!({
        "maxSumSides": r.max_sum_sides,
        "sumSidesCmp": kgt_normalize(&r.sum_sides_cmp, "gt"),
        "maxWeight": r.max_weight,
        "weightCmp": kgt_normalize(&r.weight_cmp, "gt"),
        "maxVolume": r.max_volume,
        "volumeCmp": kgt_normalize(&r.volume_cmp, "gt"),
        "bonus": r.bonus,
    })
}
