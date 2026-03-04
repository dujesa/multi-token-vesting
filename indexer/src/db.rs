use sqlx::PgPool;

pub async fn init_pool(database_url: &str) -> PgPool {
    PgPool::connect(database_url)
        .await
        .expect("failed to connect to database")
}

pub async fn run_migrations(pool: &PgPool) {
    let sql = include_str!("../migrations/001_init.sql");
    sqlx::raw_sql(sql)
        .execute(pool)
        .await
        .expect("failed to run migrations");
    log::info!("migrations applied");
}
