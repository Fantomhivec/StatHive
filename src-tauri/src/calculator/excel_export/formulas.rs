//! Формулы листа «Калькулятор» (паритет CalculatorExcelExportService.Formulas).

use regex::Regex;
use std::sync::OnceLock;

pub const SHEET_CALC: &str = "Калькулятор";
pub const SHEET_MAP: &str = "Соответствие";
pub const SHEET_WB: &str = "Данные ВБ";
pub const SHEET_OZ: &str = "Данные OZON";

pub fn lookup_formula(header: &str, scope: &str, wb_last: usize, oz_last: usize) -> String {
    lookup_formula_inner(header, scope, wb_last, oz_last, false)
}

/// Текст / даты / ТН-ВЭД: пустая ячейка в data-листе даёт INDEX=0 → иначе «00.01.1900» / «0».
pub fn lookup_formula_text(header: &str, scope: &str, wb_last: usize, oz_last: usize) -> String {
    lookup_formula_inner(header, scope, wb_last, oz_last, true)
}

fn lookup_formula_inner(
    header: &str,
    scope: &str,
    wb_last: usize,
    oz_last: usize,
    blank_zero: bool,
) -> String {
    let esc = header.replace('"', "\"\"");
    let wb_idx = format!(
        "INDEX('{SHEET_WB}'!$A$2:$ZZ${wb_last},MATCH($B$1,'{SHEET_WB}'!$A$2:$A${wb_last},0),MATCH(\"{esc}\",'{SHEET_WB}'!$1:$1,0))"
    );
    let oz_idx = format!(
        "INDEX('{SHEET_OZ}'!$A$2:$ZZ${oz_last},MATCH($B$1,'{SHEET_OZ}'!$A$2:$A${oz_last},0),MATCH(\"{esc}\",'{SHEET_OZ}'!$1:$1,0))"
    );
    let wrap = |idx: &str| -> String {
        if blank_zero {
            // INDEX пустой = 0; `x=0` также истинно для "".
            format!("IFERROR(IF(({idx})=0,\"\",{idx}),\"\")")
        } else {
            format!("IFERROR({idx},\"\")")
        }
    };
    let wb = wrap(&wb_idx);
    let oz = wrap(&oz_idx);
    match scope {
        "wb" => format!("IF($B$2=\"OZON\",\"\",{wb})"),
        "ozon" => format!("IF($B$2=\"OZON\",{oz},\"\")"),
        _ => format!("IF($B$2=\"OZON\",{oz},{wb})"),
    }
}

pub fn delta_formula(cur: &str, prev: &str) -> String {
    format!("IF($B$2=\"OZON\",\"\",{})", delta_core(cur, prev))
}

pub fn delta_formula_both(cur: &str, prev: &str) -> String {
    delta_core(cur, prev)
}

fn delta_core(cur: &str, prev: &str) -> String {
    // Пустой маркер «—» Formulas.cs (не ASCII "-").
    format!(
        "IF(OR({prev}=\"\",NOT(ISNUMBER(N({prev}))),N({prev})=0),\"—\",IFERROR((N({cur})-N({prev}))/N({prev}),\"—\"))"
    )
}

pub fn conv_formula(num: &str, den: &str) -> String {
    format!(
        "IF($B$2=\"OZON\",\"\",IF(OR({den}=\"\",N({den})=0),\"—\",IFERROR(N({num})/N({den}),\"—\")))"
    )
}

pub fn wb_only_label(text: &str) -> String {
    let esc = text.replace('"', "\"\"");
    format!("IF($B$2=\"OZON\",\"\",\"{esc}\")")
}

pub fn build_profit_excel_formula(
    formula: &str,
    price: &str,
    purchase: &str,
    comm_fbo: &str,
    comm_fbs: &str,
    logistics: &str,
) -> String {
    let mut expr = if formula.trim().is_empty() {
        "Цена после скидки*(1-Комиссия FBS-Налог-Эквайринг-Реклама-Доп. расходы)-Закуп-Логистика"
            .to_string()
    } else {
        formula.trim().to_string()
    };
    expr = normalize_profit_formula(&expr);

    expr = replace_token(&expr, "налог", "$AB$2");
    expr = replace_token(&expr, "эквайринг", "$AB$3");
    expr = replace_token(&expr, "реклама", "$AB$4");
    expr = replace_token(&expr, "доп_расходы", "$AB$5");
    expr = replace_token(&expr, "цена_после_скидки", &format!("N({price})"));
    expr = replace_token(&expr, "комиссия_fbo", &format!("N({comm_fbo})/100"));
    expr = replace_token(&expr, "комиссия_fbs", &format!("N({comm_fbs})/100"));
    expr = replace_token(&expr, "закуп", &format!("N({purchase})"));
    expr = replace_token(&expr, "логистика", &format!("N({logistics})"));

    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[^0-9+\-*/().,\s$A-Za-z]").expect("profit scrub"));
    expr = re.replace_all(&expr, "").replace(',', ".");
    if expr.trim().is_empty() {
        return "\"\"".into();
    }
    format!("IFERROR({expr},\"\")")
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
        let pat = format!(r"\[?{}\]?", regex::escape(display));
        if let Ok(re) = Regex::new(&format!("(?i){pat}")) {
            s = re.replace_all(&s, tag).into_owned();
        }
    }
    s
}

fn replace_token(expr: &str, token: &str, replacement: &str) -> String {
    let Ok(re) = Regex::new(&format!("(?i){}", regex::escape(token))) else {
        return expr.to_string();
    };
    // В replacement `$`/`$2` — группы захвата regex; для ячеек Excel нужен `$$`.
    let safe = replacement.replace('$', "$$");
    re.replace_all(expr, safe.as_str()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profit_keeps_ab_rate_refs() {
        let f = build_profit_excel_formula("", "B30", "D30", "B33", "D33", "B34");
        assert!(
            f.contains("$AB$2") && f.contains("$AB$5"),
            "expected AB rate refs, got: {f}"
        );
        assert!(!f.contains("----"), "dangling minuses from stripped $AB$: {f}");
    }

    #[test]
    fn profit_custom_spaced_formula_keeps_ab() {
        let f = build_profit_excel_formula(
            "Цена после скидки * (1 - Комиссия FBS - Доп. расходы - Эквайринг - Реклама - Налог) - Закуп - Логистика",
            "B30",
            "D30",
            "B33",
            "D33",
            "B34",
        );
        assert!(
            f.contains("$AB$2") && f.contains("$AB$5") && f.contains("N(B30)"),
            "expected AB+N refs, got: {f}"
        );
    }
}
