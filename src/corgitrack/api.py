from fastapi import Depends, FastAPI, HTTPException, Response
from sqlalchemy.ext.asyncio import AsyncSession

from corgitrack.db import create_tables, get_session
from corgitrack.services.calendar import build_ical


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
