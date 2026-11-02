# Новый скрипт

Краткий чеклист, если добавляете скрипт в StatHive.

## 1. Код и реестр

1. Модуль `src-tauri/src/scripts/<name>/` — `impl Script` (`key`, `title`, `default_fields`, `execute`).
2. `mod` + `register` в `src-tauri/src/scripts/mod.rs` → `build_registry()`.
3. Группа в `src-tauri/src/scripts/groups.rs` (иначе попадёт в «Прочее»).
4. Ключ в registry не меняйте после появления у пользователей (например `prices_by_nm`).
5. При необходимости — видимость диапазонов в UI шаблонов (`scriptVisibility.ts`, поля шаблона в `templates/`).
6. Даты в Sheets — `core::sheet_dates` (`dd.MM.yyyy`).
7. HTTP к маркетплейсу — только через `ctx.request` (`RequestSender`) и общие rate-limit профили. Свой sleep + retry на 429 в скрипте не добавляйте.
8. Держите файлы обозримого размера; при разрастании режьте на модули.
9. `cargo check` в `src-tauri` — без warnings.
10. После регистрации скрипт появляется в комбо по ACL (ниже).

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
3. Если скрипт можно давать участнику **без** личного Google OAuth — оберните в `MemberOk(...)` при `register` в `scripts/mod.rs`. Не делайте так для upload / set / move / trash / media / create-task.
4. Проверьте: Admin видит всё; ограниченный пресет — только свои ключи; без права — нет в комбо и run отклоняется.

## 3. Документация

Права пользователей — [USER_ACCESS.md](./USER_ACCESS.md). Оглавление — [README.md](./README.md).
