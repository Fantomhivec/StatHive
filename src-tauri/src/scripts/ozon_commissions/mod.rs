//! OZON: Комиссии — POST /v3/product/info/list (поле commissions).
//! Паритет `OZ_CommissionsScript`.

use serde_json::Value;
use crate::core::rate_profiles::ozon::commissions as rate;
use crate::core::script::{Script, ScriptContext, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_list_run::{self, ListRunParams};
use crate::scripts::ozon_util::resolve_id_mode;
use std::future::Future;
use std::pin::Pin;

pub struct OzonCommissionsScript;

impl Script for OzonCommissionsScript {
    fn key(&self) -> &'static str {
        "ozon_commissions"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_commissions")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move {
            let id_mode = resolve_id_mode(ctx);
            let empty = format!("Нет идентификаторов {id_mode} в источнике");
            ozon_list_run::run_one_row_per_item(
                ctx,
                ListRunParams {
                    script_label: "OZON: Комиссии",
                    scope: rate::SCOPE,
                    profile: rate::profile(),
                    empty_ids_message: &empty,
                    rows_label: "комиссий",
                },
                common::resolve_fields(ctx, default_fields()),
                build_row,
            )
            .await
        })
    }
}

// --- fields (merged) ---
use crate::scripts::ozon_util::{f2_ru, fill_from_item_and_source, json_str, row_from_map};
use std::collections::HashMap;

const SCHEMES: [&str; 4] = ["FBO", "FBS", "RFBS", "FBP"];

pub fn default_fields() -> Vec<ScriptField> {
    let mut fields = vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_commissions.cabinet"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_commissions.product_id"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_commissions.offer_id"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_commissions.sku"), true),
    ];
    for scheme in SCHEMES {
        fields.push(ScriptField::new(
            &format!("commission_{scheme}_delivery_amount"),
            &format!("Комиссия {scheme}: стоимость доставки"),
            true,
        ));
        fields.push(ScriptField::new(
            &format!("commission_{scheme}_percent"),
            &format!("Комиссия {scheme}: процент комиссии"),
            true,
        ));
        fields.push(ScriptField::new(
            &format!("commission_{scheme}_return_amount"),
            &format!("Комиссия {scheme}: стоимость возврата"),
            true,
        ));
        fields.push(ScriptField::new(
            &format!("commission_{scheme}_schema"),
            &format!("Комиссия {scheme}: схема продажи"),
            true,
        ));
        fields.push(ScriptField::new(
            &format!("commission_{scheme}_value"),
            &format!("Комиссия {scheme}: сумма комиссии"),
            true,
        ));
    }
    fields
}

pub fn build_row(
    item: &Value,
    cabinet: &str,
    fields: &[ScriptField],
    id_mode: &str,
    source_id: Option<&str>,
) -> Vec<String> {
    let ids = fill_from_item_and_source(item, id_mode, source_id);
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("cabinet".into(), cabinet.to_string());
    map.insert("offer_id".into(), ids.offer_id);
    map.insert("product_id".into(), ids.product_id);
    map.insert("sku".into(), ids.sku);
    for scheme in SCHEMES {
        map.insert(format!("commission_{scheme}_delivery_amount"), String::new());
        map.insert(format!("commission_{scheme}_percent"), String::new());
        map.insert(format!("commission_{scheme}_return_amount"), String::new());
        map.insert(format!("commission_{scheme}_schema"), String::new());
        map.insert(format!("commission_{scheme}_value"), String::new());
    }

    if let Some(comms) = item.get("commissions").and_then(|a| a.as_array()) {
        for c in comms {
            let schema = json_str(c.get("sale_schema"));
            let upper = schema.to_ascii_uppercase();
            if !SCHEMES.contains(&upper.as_str()) {
                continue;
            }
            map.insert(format!("commission_{upper}_schema"), schema);
            map.insert(format!("commission_{upper}_value"), f2_ru(c.get("value")));
            map.insert(format!("commission_{upper}_percent"), f2_ru(c.get("percent")));
            map.insert(
                format!("commission_{upper}_delivery_amount"),
                f2_ru(c.get("delivery_amount")),
            );
            map.insert(
                format!("commission_{upper}_return_amount"),
                f2_ru(c.get("return_amount")),
            );
        }
    }

    row_from_map(&map, fields)
}
