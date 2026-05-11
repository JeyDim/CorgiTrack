from datetime import UTC, datetime, time, timedelta
from zoneinfo import ZoneInfo

from sqlalchemy import Select, and_, select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from corgitrack.models import Dog, Dose, DoseStatus, Household, Treatment
from corgitrack.config import settings


def utcnow() -> datetime:
    return datetime.now(UTC)


def combine_due(day: datetime, reminder_time: time) -> datetime:
    local_zone = ZoneInfo(settings.app_timezone)
    local_due = datetime.combine(day.astimezone(local_zone).date(), reminder_time, tzinfo=local_zone)
    return local_due.astimezone(UTC)


def iter_due_dates(treatment: Treatment, until: datetime) -> list[datetime]:
    dates: list[datetime] = []
    due_at = combine_due(treatment.start_at, treatment.reminder_time)
    while due_at <= until:
        dates.append(due_at)
        due_at += timedelta(days=treatment.cycle_days)
    return dates


async def ensure_future_doses(session: AsyncSession, horizon_days: int = 370) -> int:
    until = utcnow() + timedelta(days=horizon_days)
    treatments = (
        await session.scalars(
            select(Treatment)
            .where(Treatment.active.is_(True))
            .options(selectinload(Treatment.dog))
        )
    ).all()
    created = 0
    for treatment in treatments:
        existing = set(
            await session.scalars(
                select(Dose.due_at).where(
                    Dose.treatment_id == treatment.id,
                    Dose.due_at <= until,
                )
            )
        )
        for due_at in iter_due_dates(treatment, until):
            if due_at not in existing:
                session.add(Dose(treatment_id=treatment.id, due_at=due_at))
                created += 1
    if created:
        await session.commit()
    return created


def due_dose_query() -> Select[tuple[Dose]]:
    return (
        select(Dose)
        .join(Dose.treatment)
        .join(Treatment.dog)
        .join(Dog.household)
        .where(Treatment.active.is_(True))
        .options(
            selectinload(Dose.treatment).selectinload(Treatment.dog).selectinload(Dog.household),
            selectinload(Dose.treatment).selectinload(Treatment.dog),
        )
    )


async def get_household_for_telegram(session: AsyncSession, telegram_user_id: int) -> Household | None:
    from corgitrack.models import FamilyMember

    member = await session.scalar(
        select(FamilyMember)
        .where(FamilyMember.telegram_user_id == telegram_user_id)
        .options(selectinload(FamilyMember.household))
    )
    return member.household if member else None


async def get_due_for_household(
    session: AsyncSession,
    household_id: int,
    lookahead: timedelta = timedelta(days=1),
) -> list[Dose]:
    now = utcnow()
    rows = await session.scalars(
        due_dose_query().where(
            Household.id == household_id,
            Dose.status.in_([DoseStatus.planned, DoseStatus.reminded]),
            Dose.due_at <= now + lookahead,
        )
    )
    return list(rows)


async def mark_taken(
    session: AsyncSession,
    dose_id: int,
    telegram_user_id: int,
    note: str | None = None,
) -> Dose | None:
    from corgitrack.models import FamilyMember

    dose = await session.scalar(
        select(Dose).where(Dose.id == dose_id).options(selectinload(Dose.treatment))
    )
    member = await session.scalar(select(FamilyMember).where(FamilyMember.telegram_user_id == telegram_user_id))
    if not dose or not member:
        return None
    dose.status = DoseStatus.taken
    dose.taken_at = utcnow()
    dose.confirmed_by_member_id = member.id
    dose.note = note
    await session.commit()
    return dose


async def mark_overdue_as_missed(session: AsyncSession, grace_minutes: int) -> list[Dose]:
    cutoff = utcnow() - timedelta(minutes=grace_minutes)
    rows = list(
        await session.scalars(
            due_dose_query().where(
                Dose.status == DoseStatus.reminded,
                Dose.due_at <= cutoff,
            )
        )
    )
    for dose in rows:
        dose.status = DoseStatus.missed
    if rows:
        await session.commit()
    return rows


async def mark_ready_to_remind(session: AsyncSession, lookahead_minutes: int) -> list[Dose]:
    now = utcnow()
    rows = list(
        await session.scalars(
            due_dose_query().where(
                Dose.status == DoseStatus.planned,
                and_(Dose.due_at >= now - timedelta(minutes=5), Dose.due_at <= now + timedelta(minutes=lookahead_minutes)),
            )
        )
    )
    for dose in rows:
        dose.status = DoseStatus.reminded
        dose.reminded_at = now
    if rows:
        await session.commit()
    return rows
