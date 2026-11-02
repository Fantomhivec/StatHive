//! Общая пагинация Ozon postings (FBO/FBS): cursor + has_next.

use crate::core::rate_limit::RateLimitProfile;
use crate::core::script::ScriptContext;
use crate::scripts::ozon_util::ozon_post;
use serde_json::Value;

pub async fn fetch_postings_pages<F>(
    ctx: &ScriptContext,
    cabinet: &str,
    client_id: &str,
    api_key: &str,
    path: &str,
    scope: &str,
    profile: &RateLimitProfile,
    mut body_for_cursor: F,
) -> Result<Vec<Value>, String>
where
    F: FnMut(&str) -> Value,
{
    let mut all = Vec::new();
    let mut cursor = String::new();
    let mut page = 0u32;

    loop {
        page += 1;
        let body = body_for_cursor(&cursor);
        let mut v = ozon_post(ctx, cabinet, scope, profile, path, client_id, api_key, &body).await?;

        let has_next = v
            .get("has_next")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        let next_cursor = v
            .get("cursor")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let postings = v
            .get_mut("postings")
            .and_then(|a| a.as_array_mut())
            .map(std::mem::take)
            .unwrap_or_default();

        ctx.log.info(format!(
            "Кабинет «{cabinet}»: страница {page}, postings={}, has_next={has_next}",
            postings.len()
        ));

        all.extend(postings);
        if !has_next {
            break;
        }
        if next_cursor.is_empty() || next_cursor == cursor {
            ctx.log.warn(format!(
                "Кабинет «{cabinet}»: has_next=true без нового cursor — стоп пагинации."
            ));
            break;
        }
        cursor = next_cursor;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Ok(all)
}
