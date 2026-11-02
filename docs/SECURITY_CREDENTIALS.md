# Безопасность и учётные данные

StatHive не кладёт облачные ключи и OAuth-секреты в репозиторий и не вшивает их в релизный бинарник.

## Где что лежит

| Данные | Где | В git | At-rest |
|--------|-----|-------|---------|
| Индекс команд | `%AppData%/StatHive/teams_index.json` | нет | plaintext (только id/названия) |
| OAuth Desktop | `teams/{id}/oauth_client.json` | нет | **plaintext** (не зависит от keyring; старый sealed мигрирует) |
| Service Account | `teams/{id}/service_account.json` | нет | **plaintext** (вход по коду не зависит от keyring) |
| Google token | `teams/{id}/google_oauth_token.json` | нет | **plaintext** (не зависит от keyring; старый sealed мигрирует) |
| Сессия | `teams/{id}/session.json` | нет | **plaintext** (не зависит от keyring; старый sealed мигрирует) |
| ID таблицы | `teams/{id}/workspace.json` | нет | plaintext |
| Ключи WB / Ozon | `*.env` **рядом с exe** | нет | **plaintext** (удобный KEY=VALUE) |

Несколько команд — отдельные `teams/{id}/`. SA, workspace, session, Google user token и OAuth Desktop client — plaintext в AppData (не в git). Старый sealed client/token/session/SA при удачном decrypt мигрирует в plaintext; битый sealed client → auto-pull/paste; битый sealed token → «Проверить подключение»; битый session → вход заново; битый SA → снова ввести код UR3.

Токены (access / refresh / `client_secret`) **не** пишутся в логи; в UI — только укороченный hint `client_id`.

### Файл ключей маркетплейсов

- Обычный plaintext в папке с приложением. Участникам не передают и в код команды не кладут.
- Редактор — только при праве `keysCheck`. Экспорт — бэкап plaintext на доверенный носитель.

Пароль Admin/участника и **пароль кода UR3** — минимум **8** символов. Смена пароля пользователя ротирует `SessionId` (другие ПК с тем же session отвалятся). Смена пароля кода → новый UR3 участникам.

## Код команды и пароли

Пошагово: [TEAM_ONBOARDING.md](./TEAM_ONBOARDING.md).

В коде **`UR3.…`**: workspace id/title + Service Account в ciphertext (Argon2id + AES-GCM, случайные salt/nonce). **Нет** OAuth Desktop, **нет** ключей WB/Ozon. Пустой пароль запрещён.

Устаревший `UR2.…` (OAuth+SA в обфусцированном payload) отклоняется.

Пароли пользователей в листе «Пользователи» хранятся как **argon2id**. Старые plaintext при успешном входе переписываются в хэш.

Участникам не выдаётся Drive writer на таблицу Users — лист читает Service Account.

### TeamOAuth (лист на таблице Users)

Admin публикует Desktop client JSON и гранты `Login | Allowed`. **Builtin / main Admin всегда допущен**. Участники по умолчанию без доступа; с грантом Desktop client подтягивается **автоматически при входе/старте** (без кнопки). Вставка Desktop JSON локально — **только main Admin** (setup или экран Admin / Keys Check).

Файлы `oauth_client.json` / `google_oauth_token.json` лежат в слоте команды. Использовать их может только Admin или участник с грантом. На shared PC: logout/kick удаляет **token**; при логине **без гранта** удаляется и **`oauth_client.json`** (чтобы чужой JSON не оставался на диске). Логин с грантом снова подтягивает client через auto-pull.

Грант — **ACL приложения**. Владелец SA + пароля UR3 всё ещё может читать лист TeamOAuth через Sheets API вне UI.

`cloud-platform` запрашивается **только** при «Создать Service Account» из приложения. Ручная вставка SA JSON и обычный Google OAuth идут без этого scope.

## Ограничения модели (остаток)

| Уровень | Риск |
|---------|------|
| Средний | Владелец SA / пароля UR3 может читать лист Users и TeamOAuth через API (пароли Users уже без plaintext). Грант TeamOAuth не заменяет контроль над SA. |
| Низкий / ОС | AppData на диске читает любой процесс под тем же пользователем ОС; ACL гранта закрывает чужой логин в приложении, не чужой доступ к файлам с диска. |
| Средний | `.env` рядом с exe читает любой с доступом к папке приложения. |
| Низкий | Передача пароля кода UR3 — вне приложения; не публикуйте код и пароль вместе в открытых каналах. |

Сессия сверяет login и SessionId со строкой Users (пустой локальный SessionId — не ok).

## Не коммитьте

`.env` / `*.env`, OAuth/SA/session/workspace JSON, PEM/P12, personal access tokens, OAuth client secrets.
