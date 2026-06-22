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
    -- При удалении назначения ставим NULL (ON DELETE SET NULL), а не удаляем дозу —
    -- история приёмов остаётся жить за счёт снимка настроек ниже.
    treatment_id           INTEGER     REFERENCES treatments(id) ON DELETE SET NULL,
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
    -- Снимок настроек назначения и собаки на момент отметки «принято» (а также при
    -- удалении назначения). Позволяет позже менять дозу/название/клинику или вовсе
    -- удалить назначение, не затрагивая историю приёмов. NULL до отметки — тогда
    -- значения берутся из живого treatments через COALESCE.
    treatment_name         VARCHAR(200),
    kind                   treatmentkind,
    category               pillcategory,
    dose_label             VARCHAR(120),
    instructions           TEXT,
    cycle_days             INTEGER,
    -- Ветклиника на момент приёма (снимок из treatments.clinic при отметке
    -- «принято»). Хранится на дозе, чтобы смена клиники не меняла историю.
    clinic                 VARCHAR(160),
    -- Снимок собаки/семьи: нужны, чтобы отфильтровать историю приёма по семье и
    -- собаке даже после того, как назначение удалено (treatment_id → NULL).
    dog_name               VARCHAR(120),
    dog_id                 INTEGER,
    household_id           INTEGER,
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

-- Снимок настроек назначения и собаки на дозе (для старых БД). Заполняется при
-- отметке «принято» и при удалении назначения; до этого NULL.
ALTER TABLE doses ADD COLUMN IF NOT EXISTS treatment_name VARCHAR(200);
ALTER TABLE doses ADD COLUMN IF NOT EXISTS kind           treatmentkind;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS category       pillcategory;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS dose_label     VARCHAR(120);
ALTER TABLE doses ADD COLUMN IF NOT EXISTS instructions   TEXT;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS cycle_days     INTEGER;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS dog_name       VARCHAR(120);
ALTER TABLE doses ADD COLUMN IF NOT EXISTS dog_id         INTEGER;
ALTER TABLE doses ADD COLUMN IF NOT EXISTS household_id   INTEGER;

-- Удаление назначения не должно стирать историю приёмов: treatment_id делаем
-- NULLABLE и пересоздаём внешний ключ как ON DELETE SET NULL (для старых БД, где
-- он был NOT NULL + ON DELETE CASCADE). Идемпотентно: если SET NULL-ключ уже есть,
-- блок ничего не делает.
ALTER TABLE doses ALTER COLUMN treatment_id DROP NOT NULL;
DO $$
DECLARE
    fk_name text;
BEGIN
    -- Снимаем любой существующий FK doses.treatment_id -> treatments, у которого
    -- поведение при удалении НЕ «SET NULL» ('n'), чтобы пересоздать как нужно.
    SELECT con.conname INTO fk_name
    FROM pg_constraint con
    JOIN pg_class rel ON rel.oid = con.conrelid
    WHERE rel.relname = 'doses'
      AND con.contype = 'f'
      AND con.confrelid = 'treatments'::regclass
      AND con.confdeltype <> 'n'
    LIMIT 1;
    IF fk_name IS NOT NULL THEN
        EXECUTE format('ALTER TABLE doses DROP CONSTRAINT %I', fk_name);
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint con
        JOIN pg_class rel ON rel.oid = con.conrelid
        WHERE rel.relname = 'doses'
          AND con.contype = 'f'
          AND con.confrelid = 'treatments'::regclass
          AND con.confdeltype = 'n'
    ) THEN
        ALTER TABLE doses
            ADD CONSTRAINT doses_treatment_id_fkey
            FOREIGN KEY (treatment_id) REFERENCES treatments(id) ON DELETE SET NULL;
    END IF;
END$$;

-- Разовый бэкфилл снимка для уже принятых доз (status = 'taken') из живого
-- назначения и собаки. До этой версии снимок при отметке не снимался, поэтому
-- редактирование назначения «задним числом» меняло бы старую историю приёмов.
-- Guard `treatment_name IS NULL` делает шаг идемпотентным (bootstrap гоняется при
-- каждом старте): после первого прогона строки уже заполнены и не переписываются.
-- Будущие (ещё не принятые) дозы намеренно не трогаем — они должны отражать правки
-- назначения вплоть до момента приёма.
UPDATE doses SET
    treatment_name = COALESCE(doses.treatment_name, t.name),
    kind           = COALESCE(doses.kind, t.kind),
    category       = COALESCE(doses.category, t.category),
    dose_label     = COALESCE(doses.dose_label, t.dose_label),
    instructions   = COALESCE(doses.instructions, t.instructions),
    cycle_days     = COALESCE(doses.cycle_days, t.cycle_days),
    clinic         = COALESCE(doses.clinic, t.clinic),
    dog_name       = COALESCE(doses.dog_name, g.name),
    dog_id         = COALESCE(doses.dog_id, g.id),
    household_id   = COALESCE(doses.household_id, g.household_id)
FROM treatments t JOIN dogs g ON g.id = t.dog_id
WHERE doses.treatment_id = t.id
  AND doses.status = 'taken'
  AND doses.treatment_name IS NULL;

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
