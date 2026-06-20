-- Идемпотентная схема CorgiTrack.
-- Повторяет то, что раньше создавал Python (SQLAlchemy create_all + ручная
-- миграция doses.api_key). Безопасна для повторного запуска и для уже
-- существующей боевой БД на Raspberry Pi.

-- Enum-типы. У PostgreSQL нет CREATE TYPE IF NOT EXISTS, поэтому через DO-блок.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'treatmentkind') THEN
        CREATE TYPE treatmentkind AS ENUM ('pill', 'vaccine');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'dosestatus') THEN
        CREATE TYPE dosestatus AS ENUM ('planned', 'reminded', 'taken', 'missed', 'skipped');
    END IF;
    -- Категория таблетки: «от клещей» / «от гельминтов». Заполняется только
    -- для kind = 'pill'; у прививок остаётся NULL.
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'pillcategory') THEN
        CREATE TYPE pillcategory AS ENUM ('tick', 'worm');
    END IF;
END$$;

CREATE TABLE IF NOT EXISTS households (
    id             SERIAL PRIMARY KEY,
    name           VARCHAR(200) NOT NULL,
    calendar_token VARCHAR(64)  NOT NULL UNIQUE,
    created_at     TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS family_members (
    id               SERIAL PRIMARY KEY,
    household_id     INTEGER     NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    display_name     VARCHAR(120) NOT NULL,
    telegram_user_id BIGINT      UNIQUE,
    notify           BOOLEAN     NOT NULL DEFAULT TRUE,
    -- Порядок обзвона при эскалации: 0 уведомляется первым, затем 1, 2, ...
    escalation_order INTEGER     NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- На случай уже существующей таблицы family_members без колонки порядка эскалации.
ALTER TABLE family_members ADD COLUMN IF NOT EXISTS escalation_order INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS dogs (
    id           SERIAL PRIMARY KEY,
    household_id INTEGER     NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    name         VARCHAR(120) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS treatments (
    id            SERIAL PRIMARY KEY,
    dog_id        INTEGER       NOT NULL REFERENCES dogs(id) ON DELETE CASCADE,
    name          VARCHAR(200)  NOT NULL,
    kind          treatmentkind NOT NULL,
    dose_label    VARCHAR(120),
    cycle_days    INTEGER       NOT NULL,
    start_at      TIMESTAMPTZ   NOT NULL,
    reminder_time TIME          NOT NULL DEFAULT '09:00',
    instructions  TEXT,
    active        BOOLEAN       NOT NULL DEFAULT TRUE,
    -- Текущая ветклиника назначения (для прививок). Снимок этой клиники
    -- копируется в дозу при отметке «принято» — см. doses.clinic.
    clinic        VARCHAR(160),
    -- Категория таблетки: 'tick' (от клещей) / 'worm' (от гельминтов).
    -- NULL для прививок и для старых записей (в UI трактуется как «от гельминтов»).
    category      pillcategory,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS doses (
    id                     SERIAL PRIMARY KEY,
    treatment_id           INTEGER     NOT NULL REFERENCES treatments(id) ON DELETE CASCADE,
    due_at                 TIMESTAMPTZ NOT NULL,
    status                 dosestatus  NOT NULL DEFAULT 'planned',
    api_key                VARCHAR(64),
    reminded_at            TIMESTAMPTZ,
    -- Состояние эскалации: 0 — ещё не уведомляли, 1 — отправлено первое напоминание,
    -- далее каждый шаг увеличивает уровень (повтор первому, затем следующие по списку).
    escalation_level       INTEGER     NOT NULL DEFAULT 0,
    last_escalated_at      TIMESTAMPTZ,
    taken_at               TIMESTAMPTZ,
    confirmed_by_member_id INTEGER     REFERENCES family_members(id),
    note                   TEXT,
    -- Ветклиника на момент приёма (снимок из treatments.clinic при отметке
    -- «принято»). Хранится на дозе, чтобы смена клиники не меняла историю.
    clinic                 VARCHAR(160),
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- На случай уже существующей таблицы doses без колонки api_key (старые БД).
ALTER TABLE doses ADD COLUMN IF NOT EXISTS api_key VARCHAR(64);
-- Колонки эскалации для уже существующих БД.
ALTER TABLE doses ADD COLUMN IF NOT EXISTS escalation_level  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS last_escalated_at TIMESTAMPTZ;
-- Ветклиника: на лечении (текущая) и на дозе (снимок при приёме) для старых БД.
ALTER TABLE treatments ADD COLUMN IF NOT EXISTS clinic VARCHAR(160);
ALTER TABLE doses      ADD COLUMN IF NOT EXISTS clinic VARCHAR(160);
-- Категория таблетки (NULL = не задана; в UI трактуется как «от гельминтов»).
ALTER TABLE treatments ADD COLUMN IF NOT EXISTS category pillcategory;

-- В старых БД (из Python-версии) у created_at не было DEFAULT — приложение
-- проставляло время само. Rust-API вставляет строки без created_at, поэтому
-- идемпотентно проставляем DEFAULT now(), чтобы INSERT не падал на NOT NULL.
ALTER TABLE households     ALTER COLUMN created_at SET DEFAULT now();
ALTER TABLE dogs           ALTER COLUMN created_at SET DEFAULT now();
ALTER TABLE family_members ALTER COLUMN created_at SET DEFAULT now();
ALTER TABLE treatments     ALTER COLUMN created_at SET DEFAULT now();
ALTER TABLE doses          ALTER COLUMN created_at SET DEFAULT now();

CREATE INDEX        IF NOT EXISTS ix_doses_due_at  ON doses (due_at);
CREATE INDEX        IF NOT EXISTS ix_doses_status  ON doses (status);
CREATE UNIQUE INDEX IF NOT EXISTS ix_doses_api_key ON doses (api_key);

-- Глобальные операционные настройки приложения. Одна строка (id = 1):
-- тайминги эскалации, окно напоминаний и период шедулера правятся через API
-- без перезапуска сервиса.
CREATE TABLE IF NOT EXISTS app_settings (
    id                             SMALLINT     PRIMARY KEY DEFAULT 1,
    escalation_first_delay_minutes INTEGER      NOT NULL DEFAULT 30,
    escalation_step_minutes        INTEGER      NOT NULL DEFAULT 5,
    reminder_lookahead_minutes     INTEGER      NOT NULL DEFAULT 30,
    scheduler_tick_seconds         INTEGER      NOT NULL DEFAULT 60,
    updated_at                     TIMESTAMPTZ  NOT NULL DEFAULT now(),
    CONSTRAINT app_settings_singleton CHECK (id = 1)
);

-- Сид строки настроек значениями по умолчанию (идемпотентно).
INSERT INTO app_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING;
