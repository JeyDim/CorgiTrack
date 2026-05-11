import asyncio
from datetime import timedelta

from aiogram import Bot, Dispatcher, F
from aiogram.filters import Command
from aiogram.types import BufferedInputFile, CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message
from sqlalchemy import select
from sqlalchemy.orm import selectinload

from corgitrack.config import settings
from corgitrack.db import SessionLocal
from corgitrack.models import Dog, Dose, FamilyMember, Household, Treatment
from corgitrack.services.reports import taken_csv_for_household
from corgitrack.services.schedules import (
    ensure_future_doses,
    get_due_for_household,
    get_household_for_telegram,
    mark_overdue_as_missed,
    mark_ready_to_remind,
    mark_taken,
)


def build_dispatcher() -> Dispatcher:
    dp = Dispatcher()

    @dp.message(Command("start"))
    async def start(message: Message) -> None:
        async with SessionLocal() as session:
            household = await get_household_for_telegram(session, message.from_user.id)
        if not household:
            await message.answer(
                "Пользователь Telegram еще не привязан.\n"
                f"Ваш ID пользователя Telegram: {message.from_user.id}\n\n"
                "Добавьте это значение в family_members.telegram_user_id."
            )
            return
        await message.answer(f"Привязано к семье {household.name}. Команды: /today, /calendar, /report.")

    @dp.message(Command("today"))
    async def today(message: Message) -> None:
        async with SessionLocal() as session:
            household = await get_household_for_telegram(session, message.from_user.id)
            if not household:
                await message.answer("Пользователь Telegram еще не привязан. Отправьте /start, чтобы узнать свой ID.")
                return
            await ensure_future_doses(session)
            doses = await get_due_for_household(session, household.id, timedelta(days=1))
            await message.answer(render_due_list(doses), reply_markup=due_keyboard(doses))

    @dp.message(Command("calendar"))
    async def calendar(message: Message) -> None:
        async with SessionLocal() as session:
            household = await get_household_for_telegram(session, message.from_user.id)
        if not household:
            await message.answer("Пользователь Telegram еще не привязан. Отправьте /start, чтобы узнать свой ID.")
            return
        url = f"{settings.public_base_url}/calendar/{household.calendar_token}.ics"
        await message.answer(f"Ссылка для подписки на календарь:\n{url}")

    @dp.message(Command("report"))
    async def report(message: Message) -> None:
        async with SessionLocal() as session:
            household = await get_household_for_telegram(session, message.from_user.id)
            if not household:
                await message.answer("Пользователь Telegram еще не привязан. Отправьте /start, чтобы узнать свой ID.")
                return
            content = await taken_csv_for_household(session, household.id)
        await message.answer_document(
            BufferedInputFile(content, filename="corgitrack-prinyatye-dozy.csv"),
            caption="Отчет по принятым дозам",
        )

    @dp.callback_query(F.data.startswith("taken:"))
    async def taken(callback: CallbackQuery) -> None:
        dose_id = int(callback.data.split(":", 1)[1])
        async with SessionLocal() as session:
            dose = await mark_taken(session, dose_id, callback.from_user.id)
        if not dose:
            await callback.answer("Доза не найдена", show_alert=True)
            return
        await callback.answer("Отмечено как принято")
        await callback.message.edit_text("Отмечено как принято. Напоминания для семьи закрыты.")

    return dp


def render_due_list(doses: list[Dose]) -> str:
    if not doses:
        return "В ближайшие 24 часа таблеток и прививок нет."
    lines = ["Скоро нужно:"]
    for dose in doses:
        treatment = dose.treatment
        lines.append(
            f"- {treatment.dog.name}: {treatment.name}, {dose.due_at:%Y-%m-%d %H:%M}"
        )
        if treatment.dose_label:
            lines.append(f"  Доза: {treatment.dose_label}")
        if treatment.instructions:
            lines.append(f"  {treatment.instructions}")
    return "\n".join(lines)


def due_keyboard(doses: list[Dose]) -> InlineKeyboardMarkup | None:
    if not doses:
        return None
    rows = [
        [InlineKeyboardButton(text=f"Принято: {dose.treatment.name}", callback_data=f"taken:{dose.id}")]
        for dose in doses[:8]
    ]
    return InlineKeyboardMarkup(inline_keyboard=rows)


async def notify_due(bot: Bot) -> None:
    async with SessionLocal() as session:
        await ensure_future_doses(session)
        doses = await mark_ready_to_remind(session, settings.reminder_lookahead_minutes)
        for dose in doses:
            await send_family_message(session, bot, dose, "Напоминание")
        missed = await mark_overdue_as_missed(session, settings.missed_grace_minutes)
        for dose in missed:
            await send_family_message(session, bot, dose, "Пропущенная доза")


async def send_family_message(session, bot: Bot, dose: Dose, prefix: str) -> None:
    dose = await session.scalar(
        select(Dose)
        .where(Dose.id == dose.id)
        .options(selectinload(Dose.treatment).selectinload(Treatment.dog).selectinload(Dog.household))
    )
    members = (
        await session.scalars(
            select(FamilyMember).where(
                FamilyMember.household_id == dose.treatment.dog.household_id,
                FamilyMember.notify.is_(True),
                FamilyMember.telegram_user_id.is_not(None),
            )
        )
    ).all()
    text = render_due_list([dose]).replace("Скоро нужно:", prefix + ":")
    keyboard = due_keyboard([dose])
    for member in members:
        await bot.send_message(member.telegram_user_id, text, reply_markup=keyboard)


async def run_scheduler(bot: Bot) -> None:
    while True:
        try:
            await notify_due(bot)
        except Exception:
            pass
        await asyncio.sleep(settings.scheduler_tick_seconds)


async def run_bot_polling() -> None:
    if not settings.telegram_bot_token:
        return
    bot = Bot(settings.telegram_bot_token)
    dp = build_dispatcher()
    asyncio.create_task(run_scheduler(bot))
    await dp.start_polling(bot)
