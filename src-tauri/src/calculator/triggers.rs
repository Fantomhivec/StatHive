//! Триггеры авто-таблиц калькулятора → TimeTriggers шаблонов (`setTriggers`).

use crate::calculator::catalog::{self, tables_for_marketplace};
use crate::calculator::profile::{self, CalculatorProfile, CalculatorTriggerSettings};
use crate::calculator::snapshot::{self, resolve_tables_marketplace};
use crate::core::sheet_dates;
use crate::templates::store as templates_store;
use crate::templates::trigger::{TimeTrigger, TriggerType};
use serde_json::{json, Value};
use uuid::Uuid;

fn display_name_for_key(table_key: &str) -> String {
    catalog::find_table(table_key)
        .map(|t| t.label_tr())
        .unwrap_or_else(|| table_key.to_string())
}

pub fn build_trigger_rows(profile: &CalculatorProfile) -> Vec<Value> {
    let tables_mp = resolve_tables_marketplace(profile);
    let by_key: std::collections::HashMap<&str, &CalculatorTriggerSettings> = profile
        .triggers
        .iter()
        .map(|t| (t.table_key.as_str(), t))
        .collect();
    tables_for_marketplace(&tables_mp)
        .into_iter()
        .filter(|t| !catalog::is_table_manual(&profile.table_modes, t.key))
        .map(|t| {
            let existing = by_key
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(t.key))
                .map(|(_, v)| *v);
            json!({
                "tableKey": t.key,
                "displayName": t.label_tr(),
                "mode": existing.map(|e| e.mode.as_str()).unwrap_or("None"),
                "time": existing.map(|e| e.time.as_str()).unwrap_or("09:00"),
                "intervalHours": existing.map(|e| e.interval_hours).unwrap_or(1),
                "intervalMinutes": existing.map(|e| e.interval_minutes).unwrap_or(0),
                "nextExecution": existing.map(|e| e.next_execution.as_str()).unwrap_or(""),
                "scheduledDateTime": existing.map(|e| e.scheduled_date_time.as_str()).unwrap_or(""),
                "weeklyDay": existing.map(|e| e.weekly_day.as_str()).unwrap_or("Monday"),
                "weeklyTime": existing.map(|e| e.weekly_time.as_str()).unwrap_or("09:00"),
            })
        })
        .collect()
}

fn json_str<'a>(t: &'a Value, key: &str) -> Option<&'a str> {
    t.get(key).and_then(|x| x.as_str())
}

fn json_i32(t: &Value, key: &str, default: i32) -> i32 {
    t.get(key)
        .and_then(|x| x.as_i64())
        .map(|n| n as i32)
        .unwrap_or(default)
}

pub fn set_triggers_from_json(triggers: &Value) -> Result<Value, String> {
    let mut profile = profile::load();
    let mut list = Vec::new();
    if let Some(arr) = triggers.as_array() {
        for t in arr {
            let table_key = json_str(t, "tableKey").unwrap_or("").to_string();
            let mode = json_str(t, "mode").unwrap_or("None").to_string();
            let interval_minutes = json_i32(t, "intervalMinutes", 0);
            let interval_hours = json_i32(t, "intervalHours", 0);
            // Prefer total minutes from editor; keep hours for legacy UI.
            let (ih, im) = if interval_hours == 0 && interval_minutes >= 60 {
                (interval_minutes / 60, interval_minutes % 60)
            } else {
                (interval_hours, interval_minutes)
            };
            list.push(CalculatorTriggerSettings {
                display_name: display_name_for_key(&table_key),
                table_key,
                mode,
                time: json_str(t, "time").unwrap_or("09:00").to_string(),
                interval_hours: ih,
                interval_minutes: im,
                next_execution: String::new(),
                scheduled_date_time: json_str(t, "scheduledDateTime").unwrap_or("").to_string(),
                weekly_day: json_str(t, "weeklyDay").unwrap_or("Monday").to_string(),
                weekly_time: json_str(t, "weeklyTime").unwrap_or("09:00").to_string(),
            });
        }
    }
    profile.triggers = list;
    profile::save(&profile)?;
    apply_triggers_to_templates(&mut profile)?;
    profile::save(&profile)?;
    Ok(snapshot::get_snapshot())
}

pub fn sync_template_triggers_enabled(profile: &CalculatorProfile) -> Result<(), String> {
    let ids: Vec<String> = profile
        .template_ids_by_table_key
        .values()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect();
    for tid in ids {
        let Some(mut tpl) = templates_store::get(&tid)? else {
            continue;
        };
        let mut changed = false;
        for tr in &mut tpl.time_triggers {
            if tr.is_enabled != profile.triggers_enabled {
                tr.is_enabled = profile.triggers_enabled;
                changed = true;
            }
        }
        if changed {
            templates_store::upsert(tpl)?;
        }
    }
    Ok(())
}

fn parse_mode(mode: &str) -> Option<TriggerType> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "once" => Some(TriggerType::Once),
        "daily" => Some(TriggerType::Daily),
        "weekly" => Some(TriggerType::Weekly),
        "interval" => Some(TriggerType::Interval),
        _ => None,
    }
}

fn ensure_hh_mm_ss(time: &str) -> String {
    let t = time.trim();
    if t.is_empty() {
        return "09:00:00".into();
    }
    if t.len() == 5 && t.chars().nth(2) == Some(':') {
        return format!("{t}:00");
    }
    t.to_string()
}

fn apply_triggers_to_templates(profile: &mut CalculatorProfile) -> Result<(), String> {
    for trigger in profile.triggers.iter_mut() {
        if trigger.table_key.trim().is_empty() {
            continue;
        }
        trigger.display_name = display_name_for_key(&trigger.table_key);
        let tid = profile
            .template_ids_by_table_key
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&trigger.table_key))
            .map(|(_, id)| id.clone());
        let Some(tid) = tid.filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let Some(mut tpl) = templates_store::get(&tid)? else {
            continue;
        };
        tpl.time_triggers.clear();
        if let Some(ttype) = parse_mode(&trigger.mode) {
            let mut tt = TimeTrigger {
                id: Uuid::new_v4().to_string(),
                template_id: tpl.id.clone(),
                is_enabled: profile.triggers_enabled,
                trigger_type: ttype,
                ..TimeTrigger::default()
            };
            match tt.trigger_type {
                TriggerType::Once => {
                    let raw = trigger.scheduled_date_time.trim();
                    tt.scheduled_date_time = if raw.is_empty() {
                        None
                    } else {
                        Some(raw.to_string())
                    };
                }
                TriggerType::Daily => {
                    tt.daily_time = Some(ensure_hh_mm_ss(&trigger.time));
                }
                TriggerType::Weekly => {
                    tt.weekly_day = Some(
                        if trigger.weekly_day.trim().is_empty() {
                            "Monday".into()
                        } else {
                            trigger.weekly_day.clone()
                        },
                    );
                    let wt = if trigger.weekly_time.trim().is_empty() {
                        &trigger.time
                    } else {
                        &trigger.weekly_time
                    };
                    tt.weekly_time = Some(ensure_hh_mm_ss(wt));
                }
                TriggerType::Interval => {
                    let mins = (trigger.interval_hours.max(0) as i64) * 60
                        + trigger.interval_minutes.max(0) as i64;
                    tt.interval_minutes = Some(mins.max(1));
                }
            }
            tt.refresh_next();
            trigger.next_execution = tt
                .next_execution_at
                .as_deref()
                .map(sheet_dates::format_wb_timestamp)
                .unwrap_or_default();
            tpl.time_triggers.push(tt);
        } else {
            trigger.next_execution.clear();
        }
        templates_store::upsert(tpl)?;
    }
    Ok(())
}
