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
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- На случай уже существующей таблицы doses без колонки api_key (старые БД).
ALTER TABLE doses ADD COLUMN IF NOT EXISTS api_key VARCHAR(64);
-- Колонки эскалации для уже существующих БД.
ALTER TABLE doses ADD COLUMN IF NOT EXISTS escalation_level  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS last_escalated_at TIMESTAMPTZ;

CREATE INDEX        IF NOT EXISTS ix_doses_due_at  ON doses (due_at);
CREATE INDEX        IF NOT EXISTS ix_doses_status  ON doses (status);
CREATE UNIQUE INDEX IF NOT EXISTS ix_doses_api_key ON doses (api_key);
