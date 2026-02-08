use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

use crate::points::models::Points;

pub async fn insert_points(
    db_conn: Arc<PgPool>,
    points: Points,
) -> Result<uuid::Uuid, anyhow::Error> {
    insert_points_with_executor(&*db_conn, &points).await
}

pub async fn update_points(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
    points: i32,
) -> Result<(), anyhow::Error> {
    update_points_with_executor(&*db_conn, user_id, points).await
}

pub async fn get_total_points(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<i32, anyhow::Error> {
    let row: (i32,) = sqlx::query_as("SELECT total_points FROM points WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&*db_conn)
        .await?;

    Ok(row.0)
}

pub async fn insert_points_with_executor<'e, E>(
    executor: E,
    points: &Points,
) -> Result<uuid::Uuid, anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        INSERT INTO points(id, user_id, total_points)
        VALUES($1, $2, $3)
    "#;

    sqlx::query(query)
        .bind(points.id)
        .bind(points.user_id)
        .bind(points.points_to_add)
        .execute(executor)
        .await?;

    Ok(points.id)
}

pub async fn update_points_with_executor<'e, E>(
    executor: E,
    user_id: uuid::Uuid,
    points: i32,
) -> Result<(), anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        UPDATE points
        SET total_points = total_points + $1
        WHERE user_id = $2
    "#;

    sqlx::query(query)
        .bind(points)
        .bind(user_id)
        .execute(executor)
        .await?;

    Ok(())
}
