use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "UPDATE mcp_activity a SET state='interrupted', error_code='execution_outcome_unknown',
                completed_at=EXTRACT(EPOCH FROM clock_timestamp())::bigint
                WHERE state='accepted' AND task_id IS NOT NULL
                  AND NOT EXISTS (SELECT 1 FROM mcp_jobs j WHERE j.id=a.task_id);
            CREATE INDEX mcp_activity_accepted_task ON mcp_activity(task_id) WHERE state='accepted';
            CREATE FUNCTION finalize_removed_mcp_job() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE
                finished_at bigint;
                outcome text;
                outcome_code text;
            BEGIN
                IF OLD.status IN ('completed','failed','cancelled') THEN
                    finished_at := OLD.updated_at;
                    IF OLD.status='completed' AND COALESCE(OLD.result->>'isError','false')<>'true' THEN
                        outcome := 'succeeded';
                        outcome_code := NULL;
                    ELSE
                        outcome := 'failed';
                        outcome_code := 'connected_tool_failed';
                    END IF;
                ELSE
                    finished_at := EXTRACT(EPOCH FROM clock_timestamp())::bigint;
                    outcome := 'interrupted';
                    outcome_code := 'execution_outcome_unknown';
                END IF;
                UPDATE mcp_activity SET state=outcome, error_code=outcome_code,
                    completed_at=finished_at, execution_ms=GREATEST(0,finished_at-created_at)*1000
                    WHERE state='accepted' AND task_id=OLD.id AND owner=OLD.caller;
                RETURN OLD;
            END $$;
            CREATE TRIGGER finalize_removed_mcp_job BEFORE DELETE ON mcp_jobs
                FOR EACH ROW EXECUTE FUNCTION finalize_removed_mcp_job();"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TRIGGER finalize_removed_mcp_job ON mcp_jobs;
             DROP FUNCTION finalize_removed_mcp_job();
             DROP INDEX mcp_activity_accepted_task;",
            )
            .await?;
        Ok(())
    }
}
