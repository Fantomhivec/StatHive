# Новый скрипт

Краткий чеклист, если добавляете скрипт в StatHive.

## 1. Код и реестр

1. Модуль `src-tauri/src/scripts/<name>/` — `impl Script` (`key`, `title`, `default_fields`, `execute`).
2. `mod` + `register` в `src-tauri/src/scripts/mod.rs` → `build_registry()`.
3. Группа в `src-tauri/src/scripts/groups.rs` (иначе попадёт в «Прочее»).
4. В `groups.rs` тройка `(key, api_method, api_updated)`: при создании или обновлении скрипта **по официальной доке API** проставьте/смените `api_updated` на дату сверки в формате `dd.MM.yyyy`. У ещё не сверенных скриптов оставляйте `""` — заполняйте по мере ревизий. В комбо редактора шаблонов secondary = `endpoint · дата` (если дата есть).
5. Ключ в registry не меняйте после появления у пользователей (например `prices_by_nm`).
6. i18n: `locales/ru/app.json` → `scripts.titles.<key>` — название **всегда** с префиксом площадки: `WB: …` или `OZON: …` (по группе/маркетплейсу скрипта). Поля — `scripts.fields.<key>.*`.
7. При необходимости — видимость диапазонов в UI шаблонов (`scriptVisibility.ts`, поля шаблона в `templates/`).
8. Даты в Sheets — `core::sheet_dates` (`dd.MM.yyyy`).
9. HTTP к маркетплейсу — только через `ctx.request` (`RequestSender`) и общие rate-limit профили. Свой sleep + retry на 429 в скрипте не добавляйте.
10. Держите файлы обозримого размера; при разрастании режьте на модули.
11. `cargo check` в `src-tauri` — без warnings.
12. После регистрации скрипт появляется в комбо по ACL (ниже).

Если скрипт для калькулятора — ещё строка в `calculator/catalog.rs`.

## 2. Права (пресеты)

Полная модель: [USER_ACCESS.md](./USER_ACCESS.md).

`AccessLevel` на листе «Пользователи» указывает builtin Admin или **id пресета**. Каталог прав — лист «Пресеты».

| AccessLevel | Скрипты | Калькулятор | Вкладка «Пользователи» |
|-------------|---------|-------------|-------------------------|
| `Admin` | все | да | да |
| id пресета | ключи ∩ (OAuth или `memberOk`) | по CalcAccess | если в пресете есть `users` / `keysCheck` |

Приглашение участника всегда с выбранным пресетом.

### Чеклист ACL для нового скрипта

1. Registry + groups (+ catalog для calc).
2. Admin включает ключ в нужных пресетах или ставит «Все скрипты (*)».
3. `MemberOk(...)` в `scripts/mod.rs` — только если скрипт **только** тянет данные с API маркетплейса и **не** обязан читать Google Sheets (ID/диапазоны/task id). Без OAuth результат уйдёт в Excel. Не оборачивать: upload / set / move / trash / media / create-task / auto, download/status по id из листа, утилиты Sheets (copy/symbols/cells), выборки «по списку из таблицы».
4. Проверьте: Admin видит всё; ограниченный пресет — только свои ключи; без права — нет в комбо и run отклоняется.

## 3. WB API + контракты

Для WB-скрипта (не Sheets-utils) после кода и ACL:

1. Сверка schema по актуальной официальной доке API → `default_fields` / mapping → `api_updated` в `groups.rs` (`dd.MM.yyyy`). Ссылки на внешние API docs в репозиторий не класть.
2. Rate profile в `core/http/rate_profiles/**` по personal docs; `token_bucket` capacity **1.0**; без burst.
3. Каталог `src-tauri/wb_script_contracts/catalog/`:
   - строка в `scripts.json` (`kind: "api"` или `"sheets"`);
   - `rates_ours.json` / `rates_expected.json` (для API);
   - `fields/<key>.json` + фикстуры `fixtures/<key>/{ok,empty,error}.json` (для API; Sheets — skip).
4. Команды: `cargo check -p stathive` (0 warnings); `cargo test -p wb_script_contracts`.

Crate контрактов **не** подключать к `stathive`.

## 4. Документация

Права пользователей — [USER_ACCESS.md](./USER_ACCESS.md). Оглавление — [README.md](./README.md).
