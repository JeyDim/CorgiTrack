from datetime import UTC, datetime, time

from corgitrack.models import Treatment, TreatmentKind
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
