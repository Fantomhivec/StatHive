// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! OZON: Информация о товарах — POST /v3/product/info/list.
//! Паритет `OZ_ProductInfoScript`.

pub mod fields;

use crate::core::rate_profiles::ozon::product_info as rate;
use crate::core::script::{Script, ScriptContext, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_list_run::{self, ListRunParams};
use self::fields::{build_row, default_fields};
use crate::scripts::ozon_util::resolve_id_mode;
use std::future::Future;
use std::pin::Pin;

pub struct OzonProductInfoScript;

impl Script for OzonProductInfoScript {
    fn key(&self) -> &'static str {
        "ozon_product_info"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_product_info")
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
                    script_label: "OZON: Информация о товарах",
                    scope: rate::SCOPE,
                    profile: rate::profile(),
                    empty_ids_message: &empty,
                    rows_label: "информации о товарах",
                },
                common::resolve_fields(ctx, default_fields()),
                build_row,
            )
            .await
        })
    }
}
