import csv
import io

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from corgitrack.models import Dog, Dose, DoseStatus, Treatment


async def taken_csv_for_household(session: AsyncSession, household_id: int) -> bytes:
    rows = (
        await session.scalars(
            select(Dose)
            .join(Dose.treatment)
            .join(Treatment.dog)
            .where(Dog.household_id == household_id, Dose.status == DoseStatus.taken)
            .options(
                selectinload(Dose.treatment).selectinload(Treatment.dog),
                selectinload(Dose.confirmed_by),
            )
            .order_by(Dose.taken_at.desc())
        )
    ).all()
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerow(["собака", "назначение", "доза", "плановая_дата", "принято_в", "кто_подтвердил", "заметка"])
    for dose in rows:
        writer.writerow(
            [
                dose.treatment.dog.name,
                dose.treatment.name,
                dose.treatment.dose_label or "",
                dose.due_at.isoformat(),
                dose.taken_at.isoformat() if dose.taken_at else "",
                dose.confirmed_by.display_name if dose.confirmed_by else "",
                dose.note or "",
            ]
        )
    return output.getvalue().encode("utf-8-sig")
