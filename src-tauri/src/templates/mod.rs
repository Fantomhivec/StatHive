pub(crate) mod access_gate;
mod commands;
mod json_compact;
mod model;
mod order;
pub(crate) mod run;
pub(crate) mod runs;
pub mod scheduler;
pub(crate) mod store;
pub(crate) mod trigger;

pub use commands::*;
pub use model::{SubjectsListFilters, Template, TemplateCard};

use crate::core::i18n::tf;
use crate::core::script::ScriptRegistry;
use crate::core::script_field::ScriptField;
use crate::templates::model::Template as TemplateModel;

/// Поля для редактора: сохранённый selection поверх DefaultFields скрипта.
pub fn list_fields_for_script(
    registry: &ScriptRegistry,
    script_key: &str,
    saved: Option<&[ScriptField]>,
) -> Result<Vec<ScriptField>, String> {
    let script = registry
        .get(script_key)
        .ok_or_else(|| tf("errors.templates.scriptNotFound", &[("scriptKey", script_key)]))?;
    let defaults = script.default_fields();
    if defaults.is_empty() {
        return Ok(saved.map(|s| s.to_vec()).unwrap_or_default());
    }
    let Some(saved) = saved.filter(|s| !s.is_empty()) else {
        return Ok(defaults);
    };
    let mut out: Vec<ScriptField> = Vec::new();
    let mut used = std::collections::HashSet::new();
    for s in saved {
        if let Some(d) = defaults.iter().find(|d| d.name == s.name) {
            out.push(ScriptField {
                name: d.name.clone(),
                title: if s.title.trim().is_empty() {
                    d.title.clone()
                } else {
                    s.title.clone()
                },
                selected: s.selected,
            });
            used.insert(d.name.clone());
        }
    }
    for d in defaults {
        if !used.contains(&d.name) {
            out.push(d);
        }
    }
    Ok(out)
}

pub fn new_template(
    registry: &ScriptRegistry,
    script_key: &str,
    marketplace: &str,
    tab_name: &str,
) -> Result<TemplateModel, String> {
    let script = registry
        .get(script_key)
        .ok_or_else(|| tf("errors.templates.scriptNotFound", &[("scriptKey", script_key)]))?;
    Ok(TemplateModel::new_blank(
        script.key(),
        &script.title(),
        script.default_fields(),
        marketplace,
        tab_name,
    ))
}

pub fn merge_fields_on_get(
    registry: &ScriptRegistry,
    mut tpl: TemplateModel,
) -> Result<TemplateModel, String> {
    tpl.selected_fields =
        list_fields_for_script(registry, &tpl.script_key, Some(&tpl.selected_fields))?;
    Ok(tpl)
}
