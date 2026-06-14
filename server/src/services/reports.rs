use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

/// CSV принятых доз семьи (UTF-8 с BOM — как в Python-версии, для Excel).
pub async fn taken_csv_for_household(
    pool: &PgPool,
    household_id: i32,
) -> Result<Vec<u8>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT g.name AS dog_name, t.name AS treatment_name, t.dose_label, \
                d.due_at, d.taken_at, m.display_name AS member_name, d.note \
         FROM doses d \
         JOIN treatments t ON t.id = d.treatment_id \
         JOIN dogs g ON g.id = t.dog_id \
         LEFT JOIN family_members m ON m.id = d.confirmed_by_member_id \
         WHERE g.household_id = $1 AND d.status = 'taken' \
         ORDER BY d.taken_at DESC",
    )
    .bind(household_id)
    .fetch_all(pool)
    .await?;

    // Разделитель ; — чтобы Excel (в т.ч. русская локаль) корректно разносил по столбцам.
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b';')
        .from_writer(Vec::new());
    writer
        .write_record([
            "собака",
            "назначение",
            "доза",
            "плановая дата",
            "принято в",
            "кто подтвердил",
            "заметка",
        ])
        .ok();

    for row in &rows {
        let dog_name: String = row.try_get("dog_name")?;
        let treatment_name: String = row.try_get("treatment_name")?;
        let dose_label: Option<String> = row.try_get("dose_label")?;
        let due_at: DateTime<Utc> = row.try_get("due_at")?;
        let taken_at: Option<DateTime<Utc>> = row.try_get("taken_at")?;
        let member_name: Option<String> = row.try_get("member_name")?;
        let note: Option<String> = row.try_get("note")?;

        writer
            .write_record([
                dog_name,
                treatment_name,
                dose_label.unwrap_or_default(),
                due_at.to_rfc3339(),
                taken_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                member_name.unwrap_or_default(),
                note.unwrap_or_default(),
            ])
            .ok();
    }

    let body = writer.into_inner().unwrap_or_default();
    let mut out = Vec::with_capacity(body.len() + 3);
    out.extend_from_slice(&[0xEF, 0xBB, 0xBF]); // UTF-8 BOM
    out.extend_from_slice(&body);
    Ok(out)
}
