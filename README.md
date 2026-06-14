# CorgiTrack

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
| `MISSED_GRACE_MINUTES` | `120` | через сколько доза считается пропущенной |
| `REMINDER_LOOKAHEAD_MINUTES` | `30` | за сколько до приёма слать напоминание |
| `SCHEDULER_TICK_SECONDS` | `60` | период фонового шедулера |
| `RUST_LOG` | `info` | уровень логирования |

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
