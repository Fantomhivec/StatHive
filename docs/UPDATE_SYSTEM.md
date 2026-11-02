# Обновления

Клиент запрашивает `GET https://api.github.com/repos/{owner}/{repo}/releases/latest` **без** Bearer-токена — нужен **публичный** репозиторий. Без токена лимит GitHub ~**60 запросов/час на IP**; поэтому авто-проверка кэширует ответ (~6 ч) и после ошибки ждёт cooldown (~30 мин), а список релизов (`/releases`) грузится только по кнопке «Проверить обновления».

- Owner/repo зашиты в приложение: `Fantomhivec/StatHive`.
- Скачивается zip текущей ОС (предпочтительно `browser_download_url`); хост только GitHub; если API отдал `digest` (`sha256:…`) — проверяется после скачивания.
- Установка: временная папка + StatHive.Updater (или тихий install-скрипт).
- User OAuth: scopes `spreadsheets` + `drive.file` (без полного `drive`); SA по-прежнему с полным Drive.
- CSP в `tauri.conf.json` ограничен; `openUrl` — allowlist https (Discord, Google Docs/Drive, WB/Ozon, GitHub).

Если репозиторий private, API обычно отвечает 404/401 — клиент не найдёт обновление. Сборка релизов через Actions при этом может работать (см. [RELEASE.md](./RELEASE.md)).

Ответ 404 на `releases/latest` клиент трактует как «уже актуальная версия» (релизов ещё нет). Токен GitHub в приложении и updater **не используется** — только публичный API / `browser_download_url`.
