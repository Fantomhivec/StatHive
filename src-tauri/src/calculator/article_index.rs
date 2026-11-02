//! Индекс артикул → площадка + строка свода + сами листы в памяти (до закрытия приложения).
//! Прогрев при входе на калькулятор; lookup без сети при hit.

use crate::calculator::catalog::{self, OZ_SUMMARY_KEY, WB_SUMMARY_KEY};
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::{
    self, cell_at, find_header_index, get_sheet_rows_res, header_row, is_valid_article, SheetCache,
    SheetRows,
};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Copy, Debug)]
struct Loc {
    /// 0 = wb, 1 = ozon (при коллизии в map кладём wb).
    is_ozon: bool,
    /// Индекс строки в `Side.rows` (0 = заголовок; данные ≥ 1).
    row: u32,
    collision: bool,
}

struct Side {
    rows: SheetRows,
    cache_key: String,
}

#[derive(Default)]
struct LookupMem {
    map: HashMap<String, Loc>,
    wb: Option<Side>,
    oz: Option<Side>,
}

static MEM: LazyLock<Mutex<LookupMem>> = LazyLock::new(|| Mutex::new(LookupMem::default()));

fn norm_key(raw: &str) -> String {
    raw.trim().to_lowercase()
}

pub fn count() -> usize {
    MEM.lock().map(|g| g.map.len()).unwrap_or(0)
}

pub fn has_sheets() -> bool {
    MEM.lock()
        .map(|g| g.wb.is_some() || g.oz.is_some())
        .unwrap_or(false)
}

/// Сброс листов (после пересборки свода). Ключи mp можно оставить пустыми — clear полный.
pub fn clear() {
    if let Ok(mut g) = MEM.lock() {
        *g = LookupMem::default();
    }
}

/// Сбросить только листы в RAM (ключи mp оставить). Для force-lookup / после обновления таблиц.
pub fn invalidate_sheets() {
    if let Ok(mut g) = MEM.lock() {
        g.wb = None;
        g.oz = None;
        for loc in g.map.values_mut() {
            loc.row = 0;
        }
    }
}

/// Hit в RAM: площадка, лист, индекс строки, sheet-row (1-based), collision.
/// `row == 0` — только ключ mp без листа (после пересборки свода) → None.
pub fn memory_hit(article: &str) -> Option<(bool, SheetRows, usize, usize, bool)> {
    let k = norm_key(article);
    if !is_valid_article(&k) {
        return None;
    }
    let Ok(g) = MEM.lock() else {
        return None;
    };
    let loc = *g.map.get(&k)?;
    if loc.row == 0 {
        return None;
    }
    let side = if loc.is_ozon {
        g.oz.as_ref()?
    } else {
        g.wb.as_ref()?
    };
    let idx = loc.row as usize;
    if idx >= side.rows.len() {
        return None;
    }
    let sheet_row = idx + 1;
    Some((
        loc.is_ozon,
        std::sync::Arc::clone(&side.rows),
        idx,
        sheet_row,
        loc.collision,
    ))
}

/// Подставить глобальные своды в локальный SheetCache lookup.
pub fn fill_local_cache(cache: &mut SheetCache) {
    let Ok(g) = MEM.lock() else {
        return;
    };
    if let Some(s) = &g.wb {
        cache.insert(s.cache_key.clone(), std::sync::Arc::clone(&s.rows));
    }
    if let Some(s) = &g.oz {
        cache.insert(s.cache_key.clone(), std::sync::Arc::clone(&s.rows));
    }
}

/// `wb` / `ozon` / `None`. Коллизии → `wb` + `collision=true`.
pub fn resolve(article: &str) -> (Option<&'static str>, bool) {
    let k = norm_key(article);
    if !is_valid_article(&k) {
        return (None, false);
    }
    let Ok(g) = MEM.lock() else {
        return (None, false);
    };
    match g.map.get(&k) {
        Some(loc) if loc.collision => (Some("wb"), true),
        Some(loc) if loc.is_ozon => (Some("ozon"), false),
        Some(_) => (Some("wb"), false),
        None => (None, false),
    }
}

/// Прогрев: оба свода параллельно → память (листы + строка на артикул).
pub async fn warm_from_summaries(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    log: Option<&AppLog>,
) {
    if has_sheets() && count() > 0 {
        fill_local_cache(cache);
        return;
    }
    if let Err(e) = rebuild_from_summaries(profile, gs, cache).await {
        if let Some(l) = log {
            run_log::calc_info(l, format!("[Lookup] Не удалось прогреть индекс: {e}"));
        }
    }
}

async fn rebuild_from_summaries(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
) -> Result<usize, String> {
    let (wb_pack, oz_pack) = tokio::join!(
        load_side(profile, gs, WB_SUMMARY_KEY, &["Артикул", "Артикул WB", "Артикул ВБ"]),
        load_side(profile, gs, OZ_SUMMARY_KEY, &["SKU"]),
    );

    let mut next = LookupMem::default();
    if let Some(side) = wb_pack {
        install_side(&mut next, false, side.rows, side.cache_key, &["Артикул", "Артикул WB", "Артикул ВБ"]);
        if let Some(s) = &next.wb {
            cache.insert(s.cache_key.clone(), std::sync::Arc::clone(&s.rows));
        }
    }
    if let Some(side) = oz_pack {
        install_side(&mut next, true, side.rows, side.cache_key, &["SKU"]);
        if let Some(s) = &next.oz {
            cache.insert(s.cache_key.clone(), std::sync::Arc::clone(&s.rows));
        }
    }
    let n = next.map.len();
    if let Ok(mut g) = MEM.lock() {
        *g = next;
    }
    Ok(n)
}

struct LoadedSide {
    rows: SheetRows,
    cache_key: String,
}

async fn load_side(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    table_key: &str,
    _article_headers: &[&str],
) -> Option<LoadedSide> {
    let url = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())?;
    let sid = sheet_util::extract_spreadsheet_id(&url)?;
    let sheet = catalog::find_table(table_key).map(|t| t.sheet_name_tr())?;
    let cache_key = format!("{sid}|{sheet}");
    let mut local = SheetCache::new();
    let rows = get_sheet_rows_res(gs, &mut local, &sid, &sheet).await.ok()?;
    Some(LoadedSide { rows, cache_key })
}

fn install_side(
    mem: &mut LookupMem,
    is_ozon: bool,
    rows: SheetRows,
    cache_key: String,
    article_headers: &[&str],
) {
    if rows.len() < 2 {
        if is_ozon {
            mem.oz = Some(Side { rows, cache_key });
        } else {
            mem.wb = Some(Side { rows, cache_key });
        }
        return;
    }
    let header = header_row(&rows);
    let mut col: isize = -1;
    for h in article_headers {
        col = find_header_index(&header, h, article_headers);
        if col >= 0 {
            break;
        }
    }
    if col < 0 {
        col = 0;
    }
    for (idx, row) in rows.iter().enumerate().skip(1) {
        let v = cell_at(row, col);
        if !is_valid_article(&v) {
            continue;
        }
        let k = norm_key(&v);
        let loc = Loc {
            is_ozon,
            row: idx as u32,
            collision: false,
        };
        match mem.map.get_mut(&k) {
            Some(prev) if prev.is_ozon != is_ozon => {
                // Коллизия: предпочитаем wb (строка WB).
                if is_ozon {
                    prev.collision = true;
                } else {
                    *prev = Loc {
                        is_ozon: false,
                        row: idx as u32,
                        collision: true,
                    };
                }
            }
            Some(_) => {}
            None => {
                mem.map.insert(k, loc);
            }
        }
    }
    if is_ozon {
        mem.oz = Some(Side { rows, cache_key });
    } else {
        mem.wb = Some(Side { rows, cache_key });
    }
}

fn keys_from_map_side(side_ozon: bool) -> Vec<String> {
    let Ok(g) = MEM.lock() else {
        return Vec::new();
    };
    g.map
        .iter()
        .filter(|(_, loc)| {
            if side_ozon {
                loc.is_ozon || loc.collision
            } else {
                !loc.is_ozon || loc.collision
            }
        })
        .map(|(k, _)| k.clone())
        .collect()
}

/// После сборки свода WB: ключи mp без листов (листы сброшены — нужен повторный warm).
pub async fn rebuild_after_wb_keys(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    wb_keys: Vec<String>,
    log: &AppLog,
) {
    let oz = {
        let from_map = keys_from_map_side(true);
        if !from_map.is_empty() {
            from_map
        } else {
            let mut cache = SheetCache::new();
            read_summary_keys_only(profile, gs, &mut cache, OZ_SUMMARY_KEY, &["SKU"]).await
        }
    };
    // Своды на Sheets обновились — RAM-листы устарели.
    clear();
    let mut next = LookupMem::default();
    for a in wb_keys {
        let k = norm_key(&a);
        if !is_valid_article(&k) {
            continue;
        }
        next.map.insert(
            k,
            Loc {
                is_ozon: false,
                row: 0,
                collision: false,
            },
        );
    }
    for sku in oz {
        let k = norm_key(&sku);
        if !is_valid_article(&k) {
            continue;
        }
        match next.map.get_mut(&k) {
            Some(prev) => prev.collision = true,
            None => {
                next.map.insert(
                    k,
                    Loc {
                        is_ozon: true,
                        row: 0,
                        collision: false,
                    },
                );
            }
        }
    }
    let n = next.map.len();
    if let Ok(mut g) = MEM.lock() {
        *g = next;
    }
    run_log::calc_info(
        log,
        format!("[Калькулятор] Индекс артикулов: {n} ключей (листы сброшены, нужен прогрев)."),
    );
}

pub async fn rebuild_after_oz_keys(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    oz_keys: Vec<String>,
    log: &AppLog,
) {
    let wb = {
        let from_map = keys_from_map_side(false);
        if !from_map.is_empty() {
            from_map
        } else {
            let mut cache = SheetCache::new();
            read_summary_keys_only(
                profile,
                gs,
                &mut cache,
                WB_SUMMARY_KEY,
                &["Артикул", "Артикул WB", "Артикул ВБ"],
            )
            .await
        }
    };
    clear();
    let mut next = LookupMem::default();
    for a in wb {
        let k = norm_key(&a);
        if !is_valid_article(&k) {
            continue;
        }
        next.map.insert(
            k,
            Loc {
                is_ozon: false,
                row: 0,
                collision: false,
            },
        );
    }
    for sku in oz_keys {
        let k = norm_key(&sku);
        if !is_valid_article(&k) {
            continue;
        }
        match next.map.get_mut(&k) {
            Some(prev) => prev.collision = true,
            None => {
                next.map.insert(
                    k,
                    Loc {
                        is_ozon: true,
                        row: 0,
                        collision: false,
                    },
                );
            }
        }
    }
    let n = next.map.len();
    if let Ok(mut g) = MEM.lock() {
        *g = next;
    }
    run_log::calc_info(
        log,
        format!("[Калькулятор] Индекс артикулов: {n} ключей (листы сброшены, нужен прогрев)."),
    );
}

async fn read_summary_keys_only(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    table_key: &str,
    article_headers: &[&str],
) -> Vec<String> {
    let Some(loaded) = load_side(profile, gs, table_key, article_headers).await else {
        return Vec::new();
    };
    cache.insert(loaded.cache_key.clone(), std::sync::Arc::clone(&loaded.rows));
    if loaded.rows.len() < 2 {
        return Vec::new();
    }
    let header = header_row(&loaded.rows);
    let mut idx: isize = -1;
    for h in article_headers {
        idx = find_header_index(&header, h, article_headers);
        if idx >= 0 {
            break;
        }
    }
    if idx < 0 {
        idx = 0;
    }
    let mut list = Vec::new();
    for row in loaded.rows.iter().skip(1) {
        let v = cell_at(row, idx);
        if is_valid_article(&v) {
            list.push(v.trim().to_string());
        }
    }
    list
}
