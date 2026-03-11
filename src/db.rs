use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use crate::models::Item;

pub async fn fetch_recent_items(pool: &PgPool, nb: i64) -> Result<Vec<Item>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT time, nb_people, source FROM line ORDER BY time DESC LIMIT $1",
    )
    .bind(nb)
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|(time, nb_people, source)| Item {
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
            nb_people,
            source,
        })
        .collect();

    Ok(items)
}

pub async fn fetch_day_items(
    pool: &PgPool,
    date: &str,
) -> Result<Vec<Item>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (OffsetDateTime, i32, Option<String>)>(
        "SELECT time, nb_people, source FROM line WHERE time >= $1::date AND time < ($1::date + INTERVAL '1 day') ORDER BY time DESC",
    )
    .bind(date)
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|(time, nb_people, source)| Item {
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
            nb_people,
            source,
        })
        .collect();

    Ok(items)
}

pub async fn create_item(
    pool: &PgPool,
    nb_people: i32,
    source: Option<String>,
) -> Result<Item, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO line (time, nb_people, source) VALUES (NOW(), $1, $2) RETURNING time, nb_people, source",
    )
    .bind(nb_people)
    .bind(source)
    .fetch_one(pool)
    .await?;

    let time: OffsetDateTime = row.get("time");
    let nb_people: i32 = row.get("nb_people");
    let source: Option<String> = row.get("source");

    Ok(Item {
        time: format!("{:02}:{:02}", time.hour(), time.minute()),
        nb_people,
        source,
    })
}

pub async fn test_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    let _ = sqlx::query("
    INSERT INTO line (time, nb_people, source)
    SELECT 
      date_trunc('day', NOW() - INTERVAL '11 hour 20 minute') - INTERVAL '1 day' * ((s - 1) / 100) +
      INTERVAL '11 hour 20 minute' + 
      INTERVAL '1 minute' * ((s - 1) % 160),
      (random() * 100)::int,
      CASE WHEN random() < 0.5 THEN 'wifi' ELSE 'photo' END
    FROM generate_series(1, 16000) s;
    ")
    .execute(pool)
    .await?;
    Ok(())
}