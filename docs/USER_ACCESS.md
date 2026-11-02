# Права пользователей

Документ для Admin и разработчиков: как устроены пресеты и вкладки.  
Код: `src-tauri/src/core/access/`, листы **Пресеты** и **Пользователи**.  
Первый вход — [TEAM_ONBOARDING.md](./TEAM_ONBOARDING.md). Секреты — [SECURITY_CREDENTIALS.md](./SECURITY_CREDENTIALS.md).

## Роли

| Кто | Как определяется | Что может |
|-----|------------------|-----------|
| **Основной Admin** | builtin `Admin` / `Админ` в `AccessLevel` | Всё: SA, код UR3 + пароль, лист TeamOAuth (publish/grants), открыть лист «Пользователи», удалить пользователя, каталог CalcTeam (publish/default/delete), полный UI |
| **Второй «админ»** | пресет «на всё» (скрипты `*`, нужные флаги calc/app) **и** свой Google OAuth (`HasOauth=да`) | Полные шаблоны и таблицы калькулятора по пресету; приглашения / пресеты / назначение конфига — если в пресете есть `users`. **Нет:** SA, открытие листа Users, delete user, publish CalcTeam, код команды |
| **Участник без OAuth** | любой не-Admin пресет, колонка `HasOauth` пуста | Узкий режим (ниже), даже если в пресете много галочек |
| **Участник с OAuth** | `HasOauth=да` **и** локальный Google token (`effective_has_oauth`) | Полный режим **по выданным флагам пресета**; один флаг без файла token → узкий режим |

Лист «Пользователи» участник **никогда** не открывает в браузере (кнопка UI только у main Admin). Колонки приложения (в т.ч. `HasOauth`, `CalcConfig`) пишет **Service Account**. Drive: участникам не выдавать writer на лист Users — защита ACL Google, не только UI (`workspaceId` может быть в dashboard у users).

## Пресет: что настраивается в UI

Вкладка **Пользователи → Пресеты** (`UsersAccessEditor`).

### CalcAccess (калькулятор)

| Флаг | Вкладка / действие |
|------|--------------------|
| `view` | Калькулятор (карточка) |
| `settings` | Настройки калькулятора |
| `tables` | Таблицы + **Drive reader** на Gmail участника по `tableLinks` конфига |
| `kgt` | Правила КГТ |
| `excel` | Выгрузка Excel **внутри** калькулятора (при уже открытом calc shell) |

Пустой набор → без калькулятора. Флаг `excel` **сам по себе** не включает режим калькулятора в shell — нужен хотя бы один из `view` / `settings` / `tables` / `kgt`. IPC `wb_calculator_export_excel` требует **и** calc-shell (`require_any_calc_tab`), **и** `require_excel`. Если в пресете только calc и нет скриптов — shell принудительно в режиме калькулятора.

`env_list_cabinet_names` (имена без секретов): `keysCheck` **или** любой разрешённый скрипт (`can_use_any_script_ex`, в т.ч. Excel-only / memberOk). Google list/check по-прежнему: `keysCheck` **или** (OAuth ∧ скрипты).

**Старые строки листа:** пустой CalcAccess и Calculator=да при чтении раскрываются во все флаги. При сохранении из UI пустой CalcAccess значит «без калькулятора».

### AppAccess (вкладки приложения)

| Флаг | Вкладка |
|------|---------|
| `keysCheck` | Проверка ключей (`.env`; Google — «Проверить подключение» при гранте TeamOAuth / Admin) |
| `users` | Пользователи: команда, приглашения, пресеты, смена CalcConfig |

Пусто → этих вкладок нет (Admin видит обе всегда).  
`*` в ячейке листа = все app-флаги.

### Scripts

- «Все скрипты (*)» или список ключей.
- Запуск: **пресет ∩ (Admin \| HasOauth \| `memberOk` у скрипта)**.
- `memberOk` задаётся обёрткой `MemberOk(Script)` при регистрации в `scripts/mod.rs` (по умолчанию без обёртки = **false**). В каталоге пресета помечается «без OAuth».

Новый скрипт: [AddingNewScript.md](./AddingNewScript.md) §2. Экспортные (без мутаций API маркетплейса) регистрируются как `MemberOk(...)` в `scripts/mod.rs` — участник без HasOauth видит их в пресете и запускает **только через шаблоны с Excel**. Мутаторы (upload/set/move/trash/media/create-task, в т.ч. `prices_history_task`) остаются без `MemberOk`. Прямого IPC `scripts_run` нет — только `templates_run*` (без OAuth запись в Google режется).

### CalcConfig (не часть пресета)

У каждого участника в листе Users — id конфигурации CalcTeam. Назначается при invite / в таблице команды. Drive share на таблицы конфига — **только если** в пресете есть `tables`.

## HasOauth (колонка J листа «Пользователи»)

| Значение | Смысл |
|----------|--------|
| `да` (или `true` / `yes` / `1`) | Участник подключил **user** Google token (connect) |
| пусто | Узкий режим |

**Как выставить:** нужен локальный Desktop client (auto-pull по гранту или paste Admin) → **Проверка ключей** → «Проверить подключение» → consent → SA пишет `HasOauth=да` и обновляет caps.  
Участник колонку вручную не редактирует. Admin видит её при «Открыть таблицу».

`HasOauth` ≠ грант TeamOAuth. Грант — ACL на лист **TeamOAuth** (командный Desktop client). Без гранта на shared PC нельзя connect, даже если `oauth_client.json` уже на диске от другого логина.

## TeamOAuth (грант Desktop client)

| Кто | Доступ к командному Desktop client |
|-----|-------------------------------------|
| Builtin / main Admin | **Всегда** (UI locked on); может вставить/опубликовать JSON |
| Участник в грантах | Auto-pull при входе/старте; «Проверить подключение» на Keys Check |
| Остальные | Нет кнопки / connect → `teamOauthDenied`; при логине локальный `oauth_client.json` **удаляется** |

После auto-pull — «Проверить подключение»; token на logout/kick; client без гранта — при логине. См. [SECURITY_CREDENTIALS.md](./SECURITY_CREDENTIALS.md).

## Режимы по OAuth (не-Admin)

```text
Нет HasOauth                    Есть HasOauth
─────────────────────────       ─────────────────────────
Только скрипты с memberOk       Все скрипты из пресета
Excel forced on + locked        Google и Excel по шаблону
Tables: просмотр + открытие     Tables: refresh / URL /
  (Drive reader если tables)      manual / тип заполнения
Нет записи в Google Sheets      (если в пресете tables)
```

IPC отвергает Google write шаблонов и мутации таблиц без OAuth (даже если UI обойти).

## Матрица «кто что делает» (кратко)

| Действие | Main Admin | users + OAuth | users без OAuth | keysCheck | tables + OAuth | tables без OAuth |
|----------|:----------:|:-------------:|:---------------:|:---------:|:--------------:|:----------------:|
| SA create/replace / код команды | ✓ | — | — | — | — | — |
| Открыть лист Users (кнопка UI) | ✓ | — | — | — | — | — |
| Удалить пользователя (+ revoke Drive) | ✓ | — | — | — | — | — |
| CalcTeam publish / default / delete | ✓ | — | — | — | — | — |
| Invite / пресеты / назначить CalcConfig | ✓ | ✓ | ✓† | — | — | — |
| Список CalcConfig для invite | ✓ | ✓ | ✓ | — | — | — |
| Проверка `.env` ключей | ✓ | — | — | ✓ | — | — |
| Connect свой Google OAuth (`connect_member_oauth`) | ✓* | — | — | ✓ | — | — |
| Save OAuth client / сырой connect/switch Google | ✓* | — | — | — | — | — |
| Полные шаблоны (Google) | ✓ | по скриптам | — | — | — | — |
| Шаблоны Excel-only (memberOk) | ✓ | ✓ | ✓ | — | — | — |
| Tables refresh / URL / fill | ✓ | — | — | — | ✓ | — |
| Tables только открыть / смотреть | ✓ | — | — | — | ✓ | ✓ |

\* Admin OAuth обычно уже есть с регистрации workspace; кнопка connect на Keys Check для Admin не обязательна. Save OAuth client / `connect_google` / switch — setup без сессии или main Admin (не keysCheck).  
† Лист Users/пресеты пишет **SA**. Drive-share на таблицы invitee при флаге `tables` у invitee требует **user OAuth менеджера**. **Fail-closed до append:** если Drive нужен, а OAuth менеджера нет — invite падает **до** записи строки Users. Остаточный риск «полуприглашённого»: OAuth был, append прошёл, а share/SA-share упал после записи строки.

## Типовые пресеты (рекомендации)

1. **Аналитик (узкий)** — нужные скрипты с `memberOk` где уместно; calc `view` (+ `excel` при необходимости); без `tables` / `users` / `keysCheck`. Работает без своего OAuth (Excel).
2. **Оператор таблиц** — calc: `view`+`tables` (+ settings/kgt по нужде); AppAccess: `keysCheck`; скрипты по списку. После connect OAuth — полный tables; до — только просмотр + Drive reader.
3. **Менеджер команды** — AppAccess: `users` (+ `keysCheck`); calc/scripts по политике. Без OAuth не получит полные шаблоны/таблицы. Не видит SA / delete / CalcTeam catalog.
4. **«Второй админ»** — scripts `*`, все CalcAccess, `keysCheck`+`users`; участник обязан подключить OAuth. Ограничения только admin-only операции выше.

## Листы Google

| Лист | Кто пишет | Содержание прав |
|------|-----------|-----------------|
| **Пресеты** | Admin / users через UI (SA) | Id, Title, Calculator, Scripts, CalcAccess, AppAccess |
| **Пользователи** | SA | AccessLevel = id пресета или `Admin`; CalcConfig; HasOauth |
| **CalcTeam** | main Admin (publish) | именованные конфиги; участник pull по своему CalcConfig |

## Связанный код

- ACL: `core/access/mod.rs`, `core/access/store.rs`
- Layout caps: `core/tabs_layout.rs` (`canKeysCheck`, `canUsersTab`, `hasOauth`, `hasEnvKeys`, `canLogs`, `canTablesTab`, `templatesExcelOnly`, `tablesCanMutate`, `isMainAdmin`)
- **Вкладки Шаблоны / Логи / Таблицы (calc):** не показываются, пока в `.env` нет хотя бы одного действующего ключа (WB не просрочен или OZON с value). Проверка ключей / выбор `.env` / сохранение редактора → `tabsLayout.changed`.
- OAuth connect / delete user / Drive после пресета: `auth/team.rs` (`resync_drive_for_user_row(old_level)`)
- Template gate: `templates/access_gate.rs`
- Tables mutate: `calculator/access_guard.rs` → `require_tables_mutate`
- UI: `UsersAccessEditor`, `UsersPage`, `KeysCheckPage`, `App.tsx`, `CalculatorTablesPage`, `TemplateEditorDialog`

## Живая сессия и смена пресета

- Activity timer и `refresh_profile` **перечитывают** строку Users (SA): пустой/orphan AccessLevel → clear session + `auth.accessRevoked`.
- Строка Users должна совпадать с сессией по **login** и **SessionId**; иначе kick (защита от подмены `row_index` в `session.json`); kick делает `take()` слота таймера — после логина `ensure_activity_timer` снова стартует.
- Смена пресета / HasOauth / **флагов тела пресета** (AppAccess / CalcAccess / Scripts при том же id) → `UserInfo` в RAM + `auth.sessionUpdated` + `tabsLayout.changed`. Activity двигает `last_emitted_fp` **только после успешного emit** (повтор на следующем тике, если UI/app не принял). UI: `refreshTabsLayout()` с **generation** кэша (устаревший `tabs_layout_get` не затирает свежий layout).
- В том же процессе: `access_set_user_preset` на свою строку → `refresh_profile` + emit сразу; `access_save_preset` своего id → `emit_session_updated` сразу (без ожидания 5 мин).
- Pull CalcTeam / snapshot: `tableLinks` только при флаге `tables` (`profile_for_user_drive`; иначе пусто в локальном профиле и в IPC snapshot).

## Шаблоны на диске

CRUD файлов шаблонов на машине — у любого с вкладкой «Шаблоны» (`require_any_script`). **Запуск** скрипта — строго по пресету ∩ (Admin \| OAuth \| memberOk).

## Accepted (вне матрицы ролей)

- `updates_*` — только авторизованная сессия (`require_user`; бинарь общий).
- `logs_get` / `logs_clear` — только авторизованная сессия (`require_user`).
## IPC harden

- `access_set_user_preset` не меняет builtin Admin и `row_index < 2`.
- Save OAuth client / `connect_google` / switch / `join_workspace`: без сессии только если workspace ещё не настроен; иначе main Admin (после logout на настроенной машине — нужен вход Admin).
- `auth_apply_team_invite`: **без сессии всегда** (хаб / смена команды на этой машине, см. TEAM_ONBOARDING); при живой сессии — только main Admin.
- `connect_member_oauth`: email consent сверяется с колонкой E (или пишется E, если пуста).
- Activity timer: 3 подряд ошибки `spreadsheet_id` / `auth_client` / `reload_presets_cache` / чтения строки Users → clear session + `auth.accessRevoked` (неуспешный reload пресетов сбрасывает кэш, без stale caps).
- Logout / kick сбрасывает `article_index` (кэш сводов калькулятора в процессе).

## Удаление пресета

- `access_delete_preset` после удаления строки на листе **Пресеты** очищает `AccessLevel` у пользователей с этим id (пусто → storage `Denied`) и снимает Drive reader (`profile_for_user_drive` → пустые `tableLinks`).
- Логин без валидного пресета (не Admin): `errors.auth.noProjectAccess` — «Нет прав на проект».
- Уже в сессии: timer перечитывает строку → clear session + `auth.accessRevoked` → UI на логин с тем же смыслом.

## Чеклист после смены пресета

- [ ] Нужные CalcAccess / AppAccess / Scripts отмечены.
- [ ] Участнику с `tables` назначен CalcConfig; Drive reader появится после sync / **publish** / **смены пресета** (old/new через `profile_for_user_drive`; не выдавать writer на лист Users).
- [ ] Если нужны Google Sheets в шаблонах или мутации tables — выдан `keysCheck` и пользователь подключил OAuth (при битом token — reconnect на Keys Check).
- [ ] Скрипты без OAuth помечены `memberOk` в коде (иначе без HasOauth вкладка «Шаблоны» может пропасть).
- [ ] Main Admin-only операции не делегируются через пресет.
- [ ] Invite / assign / смена пресета: IPC дожидается Drive; при сбое share ошибка в UI (пресет/строка Users может уже быть записана).
- [ ] Удаление пресета: AccessLevel очищен; orphan-пользователь не входит / выкидывается таймером (перечитывание строки).
- [ ] Даунгрейд пресета / правка флагов у живой сессии: нав и caps без релогина (свой процесс — сразу из access_*; чужая машина — тик activity с повтором emit; фронт — generation + `refreshTabsLayout`).
