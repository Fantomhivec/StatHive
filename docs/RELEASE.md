# Релиз StatHive

Версия — файл [`VERSION`](../VERSION) (четыре сегмента, например `0.0.0.1`). В Cargo / npm / Tauri уходят первые три.

## Быстрый релиз

```powershell
.\scripts\release-next.ps1 -Yes
```

Скрипт:

1. Поднимает `VERSION` на один шаг.
2. Синхронизирует `package.json`, `Cargo.toml`, `tauri.conf.json`, `Cargo.lock`.
3. Коммит и тег `vX.Y.Z.W` → GitHub Actions [`release.yml`](../.github/workflows/release.yml).

Отдельные секреты в Actions для сборки не нужны. Для загрузки assets в Releases достаточно штатного `GITHUB_TOKEN`.

Репозиторий для in-app updates в бинарнике: **`Fantomhivec/StatHive`**. Имя репо на GitHub должно совпадать (иначе правьте `updates/embedded.rs`).

## Workflow

На GitHub Actions работает **только** сборка по тегу `v*` ([`release.yml`](../.github/workflows/release.yml)).

Матрица: Windows / Linux / macOS → zip в GitHub Releases.

Updater: `cargo build -p stathive_updater --release` в `src-tauri`.

In-app обновления ходят в публичный GitHub API **без** токена. Пока репозиторий private, «Проверить обновления» в клиенте не увидит релизы — раздавайте zip вручную. Подробнее: [UPDATE_SYSTEM.md](./UPDATE_SYSTEM.md).

## Правила версии

- Следующий релиз = ровно **+1** к текущему `VERSION`.
- Не перескакивайте номер из‑за уже существующего тега или сбоя Actions: пересоздайте тот же тег или разберитесь с ошибкой.

## Локальная проверка

```powershell
cd src-tauri
cargo check
cd ..
npx tsc --noEmit
```
