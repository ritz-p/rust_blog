# Running Migrator CLI

Run these commands from the repository root after starting `web`:

```sh
docker compose up -d web
```

The container's `DATABASE_URL` points to the same `blog.db` used by the server and seed. Keep the working directory at the repository root so relative database paths remain consistent.

Generate a migration with the development image's SeaORM CLI, replacing `add_example_table` with the migration name:

```sh
docker compose exec web sea-orm-cli migrate generate add_example_table
```

Edit the generated file under `migration/src/`, then use the migration binary:

| Operation | Command |
| --- | --- |
| Apply all pending migrations | `docker compose exec web cargo run -p migration -- up` |
| Apply the first 10 pending migrations | `docker compose exec web cargo run -p migration -- up -n 10` |
| Roll back the last migration | `docker compose exec web cargo run -p migration -- down` |
| Roll back the last 10 migrations | `docker compose exec web cargo run -p migration -- down -n 10` |
| Drop all tables, then reapply migrations | `docker compose exec web cargo run -p migration -- fresh` |
| Roll back all migrations, then reapply them | `docker compose exec web cargo run -p migration -- refresh` |
| Roll back all migrations | `docker compose exec web cargo run -p migration -- reset` |
| Check migration status | `docker compose exec web cargo run -p migration -- status` |

`fresh`, `refresh` and `reset` remove existing data. Entity regeneration is covered in the [development guide](../README.md#開発).
