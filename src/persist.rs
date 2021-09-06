use tokio_postgres::NoTls;

pub struct Persistor {
    client: tokio_postgres::Client,
    base_block: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistorError {
    #[error("persistor error occurred from postgres: {0}")]
    Postgres(#[from] tokio_postgres::Error),
}

type Result<T, E = PersistorError> = std::result::Result<T, E>;

impl Persistor {
    pub async fn new(db: &str, base_block: u64) -> Result<Self> {
        let (client, conn) = tokio_postgres::connect(db, NoTls).await?;
        conn.await?;
        Ok(Self { client, base_block })
    }

    pub async fn get_block_number(&self) -> Result<u64> {
        self.client
            .query_opt(
                "select block_number from block_log order by created_at desc limit 1",
                &[],
            )
            .await
            .map(|row| {
                row.map(|row| row.get::<_, i64>("block_number") as u64)
                    .unwrap_or(self.base_block)
            })
            .map_err(|e| e.into())
    }

    pub async fn save_block_number(&self, block_number: u64) -> Result<()> {
        let rows = self
            .client
            .execute(
                "insert into block_log (block_id) values ($1)",
                &[&(block_number as i64)],
            )
            .await?;
        assert_eq!(rows, 1);
        Ok(())
    }
}
