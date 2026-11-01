use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    cli::run_cli(oxarr_migration::Migrator).await;
}
