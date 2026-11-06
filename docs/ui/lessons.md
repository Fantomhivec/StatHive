# UI lessons (StatHive)

Канон по темам: цвет **103** + `ui-color-hierarchy.mdc`; отступы **105**; select/комбо **111**; DnD **106/108**; hex **109**; opt **112**; login **113**; модалка footer **114**; realtime emit/yield **120** + `ui-realtime-updates.mdc`; stick-to-bottom логов **122**; last nested wave / итог группы **123**.

Архив инцидентов **1–114** свёрнут в правила `.cursor/rules` (файл журнала восстановлен с **115**).

## Новые инциденты

| # | Класс ошибки | Правило |
|---|--------------|---------|
| **115** | Group wave dual-source | Nested tint только из session `tones` (`run`/`ok`/`err`). Не красить из `is_running` + `active_template_id` + `last_run_status` одновременно — источники перетирают друг друга (залипание warning / «не красит»). |
| **116** | Session tint ≠ persist | `last_run_status` / `last_run_at` на диске. Фон волны — только до следующего Run группы или рестарта. Infer tint из `last_run_*` запрещён. |
| **117** | mergeEphemeral ≠ tint | `mergeEphemeralRunFlags` не решает цвет строки. Запрещены `finished → is_running:false` ради tint и sticky-OR `is_running` с `active_template_id` ради tint. Snap flags — только Stop/busy. |
| **118** | Stop not immediate | `templates_cancel` только ставит флаг. UI сразу: снять busy, tones `run`→`err`. Rust: опрос cancel в тиках rate-limit sleep и перед каждым Sheets chunk — иначе минута «тишины» на записи/лимите. |
| **119** | Wave warn stuck on first | (1) `setToneRun`: при сдвиге cursor старый `run`→`ok`, не delete. (2) Nested stop не null'ить `active_template_id` — merge `??` вернёт первый. (3) Зеркалить `wave_tones` в snap + красить через `waveFromGroup`. (4) `AppHandle` для emit — Mutex + refresh в `templates_run*`. |
| **120** | Progress emit batches | `templates.runProgress` из тела длинной async-команды без yield → WebView красит пачкой после `await`. Канон: `ui-realtime-updates.mdc` — mpsc + `spawn` + `yield_now` (как `wbCalculator.runLog`). |
| **121** | Tint ← Rust cursor | Живой курсор nested = `runs::active_tpl_map` → snapshot `active_template_id`. Краска CSS только `tones`, но двигать tones можно и с progress, и с poll hydrate → `advanceGroupWaveFromActive` (не второй CSS-источник). |
| **122** | Logs stick-to-bottom | Вкладка Логи: автоскролл только если пользователь у низа (`dist < 48`). Прокрутил вверх — позиция сохраняется; снова у низа — снова едет. Helper `stickScrollIfNearBottom`. |
| **123** | Run end stuck (group + solo) | **Симптом:** лог success / nested `ok`, UI «выполняется»+Stop (группа или **standalone**). **Не путать:** meta nested при `groupRunning` = диск `last_run_*`, не finish wave. **Канон:** (A) Wave tones: cursor `run`→`ok` (**119**); последний без next → finish/group-end `run`→`ok`/`err`, не `delete`; finish не поднимает `inFlight`. (B) **`mergeEphemeralRunFlags`: запрещён любой sticky-OR `is_running`** — у group, nested и **solo** (sticky после успеха оставлял Stop; mid-flight Stop = `busyId`). (C) Страховки: hydrate end + `finally` invoke (group finalize / solo clear `is_running`) + UI `waveSessionDone`. (D) Rust: opener/backfill spawn; emitter не silent-drop. (E) Итог группы — у имени, не сверху вкладки. (F) Tint warn≠ok: mix ~40%+ + inset edge. |
| **124** | Script change stale write keys | Смена `script_key` в редакторе шаблона обновляет `selected_fields`, но **не** сбрасывает `write_sort_field_name` / `sync_settings.key_field_*`. Комбо «Настройки записи» показывает сырое имя поля старого скрипта (нет в options). **Канон:** в `onScriptChange` после новых fields — оставить sort/keys только если имя есть в новом каталоге (sort — среди selected), иначе `""`. |
