# Hiveway

Hive's Rust backend uses Axum, Tokio, **SeaORM 2** and PostgreSQL. Shared connections live in `src/state.rs`; database entities live in `src/entities`. Public API models are separate so database secrets are never serialized accidentally.

Use SeaORM entity builders for ordinary reads and writes. Row locks, conditional budget updates, and PostgreSQL-specific operations use bound statements through SeaORM inside database transactions. SQLx is only an internal dependency of SeaORM.

Migrations run at startup through `sea-orm-migration`. Their SQL lives in `migrations/`; register new migrations in `src/database.rs`. Keep applied files immutable and add forward migrations. Existing SQLx migration records are adopted only when their success flag and SHA-384 checksum match. Automatic destructive rollback is disabled to protect payment records.

Start local infrastructure and both apps with `bun run dev` from the repository root. With PostgreSQL running, verify the database layer in isolated temporary schemas:

```sh
HIVE_TEST_DATABASE_URL=postgres://hive:hive-local-only@127.0.0.1:5432/hive \
  devenv shell -- cargo test --package hiveway --test postgres -- --ignored
```

The database tests check fresh and repeated migrations, legacy migration adoption, concurrent budget reservations, and rollback. They remove only their own temporary schemas.
