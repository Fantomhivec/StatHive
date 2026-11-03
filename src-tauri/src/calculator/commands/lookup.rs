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

use crate::app_state::AppState;
use crate::calculator::{access_guard, card_reader, profile, snapshot};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct LookupArgs {
    pub article: String,
    #[serde(default)]
    pub marketplace: Option<String>,
    /// Игнорировать RAM-свод и перечитать с Google Sheets.
    #[serde(default)]
    pub force: bool,
}

#[tauri::command]
pub async fn wb_calculator_lookup_card(
    state: State<'_, Arc<AppState>>,
    args: LookupArgs,
) -> Result<Value, String> {
    access_guard::require_view(&state)?;
    let log = &state.log;
    let profile = profile::load();
    let gs = super::calc_sheets(&state).await?;

    let tables_mp = snapshot::resolve_tables_marketplace(&profile);
    let requested = args
        .marketplace
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let sheets_ready = crate::calculator::article_index::has_sheets();
    // Индекс/листы: только если both/auto и RAM ещё пуста (прогрев обычно на входе в калькулятор).
    let needs_index = !(tables_mp == "wb" || tables_mp == "ozon")
        && (requested.is_empty() || requested == "auto" || requested == "both")
        && !sheets_ready;

    if args.force {
        crate::calculator::article_index::invalidate_sheets();
    }

    let mut cache = crate::calculator::sheet_util::SheetCache::new();
    crate::calculator::article_index::fill_local_cache(&mut cache);
    if needs_index || args.force {
        crate::calculator::article_index::warm_from_summaries(
            &profile,
            &gs,
            &mut cache,
            Some(log),
        )
        .await;
    }

    let resolved = if tables_mp == "wb" || tables_mp == "ozon" {
        tables_mp
    } else if !requested.is_empty() && requested != "auto" && requested != "both" {
        requested
    } else {
        crate::calculator::article_index::resolve(&args.article)
            .0
            .unwrap_or("wb")
            .to_string()
    };

    Ok(card_reader::load_card(
        &profile,
        &args.article,
        Some(&resolved),
        &gs,
        &mut cache,
    )
    .await)
}

/// Фоновый прогрев сводов в RAM (вход на страницу калькулятора). Идемпотентно.
#[tauri::command]
pub async fn wb_calculator_warm_lookup_cache(
    state: State<'_, Arc<AppState>>,
) -> Result<Value, String> {
    access_guard::require_view(&state)?;
    let log = &state.log;
    let keys = crate::calculator::article_index::count();
    if crate::calculator::article_index::has_sheets() && keys > 0 {
        return Ok(serde_json::json!({ "ok": true, "cached": true, "keys": keys }));
    }
    let t0 = std::time::Instant::now();
    let profile = profile::load();
    let gs = super::calc_sheets(&state).await?;
    let mut cache = crate::calculator::sheet_util::SheetCache::new();
    crate::calculator::article_index::warm_from_summaries(&profile, &gs, &mut cache, Some(log))
        .await;
    let keys = crate::calculator::article_index::count();
    Ok(serde_json::json!({
        "ok": true,
        "cached": false,
        "keys": keys,
        "ms": t0.elapsed().as_millis(),
    }))
}
