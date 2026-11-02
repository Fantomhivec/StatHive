use crate::templates::store::{self, find_market_for_template};

fn parse_dir(direction: &str) -> Result<i8, String> {
    match direction {
        "up" => Ok(-1),
        "down" => Ok(1),
        _ => Err("direction: up|down".into()),
    }
}

/// Переместить шаблон: standalone на вкладке или внутри группы (`group_id`).
pub fn move_template(
    template_id: &str,
    direction: &str,
    group_id: Option<&str>,
) -> Result<(), String> {
    let dir = parse_dir(direction)?;
    store::with_file_mut(|file| {
        let market = find_market_for_template(file, template_id)
            .ok_or_else(|| "Шаблон не найден".to_string())?;

        if let Some(gid) = group_id.filter(|s| !s.trim().is_empty()) {
            let bundle = file.market_mut(&market);
            let group = bundle
                .groups
                .iter_mut()
                .find(|g| g.id == gid)
                .ok_or_else(|| "Группа не найдена".to_string())?;
            let idx = group
                .template_ids
                .iter()
                .position(|id| id == template_id)
                .ok_or_else(|| "Шаблон не в группе".to_string())?;
            let to = idx as isize + dir as isize;
            if to < 0 || to as usize >= group.template_ids.len() {
                return Ok(((), false));
            }
            group.template_ids.swap(idx, to as usize);
            return Ok(((), true));
        }

        let bundle = file.market_mut(&market);
        let tab = bundle
            .templates
            .iter()
            .find(|t| t.id == template_id)
            .map(|t| t.tab_name.clone())
            .ok_or_else(|| "Шаблон не найден".to_string())?;
        let in_group: std::collections::HashSet<String> = bundle
            .groups
            .iter()
            .filter(|g| g.tab_name == tab)
            .flat_map(|g| g.template_ids.iter().cloned())
            .collect();
        let standalone_indices: Vec<usize> = bundle
            .templates
            .iter()
            .enumerate()
            .filter(|(_, t)| t.tab_name == tab && !in_group.contains(&t.id))
            .map(|(i, _)| i)
            .collect();
        let pos = standalone_indices
            .iter()
            .position(|&i| bundle.templates[i].id == template_id)
            .ok_or_else(|| "Шаблон в группе — перемещайте внутри группы".to_string())?;
        let neighbor = (pos as isize + dir as isize) as usize;
        if neighbor >= standalone_indices.len() {
            return Ok(((), false));
        }
        let a = standalone_indices[pos];
        let b = standalone_indices[neighbor];
        bundle.templates.swap(a, b);
        Ok(((), true))
    })
}

pub fn move_group(group_id: &str, direction: &str) -> Result<(), String> {
    let dir = parse_dir(direction)?;
    store::with_file_mut(|file| {
        let market = if file.wb.groups.iter().any(|g| g.id == group_id) {
            "wb"
        } else if file.ozon.groups.iter().any(|g| g.id == group_id) {
            "ozon"
        } else {
            return Err("Группа не найдена".into());
        };
        let bundle = file.market_mut(market);
        let tab = bundle
            .groups
            .iter()
            .find(|g| g.id == group_id)
            .map(|g| g.tab_name.clone())
            .ok_or_else(|| "Группа не найдена".to_string())?;
        let same_tab: Vec<usize> = bundle
            .groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.tab_name == tab)
            .map(|(i, _)| i)
            .collect();
        let pos = same_tab
            .iter()
            .position(|&i| bundle.groups[i].id == group_id)
            .ok_or_else(|| "Группа не найдена".to_string())?;
        let neighbor = (pos as isize + dir as isize) as usize;
        if neighbor >= same_tab.len() {
            return Ok(((), false));
        }
        let a = same_tab[pos];
        let b = same_tab[neighbor];
        bundle.groups.swap(a, b);
        Ok(((), true))
    })
}

pub fn remove_from_group(template_id: &str) -> Result<(), String> {
    store::with_file_mut(|file| {
        let market = find_market_for_template(file, template_id)
            .ok_or_else(|| "Шаблон не найден".to_string())?;
        let bundle = file.market_mut(&market);
        let mut changed = false;
        for g in &mut bundle.groups {
            let before = g.template_ids.len();
            g.template_ids.retain(|id| id != template_id);
            if g.template_ids.len() != before {
                changed = true;
            }
        }
        Ok(((), changed))
    })
}
