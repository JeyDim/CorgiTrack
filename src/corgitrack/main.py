import asyncio

import uvicorn

from corgitrack.bot import run_bot_polling
from corgitrack.config import settings
from corgitrack.db import create_tables


async def main() -> None:
    await create_tables()
    config = uvicorn.Config("corgitrack.api:app", host="0.0.0.0", port=8000, log_level="info")
    server = uvicorn.Server(config)
    tasks = [asyncio.create_task(server.serve())]
    if settings.telegram_bot_token:
        tasks.append(asyncio.create_task(run_bot_polling()))
    await asyncio.gather(*tasks)


if __name__ == "__main__":
    asyncio.run(main())
