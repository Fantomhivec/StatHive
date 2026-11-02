//! Ozon summary row builder.

use crate::calculator::summary_maps::{cell, photo_url};
use crate::calculator::sheet_util::format_ru_num;
use crate::core::i18n::t;
use std::collections::HashMap;

pub(crate) fn build_oz_row(
    sku: &str,
    p: Option<&HashMap<String, String>>,
    st: Option<&HashMap<String, String>>,
    rp: Option<&HashMap<String, String>>,
    at: Option<&HashMap<String, String>>,
    pur: Option<&HashMap<String, String>>,
    price_after_discount_ui: &str,
    discount: &str,
    price_after_promo: &str,
    log_min: f64,
    log_max: f64,
    last_mile: f64,
    kgt_bonus: f64,
    logistics_total: f64,
    orders_cur: &str,
    orders_prev: &str,
    len: &str,
    wid: &str,
    hei: &str,
    wgt: &str,
    vol: &str,
) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(t("calc.headers.oz.sku"), sku.to_string());
    m.insert(
        t("calc.headers.oz.cabinet"),
        first_nonempty(&[
            cell(p, &["Кабинет"]),
            cell(rp, &["Кабинет"]),
            cell(st, &["Кабинет"]),
        ]),
    );
    m.insert(
        t("calc.headers.oz.offerId"),
        first_nonempty(&[
            cell(p, &["Артикул продавца (offer_id)"]),
            cell(rp, &["Артикул"]),
            cell(at, &["Артикул продавца"]),
        ]),
    );
    m.insert(
        t("calc.headers.oz.photo"),
        photo_url(&cell(at, &["Главное изображение"])),
    );
    m.insert(
        t("calc.headers.oz.name"),
        first_nonempty(&[
            cell(rp, &["Название товара"]),
            cell(at, &["Название товара"]),
        ]),
    );
    m.insert(
        t("calc.headers.oz.brand"),
        first_nonempty(&[cell(rp, &["Бренд"]), cell(at, &["Бренд"])]),
    );
    m.insert(
        t("calc.headers.oz.category"),
        first_nonempty(&[cell(rp, &["Тип"]), cell(rp, &["Категория"])]),
    );
    m.insert(
        t("calc.headers.oz.stockFbo"),
        cell(rp, &["Доступно к продаже по схеме FBO, шт."]),
    );
    m.insert(
        t("calc.headers.oz.stockFbs"),
        cell(rp, &["Доступно к продаже по схеме FBS, шт."]),
    );
    m.insert(t("calc.headers.oz.rating"), cell(rp, &["Рейтинг"]));
    m.insert(t("calc.headers.oz.feedbacks"), cell(rp, &["Отзывы"]));
    m.insert(t("calc.headers.oz.orders"), orders_cur.to_string());
    m.insert(t("calc.headers.oz.ordersPrev"), orders_prev.to_string());
    m.insert(
        t("calc.headers.oz.priceAfterDiscount"),
        price_after_discount_ui.to_string(),
    );
    m.insert(t("calc.headers.oz.discount"), discount.to_string());
    m.insert(
        t("calc.headers.oz.priceAfterPromo"),
        price_after_promo.to_string(),
    );
    m.insert(t("calc.headers.oz.commissionFbo"), cell(p, &["Комиссия % FBO"]));
    m.insert(t("calc.headers.oz.commissionFbs"), cell(p, &["Комиссия % FBS"]));
    m.insert(
        t("calc.headers.oz.purchase"),
        first_nonempty(&[cell(pur, &["Закуп"]), cell(p, &["Себестоимость"])]),
    );
    m.insert(
        t("calc.headers.oz.logisticsMin"),
        if log_min > 0.0 {
            format_ru_num(log_min)
        } else {
            cell(p, &["FBS: магистраль от"])
        },
    );
    m.insert(
        t("calc.headers.oz.logisticsMax"),
        if log_max > 0.0 {
            format_ru_num(log_max)
        } else {
            cell(p, &["FBS: магистраль до"])
        },
    );
    m.insert(t("calc.headers.oz.lastMile"), format_ru_num(last_mile));
    m.insert(t("calc.headers.oz.kgt"), format_ru_num(kgt_bonus));
    m.insert(
        t("calc.headers.oz.logistics"),
        if logistics_total > 0.0 {
            format_ru_num(logistics_total)
        } else {
            String::new()
        },
    );
    m.insert(t("calc.headers.oz.length"), len.to_string());
    m.insert(t("calc.headers.oz.width"), wid.to_string());
    m.insert(t("calc.headers.oz.height"), hei.to_string());
    m.insert(t("calc.headers.oz.weight"), wgt.to_string());
    m.insert(t("calc.headers.oz.volume"), vol.to_string());
    m.insert(t("calc.headers.oz.tnved"), cell(at, &["ТН-ВЭД"]));
    m
}

pub(crate) fn first_nonempty(parts: &[String]) -> String {
    for p in parts {
        if !p.trim().is_empty() {
            return p.clone();
        }
    }
    String::new()
}

/// Ozon attributes: depth/width/height обычно mm, weight — g.
pub(crate) fn convert_oz_package_to_cm_kg(
    dimension_unit: &str,
    weight_unit: &str,
    length: &mut f64,
    width: &mut f64,
    height: &mut f64,
    weight: &mut f64,
) {
    let du = dimension_unit.trim().to_ascii_lowercase();
    let already_cm =
        du.contains("cm") || du.contains("см") || du == "centimeter" || du == "centimetre";
    let treat_as_mm = !already_cm
        && (du.is_empty()
            || du.contains("mm")
            || du.contains("мм")
            || du == "millimeter"
            || du == "millimetre");
    if treat_as_mm {
        if *length > 0.0 {
            *length /= 10.0;
        }
        if *width > 0.0 {
            *width /= 10.0;
        }
        if *height > 0.0 {
            *height /= 10.0;
        }
    }

    let wu = weight_unit.trim().to_ascii_lowercase();
    let already_kg = wu.contains("kg") || wu.contains("кг") || wu == "kilogram";
    let treat_as_g = !already_kg
        && (wu.is_empty() || wu == "g" || wu == "г" || wu.contains("gram") || wu.contains("грамм"));
    if treat_as_g && *weight > 0.0 {
        *weight /= 1000.0;
    }
}
