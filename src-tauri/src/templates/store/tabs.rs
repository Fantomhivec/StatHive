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

use super::io::with_file_mut;
use super::templates::snapshot;
use crate::core::i18n::{t, tf};
use crate::templates::model::TemplatesSnapshot;

pub fn select_tab(marketplace: &str, tab: &str) -> Result<TemplatesSnapshot, String> {
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if !bundle.tabs.iter().any(|t| t == tab) {
            return Err(tf("errors.templates.tabNotFound", &[("tab", tab)]));
        }
        bundle.selected_tab = tab.to_string();
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

pub fn add_tab(marketplace: &str, name: &str) -> Result<TemplatesSnapshot, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(t("errors.templates.tabNameRequired"));
    }
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if bundle.tabs.iter().any(|t| t == name) {
            return Err(tf("errors.templates.tabAlreadyExists", &[("name", name)]));
        }
        if bundle.tabs.len() >= 20 {
            return Err(t("errors.templates.tooManyTabs"));
        }
        bundle.tabs.push(name.to_string());
        bundle.selected_tab = name.to_string();
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

pub fn rename_tab(marketplace: &str, from: &str, to: &str) -> Result<TemplatesSnapshot, String> {
    let to = to.trim();
    if to.is_empty() {
        return Err(t("errors.templates.newNameRequired"));
    }
    if crate::templates::model::is_system_tab(from) {
        return Err(t("errors.templates.cannotRenameSystemTab"));
    }
    if crate::templates::model::is_system_tab(to) {
        return Err(tf("errors.templates.tabAlreadyExists", &[("name", to)]));
    }
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if !bundle.tabs.iter().any(|t| t == from) {
            return Err(tf("errors.templates.tabNotFound", &[("tab", from)]));
        }
        if from != to && bundle.tabs.iter().any(|t| t == to) {
            return Err(tf("errors.templates.tabAlreadyExists", &[("name", to)]));
        }
        for t in &mut bundle.tabs {
            if t == from {
                *t = to.to_string();
            }
        }
        if bundle.selected_tab == from {
            bundle.selected_tab = to.to_string();
        }
        if bundle.default_tab == from {
            bundle.default_tab = to.to_string();
        }
        for t in &mut bundle.templates {
            if t.tab_name == from {
                t.tab_name = to.to_string();
            }
        }
        for g in &mut bundle.groups {
            if g.tab_name == from {
                g.tab_name = to.to_string();
            }
        }
        Ok(((), true))
    })?;
    snapshot(marketplace)
}

pub fn delete_tab(marketplace: &str, name: &str) -> Result<TemplatesSnapshot, String> {
    if crate::templates::model::is_system_tab(name) {
        return Err(t("errors.templates.cannotDeleteSystemTab"));
    }
    with_file_mut(|file| {
        let bundle = file.market_mut(marketplace);
        if bundle.tabs.len() <= 1 {
            return Err(t("errors.templates.cannotDeleteLastTab"));
        }
        if name == bundle.default_tab {
            return Err(t("errors.templates.cannotDeleteDefaultTab"));
        }
        if !bundle.tabs.iter().any(|t| t == name) {
            return Err(tf("errors.templates.tabNotFound", &[("tab", name)]));
        }
        bundle.tabs.retain(|t| t != name);
        bundle.templates.retain(|t| t.tab_name != name);
        bundle.groups.retain(|g| g.tab_name != name);
        if bundle.selected_tab == name {
            bundle.selected_tab = bundle.default_tab.clone();
        }
        bundle.ensure_tabs();
        Ok(((), true))
    })?;
    snapshot(marketplace)
}
