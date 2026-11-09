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

use serde::Deserialize;

#[derive(Deserialize)]
pub struct MarketplaceArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
}

pub(super) fn default_wb() -> String {
    "wb".into()
}

#[derive(Deserialize)]
pub struct TemplatesGetArgs {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub script_key: String,
    #[serde(default = "default_wb")]
    pub marketplace: String,
    #[serde(default)]
    pub tab_name: String,
}

#[derive(Deserialize)]
pub struct SetSpreadsheetTitleArgs {
    pub id: String,
    #[serde(default)]
    pub spreadsheet_title: String,
}

#[derive(Deserialize)]
pub struct SetSpreadsheetTitlesArgs {
    #[serde(default)]
    pub items: Vec<SetSpreadsheetTitleArgs>,
}

#[derive(Deserialize)]
pub struct TemplatesIdArgs {
    pub id: String,
    /// Пробный прогон: скрипт выполняется, запись в Sheets/Excel пропускается.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct TemplatesListFieldsArgs {
    pub script_key: String,
    #[serde(default)]
    pub selected_fields: Vec<crate::core::script_field::ScriptField>,
}

#[derive(Deserialize)]
pub struct TemplatesCancelArgs {
    /// `template` | `group`
    #[serde(default = "default_entity_template")]
    pub entity_type: String,
    pub id: String,
}

fn default_entity_template() -> String {
    "template".into()
}

#[derive(Deserialize)]
pub struct TabNameArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct TabRenameArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
    pub from: String,
    pub to: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabColorArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
    pub name: String,
    /// Id пресета или пусто / none — сброс.
    #[serde(default)]
    pub color: String,
}

#[derive(Deserialize)]
pub struct GroupExpandArgs {
    pub id: String,
    pub expanded: bool,
}

#[derive(Deserialize)]
pub struct AddToGroupArgs {
    pub template_id: String,
    pub group_id: String,
}

#[derive(Deserialize)]
pub struct MoveTemplateArgs {
    pub template_id: String,
    pub direction: String,
    #[serde(default)]
    pub group_id: String,
}

#[derive(Deserialize)]
pub struct MoveGroupArgs {
    pub group_id: String,
    pub direction: String,
}
