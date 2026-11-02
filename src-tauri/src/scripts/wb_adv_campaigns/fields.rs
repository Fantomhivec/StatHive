use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("typeName", true),
        field("statusName", true),
        field("count", false),
        field("advertId", true),
        field("changeTime", true),
        field("all", false),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.adv_campaigns_list.{name}")),
        selected,
    }
}

pub fn build_rows(
    cabinet: &str,
    all: i64,
    groups: &[Value],
    fields: &[ScriptField],
) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    for g in groups {
        let typ = g.get("type").and_then(|v| v.as_i64()).unwrap_or(0);
        let status = g.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
        let count = g.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
        let list = g
            .get("advert_list")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        for it in list {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(cell(
                    f.name.as_str(),
                    cabinet,
                    all,
                    typ,
                    status,
                    count,
                    &it,
                ));
            }
            rows.push(row);
        }
    }
    rows
}

fn cell(
    name: &str,
    cabinet: &str,
    all: i64,
    typ: i64,
    status: i64,
    count: i64,
    item: &Value,
) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "type" => typ.to_string(),
        "typeName" => map_type(typ),
        "status" => status.to_string(),
        "statusName" => map_status(status),
        "count" => count.to_string(),
        "advertId" => num(item, "advertId"),
        "changeTime" => {
            let raw = str(item, "changeTime");
            if raw.is_empty() {
                raw
            } else {
                sheet_dates::format_wb_timestamp(&raw)
            }
        }
        "all" => all.to_string(),
        _ => String::new(),
    }
}

fn map_type(t: i64) -> String {
    match t {
        4 => "В каталоге (устар.)".into(),
        5 => "В карточке товара (устар.)".into(),
        6 => "В поиске (устар.)".into(),
        7 => "В рекомендациях на главной (устар.)".into(),
        8 => "Единая ставка (устар.)".into(),
        9 => "Единая или ручная ставка".into(),
        _ => format!("Тип {t}"),
    }
}

fn map_status(s: i64) -> String {
    match s {
        -1 => "Удалена (в процессе удаления)".into(),
        4 => "Готова к запуску".into(),
        7 => "Завершена".into(),
        8 => "Отменена".into(),
        9 => "Активна".into(),
        11 => "На паузе".into(),
        _ => format!("Статус {s}"),
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
