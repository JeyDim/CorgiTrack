from collections.abc import AsyncIterator

from sqlalchemy import inspect, text
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase

from corgitrack.config import settings


class Base(DeclarativeBase):
    pass


engine = create_async_engine(settings.database_url, echo=False, pool_pre_ping=True)
SessionLocal = async_sessionmaker(engine, expire_on_commit=False)


async def get_session() -> AsyncIterator[AsyncSession]:
    async with SessionLocal() as session:
        yield session


async def create_tables() -> None:
    import corgitrack.models  # noqa: F401

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
        dose_columns = await conn.run_sync(
            lambda sync_conn: {
                column["name"] for column in inspect(sync_conn).get_columns("doses")
            }
        )
        if "api_key" not in dose_columns:
            await conn.execute(text("ALTER TABLE doses ADD COLUMN api_key VARCHAR(64)"))
        dose_indexes = await conn.run_sync(
            lambda sync_conn: {
                index["name"] for index in inspect(sync_conn).get_indexes("doses")
            }
        )
        if "ix_doses_api_key" not in dose_indexes:
            await conn.execute(text("CREATE UNIQUE INDEX ix_doses_api_key ON doses (api_key)"))
