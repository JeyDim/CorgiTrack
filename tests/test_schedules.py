from datetime import UTC, datetime, time

from corgitrack.config import settings
from corgitrack.models import Dose, DoseStatus, Treatment, TreatmentKind
from corgitrack.services.calendar import event_description, mark_taken_url
from corgitrack.services.schedules import combine_due, iter_due_dates


def test_combine_due_uses_configured_local_timezone() -> None:
    due_at = combine_due(datetime(2026, 5, 11, tzinfo=UTC), time(9, 0))

    assert due_at == datetime(2026, 5, 11, 5, 0, tzinfo=UTC)


def test_iter_due_dates_respects_cycle_days() -> None:
    treatment = Treatment(
        dog_id=1,
        name="Test pill",
        kind=TreatmentKind.pill,
        cycle_days=3,
        start_at=datetime(2026, 5, 1, tzinfo=UTC),
        reminder_time=time(9, 0),
        active=True,
    )

    dates = iter_due_dates(treatment, datetime(2026, 5, 8, 23, 59, tzinfo=UTC))

    assert dates == [
        datetime(2026, 5, 1, 5, 0, tzinfo=UTC),
        datetime(2026, 5, 4, 5, 0, tzinfo=UTC),
        datetime(2026, 5, 7, 5, 0, tzinfo=UTC),
    ]


def test_calendar_description_contains_api_mark_link(monkeypatch) -> None:
    monkeypatch.setattr(settings, "public_base_url", "https://corgi.example")
    treatment = Treatment(
        dog_id=1,
        name="Test pill",
        kind=TreatmentKind.pill,
        cycle_days=3,
        start_at=datetime(2026, 5, 1, tzinfo=UTC),
        reminder_time=time(9, 0),
        active=True,
    )
    dose = Dose(
        id=42,
        treatment=treatment,
        due_at=datetime(2026, 5, 1, 5, 0, tzinfo=UTC),
        status=DoseStatus.planned,
        api_key="secret-key-123456789",
    )

    url = mark_taken_url(dose)
    description = event_description(dose)

    assert url == "https://corgi.example/api/doses/42/taken?key=secret-key-123456789"
    assert f"Отметить прием: {url}" in description
