use crate::app_state::AppState;
use crate::core::i18n::tf;
use crate::templates::model::{Template, TemplateGroup};
use crate::templates::run;
use crate::templates::store;
use crate::templates::trigger::TriggerType;
use chrono::{Local, NaiveDateTime};
use std::sync::Arc;
use std::time::Duration;

/// Фоновый опрос триггеров (TimeTriggerService, упрощённо).
pub fn spawn(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        loop {
            if let Err(e) = tick(&state).await {
                state.log.warn(format!("triggers tick: {e}"));
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

struct DueTemplate {
    tpl: Template,
    trigger_ids: Vec<String>,
}

struct DueGroup {
    group: TemplateGroup,
    trigger_ids: Vec<String>,
}

async fn tick(state: &AppState) -> Result<(), String> {
    let now = Local::now().naive_local();

    // refresh_next для пустых next_execution_at — один проход без clone всех.
    store::with_file_mut(|file| {
        let mut dirty = false;
        for tpl in file
            .wb
            .templates
            .iter_mut()
            .chain(file.ozon.templates.iter_mut())
        {
            for tr in &mut tpl.time_triggers {
                if tr.next_execution_at.is_none() {
                    tr.refresh_next();
                    dirty = true;
                }
            }
        }
        for group in file.wb.groups.iter_mut().chain(file.ozon.groups.iter_mut()) {
            for tr in &mut group.time_triggers {
                if tr.next_execution_at.is_none() {
                    tr.refresh_next();
                    dirty = true;
                }
            }
        }
        Ok(((), dirty))
    })?;

    let due_templates = store::with_file(|file| collect_due_templates(file, now))?;
    let due_groups = store::with_file(|file| collect_due_groups(file, now))?;

    for due in due_templates {
        for tid in due.trigger_ids {
            state
                .log
                .info(tf(
                    "log.templates.triggerTemplate",
                    &[("name", due.tpl.name.as_str())],
                ));
            let status = match run::run_one_template(state, &due.tpl, false).await {
                Ok((_, msg)) => {
                    state.log.info(tf(
                        "log.templates.triggerOk",
                        &[("message", msg.as_str())],
                    ));
                    "ok".to_string()
                }
                Err(e) => {
                    state.log.warn(tf(
                        "log.templates.triggerError",
                        &[("error", e.as_str())],
                    ));
                    format!("error: {e}")
                }
            };
            let once_done = due
                .tpl
                .time_triggers
                .iter()
                .find(|t| t.id == tid)
                .map(|t| t.trigger_type == TriggerType::Once)
                .unwrap_or(false);
            let _ = patch_template_trigger(&due.tpl.id, &tid, status, once_done);
        }
    }

    for due in due_groups {
        for tid in due.trigger_ids {
            state
                .log
                .info(tf(
                    "log.templates.triggerGroup",
                    &[("name", due.group.name.as_str())],
                ));
            let status = match run::run_group(state, &due.group.id, false).await {
                Ok((_, msg)) => {
                    state.log.info(tf(
                        "log.templates.triggerGroupOk",
                        &[("message", msg.as_str())],
                    ));
                    "ok".to_string()
                }
                Err(e) => {
                    state.log.warn(tf(
                        "log.templates.triggerGroupError",
                        &[("error", e.as_str())],
                    ));
                    format!("error: {e}")
                }
            };
            let once_done = due
                .group
                .time_triggers
                .iter()
                .find(|t| t.id == tid)
                .map(|t| t.trigger_type == TriggerType::Once)
                .unwrap_or(false);
            let _ = patch_group_trigger(&due.group.id, &tid, status, once_done);
        }
    }
    Ok(())
}

fn patch_template_trigger(
    template_id: &str,
    trigger_id: &str,
    status: String,
    once_done: bool,
) -> Result<(), String> {
    store::with_file_mut(|file| {
        let Some(tpl) = file
            .wb
            .templates
            .iter_mut()
            .chain(file.ozon.templates.iter_mut())
            .find(|t| t.id == template_id)
        else {
            return Ok(((), false));
        };
        let Some(tr) = tpl.time_triggers.iter_mut().find(|t| t.id == trigger_id) else {
            return Ok(((), false));
        };
        tr.last_executed_at = Some(Local::now().to_rfc3339());
        tr.last_execution_status = Some(status);
        if once_done {
            tr.is_enabled = false;
            tr.next_execution_at = None;
        } else {
            tr.refresh_next();
        }
        tpl.touch();
        Ok(((), true))
    })
}

fn patch_group_trigger(
    group_id: &str,
    trigger_id: &str,
    status: String,
    once_done: bool,
) -> Result<(), String> {
    store::with_file_mut(|file| {
        let Some(group) = file
            .wb
            .groups
            .iter_mut()
            .chain(file.ozon.groups.iter_mut())
            .find(|g| g.id == group_id)
        else {
            return Ok(((), false));
        };
        let Some(tr) = group.time_triggers.iter_mut().find(|t| t.id == trigger_id) else {
            return Ok(((), false));
        };
        tr.last_executed_at = Some(Local::now().to_rfc3339());
        tr.last_execution_status = Some(status);
        if once_done {
            tr.is_enabled = false;
            tr.next_execution_at = None;
        } else {
            tr.refresh_next();
        }
        Ok(((), true))
    })
}

fn collect_due_templates(file: &super::model::TemplatesFile, now: NaiveDateTime) -> Vec<DueTemplate> {
    file.wb
        .templates
        .iter()
        .chain(file.ozon.templates.iter())
        .filter_map(|tpl| {
            let trigger_ids: Vec<String> = tpl
                .time_triggers
                .iter()
                .filter(|tr| tr.should_execute_now(now))
                .map(|tr| tr.id.clone())
                .collect();
            if trigger_ids.is_empty() {
                None
            } else {
                Some(DueTemplate {
                    tpl: tpl.clone(),
                    trigger_ids,
                })
            }
        })
        .collect()
}

fn collect_due_groups(file: &super::model::TemplatesFile, now: NaiveDateTime) -> Vec<DueGroup> {
    file.wb
        .groups
        .iter()
        .chain(file.ozon.groups.iter())
        .filter_map(|group| {
            let trigger_ids: Vec<String> = group
                .time_triggers
                .iter()
                .filter(|tr| tr.should_execute_now(now))
                .map(|tr| tr.id.clone())
                .collect();
            if trigger_ids.is_empty() {
                None
            } else {
                Some(DueGroup {
                    group: group.clone(),
                    trigger_ids,
                })
            }
        })
        .collect()
}
