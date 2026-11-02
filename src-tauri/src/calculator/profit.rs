//! Оценка прибыли по формуле (паритет EvaluateProfit).

use crate::calculator::profile::CalculatorProfile;
use crate::calculator::sheet_util::try_parse_number;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn evaluate_profit(
    formula: &str,
    values: &HashMap<String, String>,
    profile: &CalculatorProfile,
    use_ozon_rates: bool,
) -> Option<f64> {
    let default =
        "Цена после скидки*(1-Комиссия FBS-Налог-Эквайринг-Реклама-Доп. расходы)-Закуп-Логистика";
    let mut expr = if formula.trim().is_empty() {
        default.to_string()
    } else {
        formula.trim().to_string()
    };
    expr = normalize_profit_formula(&expr);

    let tax = if use_ozon_rates {
        profile.ozon_tax_percent
    } else {
        profile.tax_percent
    };
    let acquiring = if use_ozon_rates {
        profile.ozon_acquiring_percent
    } else {
        profile.acquiring_percent
    };
    let adv = if use_ozon_rates {
        profile.ozon_adv_percent
    } else {
        profile.adv_percent
    };
    let extra = if use_ozon_rates {
        profile.ozon_extra_expenses_percent
    } else {
        profile.extra_expenses_percent
    };

    expr = replace_token(&expr, "налог", tax / 100.0);
    expr = replace_token(&expr, "эквайринг", acquiring / 100.0);
    expr = replace_token(&expr, "реклама", adv / 100.0);
    expr = replace_token(&expr, "доп_расходы", extra / 100.0);
    expr = replace_token(
        &expr,
        "цена_после_скидки",
        parse_metric(values, &["priceAfterDiscount", "price", "ozPrice"]),
    );
    expr = replace_token(
        &expr,
        "комиссия_fbo",
        parse_metric(values, &["commissionFbo"]) / 100.0,
    );
    expr = replace_token(
        &expr,
        "комиссия_fbs",
        parse_metric(values, &["commissionFbs"]) / 100.0,
    );
    expr = replace_token(
        &expr,
        "закуп",
        parse_metric(values, &["purchase", "ozPurchase"]),
    );
    expr = replace_token(&expr, "логистика", parse_metric(values, &["logistics"]));

    static CLEAN: OnceLock<Regex> = OnceLock::new();
    let clean = CLEAN.get_or_init(|| Regex::new(r"[^0-9+\-*/().,\s]").expect("clean"));
    let cleaned = clean.replace_all(&expr, "").replace(',', ".");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    eval_arith(cleaned)
}

fn normalize_profit_formula(expr: &str) -> String {
    let map = [
        ("Цена после скидки", "цена_после_скидки"),
        ("Комиссия FBS", "комиссия_fbs"),
        ("Комиссия FBO", "комиссия_fbo"),
        ("Доп. расходы", "доп_расходы"),
        ("Эквайринг", "эквайринг"),
        ("Реклама", "реклама"),
        ("Логистика", "логистика"),
        ("Налог", "налог"),
        ("Закуп", "закуп"),
    ];
    let mut s = expr.to_string();
    for (display, tag) in map {
        let re = Regex::new(&format!(r"(?i)\[?{}\]?", regex::escape(display))).expect("map");
        s = re.replace_all(&s, tag).to_string();
    }
    s
}

fn replace_token(expr: &str, token: &str, value: f64) -> String {
    let re = Regex::new(&format!("(?i){}", regex::escape(token))).expect("tok");
    re.replace_all(expr, format!("{value}")).to_string()
}

fn parse_metric(values: &HashMap<String, String>, keys: &[&str]) -> f64 {
    for k in keys {
        if let Some(v) = values
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
        {
            if let Some(n) = try_parse_number(v) {
                return n;
            }
        }
    }
    0.0
}

/// Простой рекурсивный спуск: + − * / ( ).
fn eval_arith(input: &str) -> Option<f64> {
    let chars: Vec<char> = input.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0usize;
    let result = parse_expr(&chars, &mut i)?;
    if i != chars.len() {
        return None;
    }
    Some(result)
}

fn parse_expr(chars: &[char], i: &mut usize) -> Option<f64> {
    let mut left = parse_term(chars, i)?;
    while *i < chars.len() {
        match chars[*i] {
            '+' => {
                *i += 1;
                left += parse_term(chars, i)?;
            }
            '-' => {
                *i += 1;
                left -= parse_term(chars, i)?;
            }
            _ => break,
        }
    }
    Some(left)
}

fn parse_term(chars: &[char], i: &mut usize) -> Option<f64> {
    let mut left = parse_factor(chars, i)?;
    while *i < chars.len() {
        match chars[*i] {
            '*' => {
                *i += 1;
                left *= parse_factor(chars, i)?;
            }
            '/' => {
                *i += 1;
                let r = parse_factor(chars, i)?;
                if r == 0.0 {
                    return None;
                }
                left /= r;
            }
            _ => break,
        }
    }
    Some(left)
}

fn parse_factor(chars: &[char], i: &mut usize) -> Option<f64> {
    if *i >= chars.len() {
        return None;
    }
    if chars[*i] == '(' {
        *i += 1;
        let v = parse_expr(chars, i)?;
        if *i >= chars.len() || chars[*i] != ')' {
            return None;
        }
        *i += 1;
        return Some(v);
    }
    if chars[*i] == '+' {
        *i += 1;
        return parse_factor(chars, i);
    }
    if chars[*i] == '-' {
        *i += 1;
        return Some(-parse_factor(chars, i)?);
    }
    parse_number(chars, i)
}

fn parse_number(chars: &[char], i: &mut usize) -> Option<f64> {
    let start = *i;
    while *i < chars.len() && (chars[*i].is_ascii_digit() || chars[*i] == '.') {
        *i += 1;
    }
    if start == *i {
        return None;
    }
    chars[start..*i].iter().collect::<String>().parse().ok()
}

pub fn format_profit_ru(n: f64) -> String {
    let s = format!("{n:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    s.replace('.', ",")
}
