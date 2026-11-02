//! `cards_update_dimensions` — правка габаритов / vendor / НДС / бренда.

mod payload;

use crate::core::rate_profiles::{cards_list, cards_update_dimensions as rate};
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, sheet_ranges};
use self::payload::{build_item, needs_update, Desired};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const CARDS_URL: &str = "https://content-api.wildberries.ru/content/v2/get/cards/list";
const UPDATE_URL: &str = "https://content-api.wildberries.ru/content/v2/cards/update";
const PAGE_LIMIT: i64 = 100;
const CHUNK: usize = 500;

pub struct CardsUpdateDimensionsScript;

impl Script for CardsUpdateDimensionsScript {
    fn key(&self) -> &'static str {
        "cards_update_dimensions"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.cards_update_dimensions")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        Vec::new()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

struct InputRow {
    cabinet: String,
    nm_id: i64,
    length: i32,
    width: i32,
    height: i32,
    vendor_code: String,
    vat: String,
    brand: String,
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log
        .info("Скрипт «Редактирование карточек»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let dims = flag(ctx, "cards_update_dimensions_mode", true);
    let vendor = flag(ctx, "cards_update_vendor_code_mode", false);
    let vat = flag(ctx, "cards_update_vat_mode", false);
    let brand = flag(ctx, "cards_update_brand_mode", false);
    if !dims && !vendor && !vat && !brand {
        return Err(
            "Выберите хотя бы один режим: габариты, Артикул ПСТ, НДС или бренд.".into(),
        );
    }
    let cab_range = param(ctx, "cabinets_source_range");
    let nm_range = param(ctx, "nm_id_source_range");
    if cab_range.is_empty() || nm_range.is_empty() {
        return Err("Заполните диапазоны: кабинеты и nmID.".into());
    }
    if dims {
        for k in [
            "cards_update_length_range",
            "cards_update_width_range",
            "cards_update_height_range",
        ] {
            if param(ctx, k).is_empty() {
                return Err(
                    "Для режима габаритов заполните диапазоны: длина, ширина, высота.".into(),
                );
            }
        }
    }
    if vendor && param(ctx, "cards_update_vendor_code_range").is_empty() {
        return Err("Для режима Артикула ПСТ заполните диапазон.".into());
    }
    if vat && param(ctx, "cards_update_vat_range").is_empty() {
        return Err("Для режима НДС заполните диапазон ставки НДС.".into());
    }
    if brand && param(ctx, "cards_update_brand_range").is_empty() {
        return Err("Для режима бренда заполните диапазон брендов.".into());
    }

    let client = common::google_sheets_client(ctx).await?;
    let mut range_keys = vec![cab_range.clone(), nm_range.clone()];
    if dims {
        range_keys.push(param(ctx, "cards_update_length_range"));
        range_keys.push(param(ctx, "cards_update_width_range"));
        range_keys.push(param(ctx, "cards_update_height_range"));
    }
    if vendor {
        range_keys.push(param(ctx, "cards_update_vendor_code_range"));
    }
    if vat {
        range_keys.push(param(ctx, "cards_update_vat_range"));
    }
    if brand {
        range_keys.push(param(ctx, "cards_update_brand_range"));
    }
    let refs: Vec<&str> = range_keys.iter().map(|s| s.as_str()).collect();
    let cols = sheet_ranges::read_first_cols(&client, &spreadsheet_id, &sheet_name, &refs).await?;

    let mut idx = 0usize;
    let cabs = take_col(&cols, &mut idx);
    let nms = take_col(&cols, &mut idx);
    let ls = if dims {
        take_col(&cols, &mut idx)
    } else {
        Vec::new()
    };
    let ws = if dims {
        take_col(&cols, &mut idx)
    } else {
        Vec::new()
    };
    let hs = if dims {
        take_col(&cols, &mut idx)
    } else {
        Vec::new()
    };
    let vcs = if vendor {
        take_col(&cols, &mut idx)
    } else {
        Vec::new()
    };
    let vats = if vat {
        take_col(&cols, &mut idx)
    } else {
        Vec::new()
    };
    let brands = if brand {
        take_col(&cols, &mut idx)
    } else {
        Vec::new()
    };

    let max = [
        cabs.len(),
        nms.len(),
        ls.len(),
        ws.len(),
        hs.len(),
        vcs.len(),
        vats.len(),
        brands.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    if max == 0 {
        return Err("В указанных диапазонах нет данных".into());
    }

    let mut rows = Vec::new();
    let mut skipped = 0usize;
    for i in 0..max {
        let cabinet = at(&cabs, i).trim().to_string();
        let nm_raw = at(&nms, i);
        let empty = cabinet.is_empty()
            && nm_raw.trim().is_empty()
            && (!dims
                || (at(&ls, i).trim().is_empty()
                    && at(&ws, i).trim().is_empty()
                    && at(&hs, i).trim().is_empty()))
            && (!vendor || at(&vcs, i).trim().is_empty())
            && (!vat || at(&vats, i).trim().is_empty())
            && (!brand || i >= brands.len());
        if empty {
            skipped += 1;
            continue;
        }
        if cabinet.is_empty() {
            return Err(format!("Пустой кабинет в строке {}", i + 1).into());
        }
        let Some(nm_id) = sheet_ranges::parse_i64_cell(nm_raw).filter(|n| *n > 0) else {
            return Err(format!("Некорректный nmID '{nm_raw}' в строке {}", i + 1).into());
        };
        let mut row = InputRow {
            cabinet,
            nm_id,
            length: 0,
            width: 0,
            height: 0,
            vendor_code: String::new(),
            vat: String::new(),
            brand: String::new(),
        };
        if dims {
            row.length = parse_dim(at(&ls, i), "длина", i + 1)?;
            row.width = parse_dim(at(&ws, i), "ширина", i + 1)?;
            row.height = parse_dim(at(&hs, i), "высота", i + 1)?;
        }
        if vendor {
            let vc = at(&vcs, i).trim().to_string();
            if vc.is_empty() {
                return Err(format!("Пустой Артикул ПСТ в строке {}", i + 1).into());
            }
            row.vendor_code = vc;
        }
        if vat {
            let v = at(&vats, i).trim().to_string();
            if v.is_empty() {
                return Err(format!("Пустая ставка НДС в строке {}", i + 1).into());
            }
            row.vat = v;
        }
        if brand {
            row.brand = at(&brands, i).trim().to_string();
        }
        rows.push(row);
    }
    ctx.log
        .info(format!("Прочитано строк: {} (пустых: {skipped})", rows.len()));
    if rows.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет данных".into(),
        });
    }

    let mut by_cab: HashMap<String, Vec<&InputRow>> = HashMap::new();
    for r in &rows {
        by_cab.entry(r.cabinet.clone()).or_default().push(r);
    }
    let tokens = common::load_wb_tokens()?;
    let mut updated = 0usize;

    for (cabinet, group) in &by_cab {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let token = common::resolve_wb_token_with_map(ctx, &tokens, cabinet);
        if token.trim().is_empty() {
            return Err(format!("API ключ для кабинета '{cabinet}' не найден").into());
        }
        let cards = fetch_cards(ctx, cabinet, token.trim()).await?;
        ctx.log
            .info(format!("Кабинет {cabinet}: получено карточек {}", cards.len()));
        let desired: HashMap<i64, &InputRow> = group
            .iter()
            .map(|r| (r.nm_id, *r))
            .collect();
        let nm_set: HashSet<i64> = desired.keys().copied().collect();
        let mut payloads = Vec::new();
        for card in &cards {
            let nm = card
                .get("nmID")
                .or_else(|| card.get("nmId"))
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            if !nm_set.contains(&nm) {
                continue;
            }
            let Some(row) = desired.get(&nm) else {
                continue;
            };
            let d = Desired {
                length: row.length,
                width: row.width,
                height: row.height,
                vendor_code: row.vendor_code.clone(),
                vat: row.vat.clone(),
                brand: row.brand.clone(),
                dims,
                vendor,
                vat_mode: vat,
                brand_mode: brand,
            };
            if !needs_update(card, &d) {
                continue;
            }
            payloads.push(build_item(card, &d));
        }
        if payloads.is_empty() {
            ctx.log
                .info(format!("Кабинет {cabinet}: нет изменений — пропуск"));
            continue;
        }
        ctx.log.info(format!(
            "Кабинет {cabinet}: карточек к обновлению: {}",
            payloads.len()
        ));
        let profile = rate::profile();
        for (ci, chunk) in payloads.chunks(CHUNK).enumerate() {
            ctx.check_cancelled().map_err(|e| e.to_string())?;
            let body = Value::Array(chunk.to_vec());
            match ctx
                .request
                .post_json(
                    Limit {
                        cabinet,
                        scope: rate::SCOPE,
                        profile: &profile,
                    },
                    UPDATE_URL,
                    Auth::Bearer(token.trim()),
                    &body,
                )
                .await
            {
                Ok(v) => {
                    if v.get("error").and_then(|e| e.as_bool()).unwrap_or(false) {
                        let t = v
                            .get("errorText")
                            .and_then(|x| x.as_str())
                            .unwrap_or("error");
                        ctx.log
                            .warn(format!("Кабинет {cabinet}: chunk {}: {t}", ci + 1));
                    } else {
                        updated += chunk.len();
                        ctx.log.info(format!(
                            "Кабинет {cabinet}: chunk {} ok ({})",
                            ci + 1,
                            chunk.len()
                        ));
                    }
                }
                Err(e) => ctx
                    .log
                    .warn(format!("Кабинет {cabinet}: chunk {}: {e}", ci + 1)),
            }
        }
    }

    Ok(ScriptOutput {
        rows_written: updated,
        message: format!("Обновлено карточек: {updated}"),
    })
}

async fn fetch_cards(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut cursor_updated_at: Option<Value> = None;
    let mut cursor_nm_id: Option<i64> = None;
    let profile = cards_list::profile();
    loop {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let mut cursor = serde_json::json!({ "limit": PAGE_LIMIT });
        if let Some(ref ua) = cursor_updated_at {
            cursor["updatedAt"] = ua.clone();
        }
        if let Some(nm) = cursor_nm_id {
            cursor["nmID"] = Value::from(nm);
        }
        let body = serde_json::json!({
            "settings": {
                "cursor": cursor,
                "filter": { "withPhoto": -1 }
            }
        });
        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: cards_list::SCOPE,
                    profile: &profile,
                },
                CARDS_URL,
                Auth::Bearer(token),
                &body,
            )
            .await?;
        let batch = v
            .get("cards")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let batch_len = batch.len();
        all.extend(batch);
        let resp_cursor = v.get("cursor");
        let total = resp_cursor
            .and_then(|c| c.get("total"))
            .and_then(|t| t.as_i64())
            .unwrap_or(batch_len as i64);
        cursor_updated_at = resp_cursor.and_then(|c| c.get("updatedAt")).cloned();
        cursor_nm_id = resp_cursor
            .and_then(|c| c.get("nmID"))
            .and_then(|n| n.as_i64());
        if batch_len == 0 || batch_len < PAGE_LIMIT as usize || total < PAGE_LIMIT {
            break;
        }
        if cursor_updated_at.is_none() && cursor_nm_id.is_none() {
            break;
        }
    }
    Ok(all)
}

fn take_col(cols: &[Vec<String>], idx: &mut usize) -> Vec<String> {
    let v = cols.get(*idx).cloned().unwrap_or_default();
    *idx += 1;
    v
}

fn at(col: &[String], i: usize) -> &str {
    col.get(i).map(|s| s.as_str()).unwrap_or("")
}

fn parse_dim(raw: &str, label: &str, row: usize) -> Result<i32, String> {
    let s = raw.trim().replace(',', ".");
    let n = s
        .parse::<f64>()
        .map_err(|_| format!("Некорректная {label} '{raw}' в строке {row}"))?;
    if n < 0.0 {
        return Err(format!("Некорректная {label} '{raw}' в строке {row}"));
    }
    Ok(n as i32)
}

fn flag(ctx: &ScriptContext, key: &str, default: bool) -> bool {
    ctx.params
        .get(key)
        .map(|s| match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            _ => default,
        })
        .unwrap_or(default)
}

fn param(ctx: &ScriptContext, key: &str) -> String {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
