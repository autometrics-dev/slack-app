use sqlx::migrate::{Migrate, MigrateError};
use sqlx::Acquire;
use std::ops::Deref;

pub async fn run_sqlx_migrations<'a, A>(migrator: A) -> Result<(), MigrateError>
where
    A: Acquire<'a>,
    <A::Connection as Deref>::Target: Migrate,
{
    sqlx::migrate!("./migrations").run(migrator).await
}
