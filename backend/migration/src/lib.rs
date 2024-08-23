pub use sea_orm_migration::prelude::*;

mod m20231207_000001_create_table;
mod m20240821_000001_create_tag;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231207_000001_create_table::Migration),
            Box::new(m20240821_000001_create_tag::Migration),
        ]
    }
}
