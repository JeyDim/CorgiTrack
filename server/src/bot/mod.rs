use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use chrono_tz::Tz;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, InputFile};
use teloxide::utils::command::BotCommands;
use url::Url;

use crate::models::{AppSettings, DoseDetail, FamilyMember};
use crate::services::reports::taken_csv_for_household;
use crate::services::schedules::{
    advance_escalation, ensure_future_doses, get_due_for_household, get_household_for_telegram,
    get_reminded_doses, mark_missed, mark_ready_to_remind, mark_taken, next_escalation_action,
    ordered_notify_members, EscalationAction,
};
use crate::services::settings as app_settings;
use crate::state::AppState;
use crate::util::timezone;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    Today,
    Calendar,
    Report,
}

/// Запуск бота: диспетчер команд + фоновый шедулер напоминаний.
pub async fn run(state: AppState) -> anyhow::Result<()> {
    let Some(token) = state.settings.telegram_bot_token.clone() else {
        return Ok(());
    };

    let mut bot = Bot::new(token);
    if let Some(api_url) = &state.settings.telegram_api_server_url {
        bot = bot.set_api_url(Url::parse(api_url)?);
    }

    // Фоновый шедулер напоминаний.
    let scheduler_state = state.clone();
    let scheduler_bot = bot.clone();
    tokio::spawn(async move {
        run_scheduler(scheduler_bot, scheduler_state).await;
    });

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: AppState,
) -> ResponseResult<()> {
    if let Err(err) = command_impl(&bot, &msg, &cmd, &state).await {
        tracing::error!(error = %err, "ошибка обработки команды бота");
        let _ = bot
            .send_message(msg.chat.id, "Произошла ошибка, попробуйте позже.")
            .await;
    }
    Ok(())
}

async fn command_impl(
    bot: &Bot,
    msg: &Message,
    cmd: &Command,
    state: &AppState,
) -> anyhow::Result<()> {
    let chat = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);
    let tz = timezone(&state.settings.app_timezone);

    let Some(user_id) = user_id else {
        return Ok(());
    };

    match cmd {
        Command::Start => match get_household_for_telegram(&state.pool, user_id).await? {
            None => {
                bot.send_message(
                        chat,
                        format!(
                            "Пользователь Telegram еще не привязан.\nВаш ID пользователя Telegram: {user_id}"
                        ),
                    )
                    .await?;
            }
            Some(h) => {
                bot.send_message(
                    chat,
                    format!(
                        "Привязано к семье {}. Команды: /today, /calendar, /report.",
                        h.name
                    ),
                )
                .await?;
            }
        },
        Command::Today => {
            let Some(h) = get_household_for_telegram(&state.pool, user_id).await? else {
                bot.send_message(chat, not_linked()).await?;
                return Ok(());
            };
            ensure_future_doses(&state.pool, tz, 370).await?;
            let doses = get_due_for_household(&state.pool, h.id, Duration::days(1)).await?;
            let mut req = bot.send_message(chat, render_due_list(&doses));
            if let Some(kb) = due_keyboard(&doses) {
                req = req.reply_markup(kb);
            }
            req.await?;
        }
        Command::Calendar => {
            let Some(h) = get_household_for_telegram(&state.pool, user_id).await? else {
                bot.send_message(chat, not_linked()).await?;
                return Ok(());
            };
            let url = format!(
                "{}/calendar/{}.ics",
                state.settings.public_base_url.trim_end_matches('/'),
                h.calendar_token
            );
            bot.send_message(chat, format!("Ссылка для подписки на календарь:\n{url}"))
                .await?;
        }
        Command::Report => {
            let Some(h) = get_household_for_telegram(&state.pool, user_id).await? else {
                bot.send_message(chat, not_linked()).await?;
                return Ok(());
            };
            let content = taken_csv_for_household(&state.pool, h.id).await?;
            bot.send_document(
                chat,
                InputFile::memory(content).file_name("corgitrack-prinyatye-dozy.csv"),
            )
            .caption("Отчет по принятым дозам")
            .await?;
        }
    }
    Ok(())
}

async fn handle_callback(bot: Bot, q: CallbackQuery, state: AppState) -> ResponseResult<()> {
    if let Err(err) = callback_impl(&bot, &q, &state).await {
        tracing::error!(error = %err, "ошибка обработки callback бота");
    }
    Ok(())
}

async fn callback_impl(bot: &Bot, q: &CallbackQuery, state: &AppState) -> anyhow::Result<()> {
    let Some(data) = q.data.as_deref() else {
        return Ok(());
    };
    let Some(rest) = data.strip_prefix("taken:") else {
        return Ok(());
    };
    let dose_id: i32 = rest.parse()?;
    let user_id = q.from.id.0 as i64;

    let dose = mark_taken(&state.pool, dose_id, user_id, None).await?;
    if dose.is_none() {
        bot.answer_callback_query(q.id.clone())
            .text("Доза не найдена")
            .show_alert(true)
            .await?;
        return Ok(());
    }

    bot.answer_callback_query(q.id.clone())
        .text("Отмечено как принято")
        .await?;
    if let Some(message) = q.message.as_ref() {
        bot.edit_message_text(
            message.chat().id,
            message.id(),
            "Отмечено как принято. Напоминания для семьи закрыты.",
        )
        .await?;
    }
    Ok(())
}

fn not_linked() -> &'static str {
    "Пользователь Telegram еще не привязан. Отправьте /start, чтобы узнать свой ID."
}

fn render_due_list(doses: &[DoseDetail]) -> String {
    if doses.is_empty() {
        return "В ближайшие 24 часа таблеток и прививок нет.".to_string();
    }
    let mut lines = vec!["Скоро нужно:".to_string()];
    for d in doses {
        lines.push(format!(
            "- {}: {}, {}",
            d.dog_name.as_deref().unwrap_or(""),
            d.name,
            d.dose.due_at.format("%Y-%m-%d %H:%M")
        ));
        if let Some(label) = &d.dose_label {
            lines.push(format!("  Доза: {label}"));
        }
        if let Some(instructions) = &d.instructions {
            lines.push(format!("  {instructions}"));
        }
    }
    lines.join("\n")
}

fn due_keyboard(doses: &[DoseDetail]) -> Option<InlineKeyboardMarkup> {
    if doses.is_empty() {
        return None;
    }
    let rows: Vec<Vec<InlineKeyboardButton>> = doses
        .iter()
        .take(8)
        .map(|d| {
            vec![InlineKeyboardButton::callback(
                format!("Принято: {}", d.name),
                format!("taken:{}", d.dose.id),
            )]
        })
        .collect();
    Some(InlineKeyboardMarkup::new(rows))
}

async fn run_scheduler(bot: Bot, state: AppState) {
    let tz = timezone(&state.settings.app_timezone);
    loop {
        // Операционные настройки читаем из БД каждый тик — правки через API
        // подхватываются без перезапуска сервиса.
        let settings = match app_settings::get(&state.pool).await {
            Ok(s) => s,
            Err(err) => {
                tracing::error!(error = %err, "не удалось прочитать настройки шедулера");
                tokio::time::sleep(StdDuration::from_secs(60)).await;
                continue;
            }
        };
        if let Err(err) = notify_due(&bot, &state, tz, &settings).await {
            tracing::error!(error = %err, "ошибка цикла напоминаний");
        }
        let tick = settings.scheduler_tick_seconds.max(1) as u64;
        tokio::time::sleep(StdDuration::from_secs(tick)).await;
    }
}

async fn notify_due(
    bot: &Bot,
    state: &AppState,
    tz: Tz,
    settings: &AppSettings,
) -> anyhow::Result<()> {
    ensure_future_doses(&state.pool, tz, 370).await?;

    // Шаг 1: новые наступившие дозы — первое напоминание только первому по эскалации.
    let ready =
        mark_ready_to_remind(&state.pool, settings.reminder_lookahead_minutes as i64).await?;
    for d in &ready {
        // Активные дозы всегда привязаны к живому назначению и семье; None не ждём.
        let Some(household_id) = d.household_id else {
            continue;
        };
        let members = ordered_notify_members(&state.pool, household_id).await?;
        if let Some(primary) = members.first() {
            send_member_message(bot, primary, d, "Напоминание").await?;
        }
    }

    // Шаг 2: уже отправленные напоминания — продвигаем эскалацию по таймингу.
    let reminded = get_reminded_doses(&state.pool).await?;
    let now = Utc::now();
    for d in &reminded {
        let Some(household_id) = d.household_id else {
            continue;
        };
        let members = ordered_notify_members(&state.pool, household_id).await?;
        let action = next_escalation_action(
            d.dose.escalation_level,
            d.dose.last_escalated_at,
            members.len(),
            now,
            settings.escalation_first_delay_minutes as i64,
            settings.escalation_step_minutes as i64,
        );
        match action {
            EscalationAction::Wait => {}
            EscalationAction::Notify {
                member_index,
                next_level,
            } => {
                let prefix = if next_level == 2 {
                    "Напоминание (повтор)"
                } else {
                    "Эскалация: предыдущий не ответил"
                };
                send_member_message(bot, &members[member_index], d, prefix).await?;
                advance_escalation(&state.pool, d.dose.id, next_level).await?;
            }
            EscalationAction::Missed => {
                mark_missed(&state.pool, d.dose.id).await?;
                if let Some(primary) = members.first() {
                    let _ = send_member_message(bot, primary, d, "Пропущенная доза").await;
                }
            }
        }
    }
    Ok(())
}

/// Отправить адресное сообщение по одной дозе конкретному члену семьи.
async fn send_member_message(
    bot: &Bot,
    member: &FamilyMember,
    detail: &DoseDetail,
    prefix: &str,
) -> anyhow::Result<()> {
    let Some(tid) = member.telegram_user_id else {
        return Ok(());
    };
    let slice = std::slice::from_ref(detail);
    let text = render_due_list(slice).replace("Скоро нужно:", &format!("{prefix}:"));
    let mut req = bot.send_message(ChatId(tid), text);
    if let Some(kb) = due_keyboard(slice) {
        req = req.reply_markup(kb);
    }
    req.await?;
    Ok(())
}
