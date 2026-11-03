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

use crate::templates::model::TemplatesFile;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const TEMPLATES_FILE: &str = "templates.json";

pub(crate) fn store_path() -> PathBuf {
    crate::core::team_profile::absorb_legacy_root_file(TEMPLATES_FILE);
    crate::core::team_profile::active_dir().join(TEMPLATES_FILE)
}

fn cache() -> &'static Mutex<Option<TemplatesFile>> {
    static CACHE: OnceLock<Mutex<Option<TemplatesFile>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Сброс RAM-кэша при смене команды (иначе шаблоны «переезжают» между слотами).
pub fn invalidate_cache() {
    if let Ok(mut g) = cache().lock() {
        *g = None;
    }
}

fn load_from_disk() -> Result<TemplatesFile, String> {
    let path = store_path();
    if !path.is_file() {
        return Ok(TemplatesFile::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut file: TemplatesFile =
        serde_json::from_str(&raw).map_err(|e| format!("templates.json: {e}"))?;
    file.normalize();
    Ok(file)
}

fn lock_cache() -> Result<std::sync::MutexGuard<'static, Option<TemplatesFile>>, String> {
    cache()
        .lock()
        .map_err(|_| "templates cache lock".to_string())
}

fn ensure_cached(slot: &mut Option<TemplatesFile>) -> Result<&mut TemplatesFile, String> {
    if slot.is_none() {
        *slot = Some(load_from_disk()?);
    }
    Ok(slot.as_mut().expect("cache filled"))
}

/// Чтение без глубокого clone всего файла.
pub(crate) fn with_file<R>(f: impl FnOnce(&TemplatesFile) -> R) -> Result<R, String> {
    let mut guard = lock_cache()?;
    let file = ensure_cached(&mut guard)?;
    Ok(f(file))
}

/// Мутация на месте; `dirty == true` → запись на диск (кэш уже обновлён).
pub(crate) fn with_file_mut<T>(
    f: impl FnOnce(&mut TemplatesFile) -> Result<(T, bool), String>,
) -> Result<T, String> {
    let mut guard = lock_cache()?;
    let file = ensure_cached(&mut guard)?;
    let (out, dirty) = f(file)?;
    if dirty {
        write_disk(file)?;
    }
    Ok(out)
}

fn write_disk(file: &TemplatesFile) -> Result<(), String> {
    let path = store_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    let compact = crate::templates::json_compact::compact_value(value);
    let raw = serde_json::to_string_pretty(&compact).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())?;
    Ok(())
}
