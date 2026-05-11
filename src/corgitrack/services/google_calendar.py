"""Опциональная запись событий в Google Calendar.

MVP отдает ссылку подписки iCal. Этот модуль намеренно маленький:
когда понадобится запись в Google Calendar, здесь можно хранить OAuth-токены
семьи или участника и обновлять события доз.
"""

from corgitrack.models import Dose


async def upsert_dose_event(dose: Dose) -> None:
    raise NotImplementedError("Запись через Google OAuth не входит в MVP с iCal-подпиской.")
