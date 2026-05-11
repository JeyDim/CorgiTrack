from fastapi import Depends, FastAPI, HTTPException, Query, Response
from sqlalchemy.ext.asyncio import AsyncSession

from corgitrack.db import create_tables, get_session
from corgitrack.services.calendar import build_ical
from corgitrack.services.schedules import mark_taken_by_api_key


app = FastAPI(title="API CorgiTrack")


@app.on_event("startup")
async def startup() -> None:
    await create_tables()


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/calendar/{token}.ics")
async def calendar(token: str, session: AsyncSession = Depends(get_session)) -> Response:
    content = await build_ical(session, token)
    if content is None:
        raise HTTPException(status_code=404, detail="Календарь не найден")
    return Response(content=content, media_type="text/calendar; charset=utf-8")


@app.get("/api/doses/{dose_id}/taken")
@app.post("/api/doses/{dose_id}/taken")
async def mark_dose_taken(
    dose_id: int,
    key: str = Query(min_length=16),
    session: AsyncSession = Depends(get_session),
) -> dict[str, str | int]:
    dose = await mark_taken_by_api_key(
        session,
        dose_id,
        key,
        note="Отмечено по ссылке из календаря",
    )
    if not dose:
        raise HTTPException(
            status_code=404,
            detail="Доза не найдена или ключ неверный",
        )
    return {"status": "taken", "dose_id": dose.id}
