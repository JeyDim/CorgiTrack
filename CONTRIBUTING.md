# Как внести вклад в CorgiTrack

Спасибо за интерес к проекту! 🐕 Любой вклад — баг-репорт, идея, документация или код —
приветствуется. Этот документ описывает, как всё устроено и чего мы ждём от pull request'ов.

Участвуя в проекте, вы соглашаетесь соблюдать [Кодекс поведения](CODE_OF_CONDUCT.md).

## С чего начать

- **Нашли баг?** Откройте [issue](https://github.com/JeyDim/CorgiTrack/issues/new/choose)
  по шаблону «Баг-репорт». Сначала поищите среди существующих — возможно, о нём уже знают.
- **Есть идея?** Заведите issue по шаблону «Предложение». Для крупных изменений лучше
  сначала обсудить подход в issue, прежде чем писать код.
- **Нашли уязвимость?** Не создавайте публичный issue — см. [SECURITY.md](SECURITY.md).

## Структура репозитория

| Каталог | Что внутри |
|---|---|
| `server/` | Rust-бэкенд: HTTP API (axum), Telegram-бот, iCal, шедулер |
| `desktop/` | Десктоп-клиент на Tauri 2 + Vue 3 (см. [`desktop/README.md`](desktop/README.md)) |
| `scripts/` | Вспомогательные скрипты (например, `set-version.sh`) |
| `.github/workflows/` | CI (`ci.yml`) и сборка релизов (`release.yml`) |

## Настройка окружения

Нужны **Rust (stable)**, **Node + npm** и **Docker** (для локального PostgreSQL).

```bash
cp .env.example .env   # впишите SERVICE_TOKEN (и TELEGRAM_BOT_TOKEN при необходимости)
docker compose up -d postgres

# Сервер
cd server
cargo run

# Десктоп-клиент (в отдельном терминале)
cd desktop
npm install
npm run tauri dev
```

Подробности — в [README.md](README.md) (сервер) и [desktop/README.md](desktop/README.md) (клиент).

## Стиль кода и проверки

Перед открытием PR убедитесь, что локально проходят те же проверки, что и в CI.

**Сервер (Rust):**

```bash
cd server
cargo fmt --all              # отформатировать
cargo fmt --all --check      # проверка форматирования (как в CI)
cargo clippy --all-targets -- -D warnings
cargo test
```

**Десктоп (фронтенд):**

```bash
cd desktop
npm run build                # vue-tsc (типы) + vite build
```

## Коммиты

Проект использует [Conventional Commits](https://www.conventionalcommits.org/ru/):

```
feat: добавил экспорт отчёта в PDF
fix: исправил расчёт времени приёма в нестандартной таймзоне
docs: обновил описание API в README
refactor: вынес расчёт расписания в отдельный модуль
chore: обновил зависимости
```

Сообщения можно писать на русском или английском — главное, осмысленно и в одном стиле
с историей проекта.

## Pull request

1. Сделайте форк и создайте ветку от `master` (например, `feat/pdf-report` или `fix/timezone`).
2. Внесите изменения; держите PR сфокусированным на одной задаче.
3. Прогоните проверки из раздела выше.
4. Обновите документацию (`README.md`, `.env.example` и т.п.), если поменялись поведение или конфигурация.
5. Откройте PR, заполнив шаблон, и свяжите его с issue (`Closes #123`), если он есть.

CI должен быть зелёным — после этого PR будет рассмотрен. Спасибо за вклад! 🙌
