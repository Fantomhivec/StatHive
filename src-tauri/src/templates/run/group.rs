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

use super::one::run_one_with_flag;
use crate::app_state::AppState;
use crate::core::i18n::t;
use crate::templates::{runs, store};
use chrono::Utc;
use std::sync::atomic::Ordering;

pub async fn run_group(
    state: &AppState,
    group_id: &str,
    dry_run: bool,
) -> Result<(usize, String), String> {
    let group = store::get_group(group_id)?.ok_or_else(|| t("errors.templates.groupNotFound"))?;
    let run_key = format!("group:{}", group.id);
    let cancelled = runs::register(&run_key);
    let started_at = Utc::now().to_rfc3339();
    runs::emit_progress_ex(
        "group",
        &group.id,
        Some("running"),
        true,
        Some(&started_at),
        None,
        Some(&group.id),
    );

    let mut total_rows = 0usize;
    let mut messages = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut was_cancelled = false;

    for tid in &group.template_ids {
        if cancelled.load(Ordering::Relaxed) {
            was_cancelled = true;
            break;
        }
        let tpl = match store::get(tid)? {
            Some(t) => t,
            None => continue,
        };
        runs::set_group_active_template(&group.id, &tpl.id);
        runs::emit_progress_ex(
            "group",
            &group.id,
            Some("running"),
            true,
            None,
            Some(&tpl.id),
            Some(&group.id),
        );
        // Share group cancel flag with each template run (nested register would double-count).
        match run_one_with_flag(state, &tpl, cancelled.clone(), dry_run).await {
            Ok((rows, msg)) => {
                total_rows += rows;
                messages.push(format!("{}: {msg}", tpl.name));
            }
            Err(e) => {
                let cancelled_msg = t("errors.cancelled");
                if cancelled.load(Ordering::Relaxed) || e.contains(&cancelled_msg) {
                    was_cancelled = true;
                    errors.push(format!("«{}»: {e}", tpl.name));
                    break;
                }
                errors.push(format!("«{}»: {e}", tpl.name));
                // Continue remaining templates (except cancel).
            }
        }
    }

    runs::clear_group_active_template(&group.id);
    runs::unregister(&run_key);
    let at = Utc::now().to_rfc3339();

    if was_cancelled {
        let status = "cancelled".to_string();
        let _ = store::set_group_last_run(&group.id, &status);
        runs::emit_progress_ex(
            "group",
            &group.id,
            Some(&status),
            false,
            Some(&at),
            None,
            Some(&group.id),
        );
        return Err(t("errors.cancelled"));
    }

    if !errors.is_empty() {
        let summary = errors.join("; ");
        let status = format!("error: {summary}");
        let _ = store::set_group_last_run(&group.id, &status);
        runs::emit_progress_ex(
            "group",
            &group.id,
            Some(&status),
            false,
            Some(&at),
            None,
            Some(&group.id),
        );
        return Err(summary);
    }

    let _ = store::set_group_last_run(&group.id, "ok");
    runs::emit_progress_ex(
        "group",
        &group.id,
        Some("ok"),
        false,
        Some(&at),
        None,
        Some(&group.id),
    );
    Ok((total_rows, messages.join("; ")))
}
