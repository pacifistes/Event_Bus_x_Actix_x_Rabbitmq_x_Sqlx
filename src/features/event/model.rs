use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: Uuid,
    pub message: String,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Event {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;
        let message: String = row.try_get("message")?;
        Ok(Event { id, message })
    }
}

#[derive(Debug, Deserialize)]
pub struct NewEvent {
    pub message: String,
}
