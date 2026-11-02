//! Чтение листов и поиск колонок для CardReader / Excel.

use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::spreadsheet_id;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Строки листа без clone на каждый lookup (Arc).
pub type SheetRows = Arc<Vec<Vec<String>>>;
pub type SheetCache = HashMap<String, SheetRows>;

pub const ARTICLE_ALIASES: &[&str] = &[
    "Артикул",
    "Артикул ВБ",
    "Артикул WB",
    "Артикул WB (nmID)",
    "Артикул WB (nmId)",
    "nmID",
    "nmId",
    "NmID",
    "SKU",
    "SKU (ID товара OZON)",
];

/// Ключ nmId при сборке свода WB (паритет `CalculatorSummaryBuilder` artHeaders).
/// Без голого «Артикул» первым: иначе при наличии колонки продавца закуп/воронка
/// индексируются не по nmId и в карточку не попадают.
pub const WB_SOURCE_ARTICLE_HEADERS: &[&str] = &[
    "Артикул WB",
    "Артикул ВБ",
    "nmID",
    "nmId",
    "Артикул WB (nmID)",
    "Артикул WB (nmId)",
];

pub const SUBJECT_ALIASES: &[&str] = &["ID предмета", "subjectID", "subjectId", "subject_id"];

pub fn resolve_value_aliases(column_header: &str) -> Vec<&'static str> {
    let groups: &[&[&str]] = &[
        &["photo1", "Фото", "photos", "Фото (до 30 ссылок в конце)"],
        &["Кабинет", "cabinet"],
        &["Наименование", "Название", "Название товара", "title"],
        &["Бренд", "brandName", "brand"],
        &["Артикул продавца", "vendorCode", "vendor_code"],
        &["Предмет", "subjectName", "Название предмета", "Категория"],
        &["ID предмета", "subjectID", "subjectId", "subject_id"],
        &["Длина, см", "dimensions.length", "length", "Длина"],
        &["Ширина, см", "dimensions.width", "width", "Ширина"],
        &["Высота, см", "dimensions.height", "height", "Высота"],
        &["Остатки WB", "Остаток FBO", "stocks.wb"],
        &["Остатки продавца", "Остаток FBS", "stocks.mp"],
        &["Остаток", "Остаток FBS", "quantity"],
        &["Переходы", "Клики", "selected.openCount"],
        &["Переходы (прошл.)", "Клики (прошл.)", "past.openCount"],
        &["В корзину", "Корзина", "selected.cartCount"],
        &["В корзину (прошл.)", "Корзина (прошл.)", "past.cartCount"],
        &["В отложенное", "В желаемое", "selected.addToWishlist"],
        &["Заказы", "Заказы 2 недели", "selected.orderCount"],
        &["Заказы (прошл.)", "past.orderCount"],
        &["Отмены", "Отмены, шт", "selected.cancelCount"],
        &["Отмены (прошл.)", "past.cancelCount"],
        &["Выкупы", "selected.buyoutCount"],
        &["Выкупы (прошл.)", "past.buyoutCount"],
        &["Цена", "price"],
        &["Скидка, %", "Скидка", "discount"],
        &[
            "Цена после скидки",
            "Цена со скидкой (WB)",
            "Цена со скидкой",
            "discountedPrice",
        ],
        &[
            "Рейтинг",
            "Рейтинг по отзывам (текущий)",
            "feedback_rating_current",
            "Оценка пользователей",
            "feedbackRating",
        ],
        &[
            "Отзывы",
            "Все отзывы за период (текущий)",
            "feedback_count_current",
            "feedbacks",
        ],
        &["Есть реклама", "hasAds"],
        &["Расходы 2 нед.", "Расходы 2 недели", "adExpenses"],
        &["Стоимость заказа", "orderCost"],
        &["Логистика", "logistics"],
        &["Артикул ПСТ", "Артикул продавца", "vendorCode"],
        &["Склад WB (FBW) %", "Комиссия FBO", "paidStorageKgvp", "kgvpSupplier"],
        &["Маркетплейс (FBS) %", "Комиссия FBS", "kgvpMarketplace"],
        &[
            "Логистика FBS 1л, ₽",
            "Логистика 1л, ₽",
            "boxDeliveryMarketplaceBase",
            "boxDeliveryBase",
        ],
        &[
            "Логистика FBS доп. л, ₽",
            "Логистика доп. л, ₽",
            "boxDeliveryMarketplaceLiter",
            "boxDeliveryLiter",
        ],
        &["Вес, кг", "dimensions.weightBrutto", "weightBrutto", "Вес"],
    ];
    for g in groups {
        if g.iter()
            .any(|a| a.eq_ignore_ascii_case(column_header.trim()))
        {
            return g.to_vec();
        }
    }
    Vec::new()
}

pub fn extract_spreadsheet_id(url_or_id: &str) -> Option<String> {
    spreadsheet_id::try_extract(url_or_id)
}

pub async fn get_sheet_rows(
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    sid: &str,
    sheet_name: &str,
) -> Option<SheetRows> {
    match get_sheet_rows_res(gs, cache, sid, sheet_name).await {
        Ok(rows) => Some(rows),
        Err(_) => None,
    }
}

/// Как `get_sheet_rows`, но с текстом ошибки Sheets (не маскировать Err как «пустой лист»).
pub async fn get_sheet_rows_res(
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    sid: &str,
    sheet_name: &str,
) -> Result<SheetRows, String> {
    let key = format!("{sid}|{sheet_name}");
    if let Some(rows) = cache.get(&key) {
        return Ok(Arc::clone(rows));
    }
    let range = format!("'{sheet_name}'");
    let rows = gs
        .read_values(sid, &range)
        .await
        .map_err(|e| e.to_string())?;
    let arc = Arc::new(rows);
    cache.insert(key, Arc::clone(&arc));
    Ok(arc)
}

pub fn cell_at(row: &[String], idx: isize) -> String {
    if idx < 0 {
        return String::new();
    }
    row.get(idx as usize)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub fn find_header_index(header: &[String], name: &str, aliases: &[&str]) -> isize {
    let mut candidates: Vec<String> = Vec::new();
    if !name.trim().is_empty() {
        candidates.push(name.trim().to_string());
    }
    for a in aliases {
        let t = a.trim();
        if t.is_empty() {
            continue;
        }
        if !candidates
            .iter()
            .any(|c| c.eq_ignore_ascii_case(t))
        {
            candidates.push(t.to_string());
        }
    }
    for candidate in &candidates {
        for (i, h) in header.iter().enumerate() {
            if h.eq_ignore_ascii_case(candidate) {
                return i as isize;
            }
        }
    }
    for candidate in &candidates {
        if candidate.len() < 8 {
            continue;
        }
        for (i, h) in header.iter().enumerate() {
            if h.is_empty() {
                continue;
            }
            if h.to_ascii_lowercase().starts_with(&candidate.to_ascii_lowercase())
                || candidate
                    .to_ascii_lowercase()
                    .starts_with(&h.to_ascii_lowercase())
            {
                return i as isize;
            }
        }
    }
    -1
}

pub fn resolve_column_index(header: &[String], spec: &str, aliases: &[&str]) -> isize {
    if let Some(idx) = try_parse_a1_column(spec) {
        return idx;
    }
    find_header_index(header, spec, aliases)
}

fn try_parse_a1_column(spec: &str) -> Option<isize> {
    let s = spec.trim().trim_start_matches('=').replace('$', "");
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r"^(?:(?P<col>[A-Za-z])|(?P<col2>[A-Za-z]{1,3})\d+|(?P<col3>[A-Za-z]{1,3})\d*:(?:[A-Za-z]{1,3})\d*)$",
        )
        .expect("a1")
    });
    let caps = re.captures(&s)?;
    let letters = caps
        .name("col")
        .or_else(|| caps.name("col2"))
        .or_else(|| caps.name("col3"))?
        .as_str()
        .to_ascii_uppercase();
    if letters.is_empty() || !letters.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut index: isize = 0;
    for ch in letters.chars() {
        index = index * 26 + (ch as isize - 'A' as isize + 1);
    }
    Some(index - 1)
}

pub fn articles_match(cell: &str, article: &str) -> bool {
    if cell.eq_ignore_ascii_case(article) {
        return true;
    }
    if let (Some(a), Some(b)) = (try_parse_number(cell), try_parse_number(article)) {
        return (a - b).abs() < 0.5;
    }
    false
}

pub fn try_parse_number(raw: &str) -> Option<f64> {
    let s = raw
        .trim()
        .replace('%', "")
        .replace(' ', "")
        .replace(',', ".");
    if s.is_empty() {
        return None;
    }
    s.parse::<f64>().ok()
}

pub fn format_ru_num(n: f64) -> String {
    let s = format!("{n:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    s.replace('.', ",")
}

pub fn first_http_url(raw: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)https?://[^\s,;|]+").expect("url"));
    if let Some(m) = re.find(raw) {
        m.as_str()
            .trim_end_matches([')', ']', '"', '\''])
            .to_string()
    } else {
        raw.trim().to_string()
    }
}

/// nmId / SKU «0» и пустые — мусор (паритет `CalculatorArticleIndex.IsValidArticleOrSku`).
pub fn is_valid_article(article: &str) -> bool {
    let a = article.trim();
    if a.is_empty() {
        return false;
    }
    !is_zero_numeric_id(a)
}

/// Числовой nmId/SKU == 0 (`0`, `0.0`, `0,0`…). Нечисловые ключи (offer_id) — false.
pub fn is_zero_numeric_id(raw: &str) -> bool {
    let k = raw
        .trim()
        .replace('\u{00a0}', " ")
        .replace(' ', "")
        .replace(',', ".");
    if k.is_empty() {
        return false;
    }
    k.parse::<f64>().ok().is_some_and(|n| n == 0.0)
}

pub fn header_row(rows: &[Vec<String>]) -> Vec<String> {
    rows.first()
        .map(|r| r.iter().map(|c| c.trim().to_string()).collect())
        .unwrap_or_default()
}
