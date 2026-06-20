# CorgiTrack 🐕

> Учёт таблеток и прививок собаки: Rust-бэкенд (API + Telegram-бот + iCal) и десктоп-клиент на Tauri.

[![CI](https://github.com/JeyDim/CorgiTrack/actions/workflows/ci.yml/badge.svg)](https://github.com/JeyDim/CorgiTrack/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/JeyDim/CorgiTrack?sort=semver)](https://github.com/JeyDim/CorgiTrack/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Бэкенд для учёта таблеток и прививок собаки (Rust). Лёгкий бинарь для Raspberry Pi:

- **PostgreSQL** как основной источник данных.
- **HTTP API** на axum с защитой через service-токен (для десктоп-клиента и интеграций).
- **Telegram-бот** для семейных напоминаний и подтверждений.
- **iCal-подписка** для Google Calendar / Apple Calendar.
- **CSV-отчёт** с датами, когда дозы были приняты.

Раньше проект был на Python (FastAPI + aiogram); переписан на Rust ради меньшего
потребления памяти. Исходный код — в каталоге `server/`.

## Стек

axum · sqlx (PostgreSQL) · teloxide · chrono/chrono-tz · icalendar — всё на rustls
(без OpenSSL).

## Репозиторий

- `server/` — Rust-бэкенд: HTTP API (axum), Telegram-бот, iCal, шедулер.
- `desktop/` — десктоп-клиент на Tauri 2 + Vue 3 (см. [`desktop/README.md`](desktop/README.md)).
- `scripts/` — вспомогательные скрипты (например, `set-version.sh`).
- `.github/workflows/` — CI (`ci.yml`) и сборка релизов (`release.yml`).

## Локальный запуск

```powershell
copy .env.example .env   # затем впишите SERVICE_TOKEN и TELEGRAM_BOT_TOKEN
docker compose up -d postgres
cd server
cargo run
```

Открыть:

- API health: `http://localhost:8000/health`
- ссылка календаря: команда бота `/calendar` или `GET /api/v1/households/{id}/calendar-url`

Нужен Rust (stable). Сборка релиза: `cargo build --release` — бинарь в
`server/target/release/corgitrack`.

## Запуск через Docker

```powershell
copy .env.example .env
docker compose up --build
```

Приложение доступно на `http://localhost:8000`. Внутри Compose база — хост `postgres`,
при локальном `cargo run` в `.env` оставьте `localhost`.

Для Raspberry Pi (arm64) образ собирается прямо на устройстве (`docker compose up
--build`) или кросс-сборкой: `docker buildx build --platform linux/arm64 ./server`.

## Схема и совместимость

При старте сервис идемпотентно создаёт типы/таблицы/индексы
(`server/migrations/0001_bootstrap.sql`) — повторяет схему прежней Python-версии,
поэтому существующую базу на Pi можно использовать без потери данных.

## Минимальные начальные данные

Можно через защищённый API (см. ниже) или SQL:

```sql
insert into households (name, calendar_token) values ('Семья', md5(random()::text));

insert into dogs (household_id, name)
select id, 'Корги' from households where name = 'Семья';

insert into family_members (household_id, display_name, telegram_user_id)
select id, 'Я', 111111111 from households where name = 'Семья';

insert into treatments
  (dog_id, name, kind, dose_label, cycle_days, start_at, reminder_time, instructions, active)
select d.id, 'Таблетка от паразитов', 'pill', '1 таблетка', 90, now(), '09:00', 'Дать после еды', true
from dogs d
join households h on h.id = d.household_id
where h.name = 'Семья';
```

`cycle_days` задаёт период приёма. Прививки — та же модель с `kind = 'vaccine'` и
обычно более длинным циклом.

## Конфигурация (переменные окружения)

| Переменная | По умолчанию | Назначение |
|---|---|---|
| `DATABASE_URL` | `postgres://corgitrack:corgitrack@localhost:5432/corgitrack` | строка подключения (схема `postgres://`) |
| `SERVICE_TOKEN` | `change-me` | Bearer-токен для `/api/v1/**` |
| `PUBLIC_BASE_URL` | `http://localhost:8000` | базовый URL в ссылках календаря |
| `APP_TIMEZONE` | `Europe/Astrakhan` | таймзона для расчёта времени приёма |
| `BIND_ADDR` | `0.0.0.0:8000` | адрес HTTP-сервера |
| `TELEGRAM_BOT_TOKEN` | — | если пусто, бот отключён |
| `TELEGRAM_API_SERVER_URL` | `https://tgproxy.advsrvone.pw/` | кастомный Bot API server; очистите для прямого доступа |
| `RUST_LOG` | `info` | уровень логирования |

> Тайминги эскалации, окно напоминаний и период шедулера вынесены из env в БД
> (таблица `app_settings`) и правятся на лету через `GET/PATCH /api/v1/settings` —
> см. ниже. Шедулер перечитывает их каждый тик, перезапуск не нужен.

## HTTP API

Публичные (без токена):

- `GET /health`
- `GET /calendar/{token}.ics` — iCal-подписка по `calendar_token` семьи
- `GET|POST /api/doses/{id}/taken?key=…` — отметка приёма по ссылке из календаря

Защищённые — заголовок `Authorization: Bearer <SERVICE_TOKEN>`, префикс `/api/v1`:

- `households` · `dogs` · `members` · `treatments` — CRUD
  (`GET` список, `POST` создать, `GET|PATCH|DELETE /{id}`)
- `GET /doses?household_id=&from=&to=&status=` — список доз
- `POST /doses/{id}/status` — тело `{ "status": "taken|skipped|...", "note"?, "member_id"? }`
- `GET /households/{id}/due?lookahead_hours=24` — ближайшие дозы
- `GET /households/{id}/report.csv` — CSV принятых доз
- `GET /households/{id}/calendar-url` — готовая ссылка iCal
- `GET /settings` — текущие операционные настройки (тайминги эскалации и т.д.)
- `PATCH /settings` — частичное обновление настроек; тело, например,
  `{ "escalation_first_delay_minutes": 30, "escalation_step_minutes": 5,
  "reminder_lookahead_minutes": 30, "scheduler_tick_seconds": 60 }`

Пример:

```bash
curl -H "Authorization: Bearer $SERVICE_TOKEN" http://localhost:8000/api/v1/households
```

CORS разрешён для всех источников (включая будущий десктоп на Tauri).

## Команды Telegram

По умолчанию клиент ходит через кастомный Bot API server
(`TELEGRAM_API_SERVER_URL`).

- `/start` — привязывает Telegram-пользователя, если его ID уже есть в
  `family_members.telegram_user_id`; иначе бот сообщает ваш ID.
- `/today` — дозы, которые скоро нужно принять (с кнопкой «Принято»).
- `/calendar` — ссылка iCal для Google/Apple Calendar.
- `/report` — CSV с датами принятых доз.

### Эскалация напоминаний

Напоминания рассылаются не всем сразу, а по очереди — в порядке
`family_members.escalation_order` (0 уведомляется первым). Шаги:

1. В момент приёма первое напоминание получает только участник с
   `escalation_order = 0`.
2. Если за `escalation_first_delay_minutes` (по умолчанию 30 мин) никто не нажал
   «Принято» — тому же участнику приходит повторный вопрос.
3. Далее каждые `escalation_step_minutes` (по умолчанию 5 мин) уведомляется
   следующий участник по списку.
4. Когда список исчерпан, доза помечается пропущенной (`missed`), а первому
   участнику уходит уведомление об этом.

Любое нажатие «Принято» закрывает дозу и останавливает эскалацию для всей семьи.
Учитываются только участники с `notify = true` и заданным `telegram_user_id`.
Тайминги (`escalation_first_delay_minutes`, `escalation_step_minutes`,
`reminder_lookahead_minutes`, `scheduler_tick_seconds`) хранятся в таблице
`app_settings` и правятся через `PATCH /api/v1/settings` без перезапуска сервиса.

## Тесты

```powershell
cd server
cargo test
```

Юнит-тесты (`server/tests/schedules.rs`) проверяют расчёт времени приёма и ссылки
календаря и не требуют базы данных.

## Модель синхронизации календаря

MVP использует ссылку подписки iCal — работает в Apple/Google Calendar без OAuth.
Telegram остаётся интерфейсом для действий «принято»/«пропущено».

## Как внести вклад

Issues и pull request'ы приветствуются! Перед началом загляните в:

- [CONTRIBUTING.md](CONTRIBUTING.md) — настройка окружения, стиль кода и процесс PR.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — правила общения в проекте.
- [SECURITY.md](SECURITY.md) — как сообщить об уязвимости.

Коротко: форкните репозиторий, создайте ветку, убедитесь, что `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings` и `cargo test` проходят, и откройте PR с описанием.

## Лицензия

[MIT](LICENSE) © Sergey Novik
