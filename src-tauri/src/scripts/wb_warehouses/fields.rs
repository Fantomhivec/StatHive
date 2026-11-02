use crate::core::script_field::ScriptField;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("name", true),
        field("officeId", true),
        field("id", true),
        field("cargoType", true),
        field("cargoTypeName", true),
        field("deliveryType", true),
        field("deliveryTypeName", true),
        field("isDeleting", true),
        field("isProcessing", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.warehouses_list.{name}")),
        selected,
    }
}

pub fn build_rows(cabinet: &str, warehouses: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    for wh in warehouses {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(cell(f.name.as_str(), cabinet, wh));
        }
        rows.push(row);
    }
    rows
}

fn cell(name: &str, cabinet: &str, wh: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "name" => str(wh, "name"),
        "officeId" => num(wh, "officeId"),
        "id" => num(wh, "id"),
        "cargoType" => num(wh, "cargoType"),
        "cargoTypeName" => cargo_name(wh.get("cargoType").and_then(|v| v.as_i64()).unwrap_or(0)),
        "deliveryType" => num(wh, "deliveryType"),
        "deliveryTypeName" => delivery_name(wh.get("deliveryType").and_then(|v| v.as_i64()).unwrap_or(0)),
        "isDeleting" => bool_str(wh, "isDeleting"),
        "isProcessing" => bool_str(wh, "isProcessing"),
        _ => String::new(),
    }
}

fn cargo_name(t: i64) -> String {
    match t {
        1 => "Малогабаритный товар (МГТ)".into(),
        2 => "Сверхгабаритный товар (СГТ)".into(),
        3 => "Крупногабаритный товар (КГТ+)".into(),
        _ => format!("Неизвестный ({t})"),
    }
}

fn delivery_name(t: i64) -> String {
    match t {
        1 => "Доставка на склад WB (FBS)".into(),
        2 => "Доставка силами продавца (DBS)".into(),
        3 => "Доставка курьером WB (DBW)".into(),
        5 => "Самовывоз (C&C)".into(),
        6 => "Экспресс-доставка силами продавца (ЕDBS)".into(),
        _ => format!("Неизвестный ({t})"),
    }
}

fn str(it: &Value, key: &str) -> String {
    it.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn num(it: &Value, key: &str) -> String {
    match it.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

fn bool_str(it: &Value, key: &str) -> String {
    match it.get(key) {
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Сжать одинаковые склады по кабинетам → «ВСЕ» / «A, B».
pub fn merge_cabinet_rows(
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    cabinet_col: usize,
    all_cabinets: &[String],
) -> Vec<Vec<String>> {
    use std::collections::{HashMap, HashSet};
    let mut grouped: HashMap<String, (Vec<String>, HashSet<String>)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for row in rows {
        let cab = row.get(cabinet_col).cloned().unwrap_or_default();
        let key: String = row
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != cabinet_col)
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join("\u{1f}");
        if let Some((_, cabs)) = grouped.get_mut(&key) {
            if !cab.trim().is_empty() {
                cabs.insert(cab);
            }
        } else {
            let mut cabs = HashSet::new();
            if !cab.trim().is_empty() {
                cabs.insert(cab);
            }
            grouped.insert(key.clone(), (row, cabs));
            order.push(key);
        }
    }
    let all_set: HashSet<String> = all_cabinets.iter().cloned().collect();
    let mut out = vec![header];
    for key in order {
        if let Some((mut row, cabs)) = grouped.remove(&key) {
            let label = if !all_set.is_empty() && cabs.len() == all_set.len() && cabs.is_subset(&all_set)
            {
                "ВСЕ".to_string()
            } else {
                let mut list: Vec<_> = cabs.into_iter().collect();
                list.sort();
                list.join(", ")
            };
            if cabinet_col < row.len() {
                row[cabinet_col] = label;
            }
            out.push(row);
        }
    }
    out
}
