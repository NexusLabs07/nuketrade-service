#[derive(Debug, Clone)]
pub struct Points {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub points_to_add: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
