# StatHive

Бесплатный кроссплатформенный клиент выгрузки данных Wildberries и Ozon, калькуляторов и работы с Google Sheets / Excel. Стек: Tauri 2, Rust, React.

Приложение можно скачивать и использовать бесплатно. Исходный код — под [MIT License](LICENSE).

## Скачать

Публичные сборки появятся в [Releases](https://github.com/Fantomhivec/StatHive/releases) репозитория **StatHive** (после публикации). Пока репозиторий готовится — собирайте локально (`.\build.ps1`) или берите zip у команды.

| Платформа | Архив |
|-----------|--------|
| Windows x64 | `StatHive-win-x64.zip` |
| Linux x64 | `StatHive-linux-x64.zip` |
| macOS Apple Silicon | `StatHive-macos-aarch64.zip` |

В архиве — приложение и updater.

## Что умеет

- Выгрузка и обновление данных WB / Ozon
- Калькуляторы и экспорт в Excel / Google Sheets
- Команды: Admin настраивает OAuth и таблицу, участники входят по коду команды + логину/паролю
- Пресеты доступа к скриптам и калькулятору

## Быстрый старт

1. Скачайте zip своей ОС из Releases и распакуйте.
2. Первый вход и команда: [docs/TEAM_ONBOARDING.md](docs/TEAM_ONBOARDING.md).
3. Ключи маркетплейсов — файл `.env` (или `*.env`) **рядом с приложением**. Sheets — через user OAuth.

Секреты и ограничения модели (код команды, пароли в таблице): [docs/SECURITY_CREDENTIALS.md](docs/SECURITY_CREDENTIALS.md).

## Сборка из исходников

- Node.js 22+
- Rust stable (`rustup`)
- Windows: Visual Studio Build Tools с workload «Desktop development with C++»

```powershell
npm install
npm run tauri:dev
```

Локальный release: `.\build.ps1` (или `build.bat`).  
Выпуск версии на GitHub: [docs/RELEASE.md](docs/RELEASE.md).

## Документация

Оглавление: [docs/README.md](docs/README.md).

Релизы собираются только по тегу `v*` — [`.github/workflows/release.yml`](.github/workflows/release.yml).

## License

[MIT](LICENSE) — использование, изменение и распространение кода при сохранении copyright notice.  
Приложение **бесплатно** для использования (это отдельно от open-source лицензии на код).
