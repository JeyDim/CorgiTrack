use chrono::{NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use corgitrack::models::{Dose, DoseDetail, DoseStatus, Treatment, TreatmentKind};
use corgitrack::services::calendar::{event_description, mark_taken_url};
use corgitrack::services::schedules::{
    combine_due, iter_due_dates, next_escalation_action, EscalationAction,
};

fn astrakhan() -> Tz {
    "Europe/Astrakhan".parse().unwrap()
}

fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
}

#[test]
fn combine_due_uses_configured_local_timezone() {
    let due = combine_due(
        astrakhan(),
        at(2026, 5, 11, 0, 0),
        NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
    );
    assert_eq!(due, at(2026, 5, 11, 5, 0));
}

#[test]
fn iter_due_dates_respects_cycle_days() {
    let dates = iter_due_dates(
        astrakhan(),
        at(2026, 5, 1, 0, 0),
        NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        3,
        at(2026, 5, 8, 23, 59),
    );
    assert_eq!(
        dates,
        vec![
            at(2026, 5, 1, 5, 0),
            at(2026, 5, 4, 5, 0),
            at(2026, 5, 7, 5, 0)
        ]
    );
}

#[test]
fn calendar_description_contains_api_mark_link() {
    let base = "https://corgi.example";
    let detail = sample_detail();

    let url = mark_taken_url(base, 42, "secret-key-123456789");
    let description = event_description(base, &detail);

    assert_eq!(
        url,
        "https://corgi.example/api/doses/42/taken?key=secret-key-123456789"
    );
    assert!(description.contains(&format!("Отметить прием: {url}")));
}

#[test]
fn escalation_waits_until_first_delay_then_reasks_primary() {
    let t0 = at(2026, 5, 1, 9, 0);
    // level 1 = первое напоминание отправлено №0; до 30 минут — ждём.
    assert_eq!(
        next_escalation_action(1, Some(t0), 2, at(2026, 5, 1, 9, 29), 30, 5),
        EscalationAction::Wait
    );
    // ровно 30 минут — повтор тому же участнику №0.
    assert_eq!(
        next_escalation_action(1, Some(t0), 2, at(2026, 5, 1, 9, 30), 30, 5),
        EscalationAction::Notify {
            member_index: 0,
            next_level: 2
        }
    );
}

#[test]
fn escalation_steps_to_next_member_then_missed() {
    let t1 = at(2026, 5, 1, 9, 30); // момент повторного вопроса №0
                                    // через 5 минут после повтора — следующий по списку (№1, жена).
    assert_eq!(
        next_escalation_action(2, Some(t1), 2, at(2026, 5, 1, 9, 35), 30, 5),
        EscalationAction::Notify {
            member_index: 1,
            next_level: 3
        }
    );
    // участники кончились (их всего 2) — доза пропущена.
    let t2 = at(2026, 5, 1, 9, 35);
    assert_eq!(
        next_escalation_action(3, Some(t2), 2, at(2026, 5, 1, 9, 40), 30, 5),
        EscalationAction::Missed
    );
}

#[test]
fn escalation_noop_without_state() {
    let now = at(2026, 5, 1, 9, 40);
    // уровень 0 (ещё не уведомляли) и отсутствие отметки времени — ничего не делаем.
    assert_eq!(
        next_escalation_action(0, Some(now), 2, now, 30, 5),
        EscalationAction::Wait
    );
    assert_eq!(
        next_escalation_action(1, None, 2, now, 30, 5),
        EscalationAction::Wait
    );
}

fn sample_detail() -> DoseDetail {
    let now = at(2026, 5, 1, 5, 0);
    let treatment = Treatment {
        id: 1,
        dog_id: 1,
        name: "Test pill".to_string(),
        kind: TreatmentKind::Pill,
        category: None,
        dose_label: None,
        cycle_days: 3,
        start_at: now,
        reminder_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        instructions: None,
        clinic: None,
        active: true,
        created_at: now,
    };
    let dose = Dose {
        id: 42,
        treatment_id: 1,
        due_at: now,
        status: DoseStatus::Planned,
        api_key: Some("secret-key-123456789".to_string()),
        reminded_at: None,
        escalation_level: 0,
        last_escalated_at: None,
        taken_at: None,
        confirmed_by_member_id: None,
        note: None,
        clinic: None,
        created_at: now,
    };
    DoseDetail {
        dose,
        treatment,
        dog_name: "Корги".to_string(),
        household_id: 1,
    }
}
