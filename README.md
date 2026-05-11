# CorgiTrack

Бэкенд MVP для учета таблеток и прививок собаки:

- PostgreSQL как основной источник данных.
- Telegram-бот для семейных напоминаний и подтверждений.
- iCal-подписка для Google Calendar / Apple Calendar.
- CSV-отчет с датами, когда дозы были приняты.

## Локальный запуск

```powershell
copy .env.example .env
docker compose up -d postgres
pip install -e .
python -m corgitrack.main
```

Открыть:

- API: `http://localhost:8000/docs`
- ссылка календаря: команда бота `/calendar`

## Запуск через Docker

```powershell
copy .env.example .env
docker compose up --build
```

Контейнер приложения доступен на `http://localhost:8000`. Внутри Docker Compose приложение использует `postgres` как хост базы данных, а при локальном запуске в `.env` можно оставить `localhost`.

## Минимальные начальные данные

После первого старта сервис автоматически создает таблицы. Семью, собаку, Telegram ID и назначения можно добавить SQL-запросами:

```sql
insert into households (name) values ('Семья');

insert into dogs (household_id, name)
select id, 'Корги' from households where name = 'Семья';

insert into family_members (household_id, display_name, telegram_user_id)
select id, 'Я', 111111111 from households where name = 'Семья';

insert into family_members (household_id, display_name, telegram_user_id)
select id, 'Жена', 222222222 from households where name = 'Семья';

insert into treatments
  (dog_id, name, kind, dose_label, cycle_days, start_at, reminder_time, instructions, active)
select d.id, 'Таблетка от паразитов', 'pill', '1 таблетка', 90, now(), '09:00', 'Дать после еды', true
from dogs d
join households h on h.id = d.household_id
where h.name = 'Семья';
```

Поле `cycle_days` задает разные циклы приема для разных таблеток. Прививки используют ту же модель с `kind = 'vaccine'`, обычно с более длинным циклом.

## Команды Telegram

По умолчанию Telegram-клиент ходит через кастомный Bot API server:
`TELEGRAM_API_SERVER_URL=https://tgproxy.advsrvone.pw/`.
Значение можно переопределить или очистить в `.env`, если нужен прямой доступ к Telegram API.

- `/start` привязывает пользователя Telegram, если его ID уже указан в `family_members.telegram_user_id`.
  Если пользователь еще не привязан, бот ответит его Telegram ID.
- `/today` показывает дозы, которые скоро нужно принять.
- `/calendar` возвращает ссылку iCal для Google Calendar / Apple Calendar.
- `/report` отправляет CSV-документ с датами принятых доз.

## Модель синхронизации календаря

MVP использует ссылку подписки iCal. Она работает в Apple Calendar и Google Calendar без OAuth. Telegram остается интерфейсом для действий "принято" и "пропущено".

Позже можно добавить Google OAuth для семьи или отдельных участников и записывать события напрямую в Google Calendar через `corgitrack.services.google_calendar`.
