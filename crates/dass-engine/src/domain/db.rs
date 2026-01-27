use crate::domain::application::SoftwareApplication;
use crate::domain::primitive::Primitive;
use anyhow::Result;
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePool},
};
use std::path::Path;

pub struct StateStore {
    pool: Pool<Sqlite>,
}

impl StateStore {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS applications (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                work_dir TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS primitives (
                id TEXT NOT NULL,
                app_id TEXT NOT NULL,
                type TEXT NOT NULL,
                payload BLOB NOT NULL,
                version INTEGER DEFAULT 1,
                PRIMARY KEY (id, app_id),
                FOREIGN KEY (app_id) REFERENCES applications(id)
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_application(&self, app: &SoftwareApplication) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO applications (id, name, work_dir) VALUES (?1, ?2, ?3)")
            .bind(&app.id)
            .bind(&app.name)
            .bind(&app.work_dir)
            .execute(&self.pool)
            .await?;

        for primitive in app.primitives.values() {
            let payload = serde_json::to_vec(primitive)?;
            sqlx::query(
                "INSERT OR REPLACE INTO primitives (id, app_id, type, payload) VALUES (?1, ?2, ?3, ?4)"
            )
            .bind(primitive.id())
            .bind(&app.id)
            .bind(primitive.type_name())
            .bind(payload)
            .execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn load_application(&self, id: &str) -> Result<SoftwareApplication> {
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT name, work_dir FROM applications WHERE id = ?1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        let (name, work_dir) = row;
        let mut app = SoftwareApplication::new(id.to_string(), name);
        app.work_dir = work_dir;

        let rows =
            sqlx::query_as::<_, (Vec<u8>,)>("SELECT payload FROM primitives WHERE app_id = ?1")
                .bind(id)
                .fetch_all(&self.pool)
                .await?;

        for row in rows {
            let primitive: Primitive = serde_json::from_slice(&row.0)?;
            app.add_primitive(primitive);
        }

        Ok(app)
    }

    pub async fn list_applications(&self) -> Result<Vec<(String, String)>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, name FROM applications ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }
}
