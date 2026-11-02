//! Локализация: каталог `locales/ru/app.json`, `t` / `tf`, локаль в AppData.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const APP_JSON: &str = include_str!("../../../../locales/ru/app.json");
const DEFAULT_LOCALE: &str = "ru";

static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
static LOCALE: OnceLock<Mutex<String>> = OnceLock::new();

fn catalog() -> &'static HashMap<String, String> {
    CATALOG.get_or_init(|| flatten_json(APP_JSON))
}

fn locale_lock() -> &'static Mutex<String> {
    LOCALE.get_or_init(|| Mutex::new(load_saved_locale()))
}

fn locale_path() -> PathBuf {
    crate::core::config::AppConfig::config_dir().join("locale.json")
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct LocaleFile {
    #[serde(default = "default_locale")]
    locale: String,
}

fn default_locale() -> String {
    DEFAULT_LOCALE.into()
}

fn load_saved_locale() -> String {
    let Ok(raw) = fs::read_to_string(locale_path()) else {
        return DEFAULT_LOCALE.into();
    };
    let file: LocaleFile = serde_json::from_str(&raw).unwrap_or_default();
    let loc = file.locale.trim().to_ascii_lowercase();
    if loc.is_empty() {
        DEFAULT_LOCALE.into()
    } else {
        loc
    }
}

fn flatten_json(raw: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return out;
    };
    flatten_value("", &value, &mut out);
    out
}

fn flatten_value(prefix: &str, value: &Value, out: &mut HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let next = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_value(&next, v, out);
            }
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
        }
        Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
        }
        Value::Null | Value::Array(_) => {}
    }
}

/// Текущая локаль (`ru`).
pub fn locale() -> String {
    locale_lock()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| DEFAULT_LOCALE.into())
}

/// Сохранить локаль (пока поддерживается только `ru`).
pub fn set_locale(next: &str) -> Result<(), String> {
    let loc = next.trim().to_ascii_lowercase();
    if loc != "ru" {
        return Err(tf("errors.unknownLocale", &[("locale", next)]));
    }
    let dir = crate::core::config::AppConfig::config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(&LocaleFile {
        locale: loc.clone(),
    })
    .map_err(|e| e.to_string())?;
    fs::write(locale_path(), raw).map_err(|e| e.to_string())?;
    if let Ok(mut g) = locale_lock().lock() {
        *g = loc;
    }
    Ok(())
}

/// Строка по ключу. Нет ключа → сам ключ.
pub fn t(key: &str) -> String {
    let _ = locale();
    catalog()
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// `t` с подстановкой `{name}` → значение.
pub fn tf(key: &str, vars: &[(&str, &str)]) -> String {
    let mut s = t(key);
    for (name, value) in vars {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wb_headers_parity_ru() {
        let expected = [
            "Артикул",
            "Кабинет",
            "Фото",
            "Наименование",
            "Бренд",
            "Артикул ПСТ",
            "Категория",
            "Остаток FBO",
            "Остаток FBS",
            "Клики",
            "Клики (прошл.)",
            "Корзина",
            "Корзина (прошл.)",
            "В желаемое",
            "Заказы",
            "Заказы (прошл.)",
            "Отмены",
            "Отмены (прошл.)",
            "Выкупы",
            "Выкупы (прошл.)",
            "Рейтинг",
            "Отзывы",
            "Цена",
            "Скидка",
            "Цена после скидки",
            "Комиссия FBO",
            "Комиссия FBS",
            "Закуп",
            "Логистика",
            "Есть реклама",
            "Расходы 2 нед.",
            "Стоимость заказа",
            "Длина",
            "Ширина",
            "Высота",
            "Вес",
            "Объём",
            "ТН-ВЭД",
            "Номер сертификата/декларации",
            "Дата регистрации сертификата/декларации",
            "Дата окончания действия сертификата/декларации",
            "Статус проверки",
        ];
        let keys = [
            "calc.headers.wb.article",
            "calc.headers.wb.cabinet",
            "calc.headers.wb.photo",
            "calc.headers.wb.name",
            "calc.headers.wb.brand",
            "calc.headers.wb.vendorPst",
            "calc.headers.wb.category",
            "calc.headers.wb.stockFbo",
            "calc.headers.wb.stockFbs",
            "calc.headers.wb.clicks",
            "calc.headers.wb.clicksPrev",
            "calc.headers.wb.cart",
            "calc.headers.wb.cartPrev",
            "calc.headers.wb.wishlist",
            "calc.headers.wb.orders",
            "calc.headers.wb.ordersPrev",
            "calc.headers.wb.cancels",
            "calc.headers.wb.cancelsPrev",
            "calc.headers.wb.sales",
            "calc.headers.wb.salesPrev",
            "calc.headers.wb.rating",
            "calc.headers.wb.feedbacks",
            "calc.headers.wb.price",
            "calc.headers.wb.discount",
            "calc.headers.wb.priceAfterDiscount",
            "calc.headers.wb.commissionFbo",
            "calc.headers.wb.commissionFbs",
            "calc.headers.wb.purchase",
            "calc.headers.wb.logistics",
            "calc.headers.wb.hasAdvertising",
            "calc.headers.wb.adSpend2w",
            "calc.headers.wb.orderCost",
            "calc.headers.wb.length",
            "calc.headers.wb.width",
            "calc.headers.wb.height",
            "calc.headers.wb.weight",
            "calc.headers.wb.volume",
            "calc.headers.wb.tnved",
            "calc.headers.wb.certNumber",
            "calc.headers.wb.certRegDate",
            "calc.headers.wb.certEndDate",
            "calc.headers.wb.certStatus",
        ];
        assert_eq!(keys.len(), expected.len());
        for (key, exp) in keys.iter().zip(expected.iter()) {
            assert_eq!(t(key), *exp, "key {key}");
        }
    }

    #[test]
    fn wb_stocks_fields_parity() {
        assert_eq!(t("fields.wb_stocks.cabinet"), "Кабинет");
        assert_eq!(t("fields.wb_stocks.nmId"), "Артикул WB");
        assert_eq!(t("fields.wb_stocks.quantity"), "Остаток");
    }

    #[test]
    fn tf_placeholder() {
        let s = tf("templates.newTemplateName", &[("scriptTitle", "Тест")]);
        assert_eq!(s, "Новый — Тест");
    }
}
