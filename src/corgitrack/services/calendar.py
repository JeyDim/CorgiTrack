from datetime import timedelta

from icalendar import Calendar, Event
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from corgitrack.config import settings
from corgitrack.models import Dog, Dose, Household, Treatment, generate_api_key
from corgitrack.services.schedules import ensure_future_doses, utcnow


async def build_ical(session: AsyncSession, token: str) -> bytes | None:
    household = await session.scalar(select(Household).where(Household.calendar_token == token))
    if not household:
        return None

    await ensure_future_doses(session)
    doses = (
        await session.scalars(
            select(Dose)
            .join(Dose.treatment)
            .join(Treatment.dog)
            .where(Dog.household_id == household.id)
            .where(Dose.due_at >= utcnow() - timedelta(days=30))
            .options(selectinload(Dose.treatment).selectinload(Treatment.dog))
            .order_by(Dose.due_at)
        )
    ).all()
    if ensure_api_keys(doses):
        await session.commit()

    cal = Calendar()
    cal.add("prodid", "-//CorgiTrack//Kalendar Lecheniya//RU")
    cal.add("version", "2.0")
    cal.add("x-wr-calname", f"{household.name}: уход за собакой")

    for dose in doses:
        treatment = dose.treatment
        event = Event()
        event.add("uid", f"dose-{dose.id}@corgitrack")
        event.add("summary", f"{treatment.dog.name}: {treatment.name}")
        event.add("dtstart", dose.due_at)
        event.add("dtend", dose.due_at + timedelta(minutes=15))
        event.add("description", event_description(dose))
        event.add("url", mark_taken_url(dose))
        event.add("status", "CONFIRMED")
        cal.add_component(event)
    return cal.to_ical()


def ensure_api_keys(doses: list[Dose]) -> bool:
    updated = False
    for dose in doses:
        if not dose.api_key:
            dose.api_key = generate_api_key()
            updated = True
    return updated


def event_description(dose: Dose) -> str:
    treatment = dose.treatment
    parts = [
        f"Статус: {status_label(dose.status.value)}",
        f"Цикл: каждые {treatment.cycle_days} дн.",
    ]
    if treatment.dose_label:
        parts.append(f"Доза: {treatment.dose_label}")
    if treatment.instructions:
        parts.append(treatment.instructions)
    parts.append(f"Отметить прием: {mark_taken_url(dose)}")
    parts.append("Также можно подтвердить прием в Telegram-боте.")
    return "\n".join(parts)


def mark_taken_url(dose: Dose) -> str:
    return f"{settings.public_base_url}/api/doses/{dose.id}/taken?key={dose.api_key}"


def status_label(status: str) -> str:
    return {
        "planned": "запланировано",
        "reminded": "напоминание отправлено",
        "taken": "принято",
        "missed": "пропущено",
        "skipped": "пропущено вручную",
    }.get(status, status)
