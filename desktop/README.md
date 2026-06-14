# CorgiTrack Desktop

Десктоп-клиент CorgiTrack на **Tauri 2 + Vue 3** — отдельная итерация поверх готового
Rust API (`../server`). Показывает ближайшие дозы, позволяет отмечать «принято/пропущено»
и управлять назначениями (лечениями) без Telegram.

## Стек

Tauri 2 · Vue 3 (`<script setup>` + TypeScript) · Vite · Pinia · vue-router · свой CSS
(корги-тема). Запросы к API идут через `@tauri-apps/plugin-http` (drop-in `fetch` через
Rust-стек — без проблем с CORS/CSP/mixed-content при произвольном адресе бэкенда).
Настройки (адрес, service-токен, выбранная семья) хранятся локально через
`@tauri-apps/plugin-store`.

## Требования

- Node + npm
- Rust stable (`cargo` в PATH; на этой машине — `~/.cargo/bin`)
- Windows: WebView2 (есть в Win11)

## Разработка

```powershell
cd desktop
npm install
npm run tauri dev
```

Должен быть запущен бэкенд (см. `../README.md`):

```powershell
# из корня репозитория
docker compose up -d postgres
cd server; cargo run   # с заданным SERVICE_TOKEN в .env
```

При первом запуске в окне приложения откройте **Настройки** → впишите адрес
(`http://localhost:8000`) и service-токен → «Проверить и сохранить» → выберите семью.

## Сборка релиза

```powershell
cd desktop
npm run tauri build
```

Артефакты — в `src-tauri/target/release/bundle/`.

## Структура

```
src/
  api/        types.ts (зеркало serde-моделей), client.ts (HTTP-клиент)
  stores/     settings.ts (подключение + plugin-store), toast.ts
  components/ StatusBadge, ToastHost, ModalDialog, TreatmentForm
  views/      Dashboard.vue, Treatments.vue, Settings.vue
  util/       format.ts (даты/время)
src-tauri/    Tauri 2: Cargo.toml, tauri.conf.json, capabilities/, src/lib.rs
```

Иконки приложения генерируются из `app-icon.png`:
`npm run tauri -- icon app-icon.png`.
