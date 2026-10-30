use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Install trigram support in `public` and add the generated search vector
    /// and catalog indexes, including when the connection uses an isolated schema.
    ///
    /// # Errors
    /// Returns database errors if extension installation or catalog DDL fails.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Extension objects must match the qualified operator classes below even
        // when migrations run with an isolated schema as the search path.
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public")
            .await?;
        db.execute_unprepared(
            "ALTER TABLE capabilities ADD COLUMN IF NOT EXISTS search_vector tsvector \
             GENERATED ALWAYS AS (to_tsvector('english', name || ' ' || description)) STORED",
        )
        .await?;
        // Backfill the generated column for rows written before it existed.
        db.execute_unprepared("UPDATE capabilities SET name = name WHERE search_vector IS NULL")
            .await?;
        for statement in [
            "CREATE INDEX IF NOT EXISTS capability_name_trgm ON capabilities USING gin (name public.gin_trgm_ops)",
            "CREATE INDEX IF NOT EXISTS capability_description_trgm ON capabilities USING gin (description public.gin_trgm_ops)",
            "CREATE INDEX IF NOT EXISTS capability_search_vector ON capabilities USING gin (search_vector)",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
    /// Remove catalog search indexes and the generated vector, retaining the
    /// shared trigram extension for other schemas and applications.
    ///
    /// # Errors
    /// Returns database errors if an index or column cannot be removed.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "DROP INDEX IF EXISTS capability_search_vector",
            "DROP INDEX IF EXISTS capability_description_trgm",
            "DROP INDEX IF EXISTS capability_name_trgm",
        ] {
            db.execute_unprepared(statement).await?;
        }
        db.execute_unprepared("ALTER TABLE capabilities DROP COLUMN IF EXISTS search_vector")
            .await?;
        Ok(())
    }
}
