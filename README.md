# coros-workout-mcp-rs

> **Fork attribution:** This project is an independent Rust port of [rowlando/coros-workout-mcp](https://github.com/rowlando/coros-workout-mcp), the original TypeScript MCP created and maintained by [@rowlando](https://github.com/rowlando). It preserves the original project's purpose while reimplementing it in Rust. It is not affiliated with COROS or endorsed by the original author.

Unofficial Rust implementation of the COROS strength-workout MCP server. It uses the same reverse-engineered COROS Training Hub consumer API as the original project; COROS may change it without notice.

## Included tools

`authenticate_coros`, `check_coros_auth`, `search_exercises`, `create_workout`, `update_exercises`, `list_workouts`, `list_training_plans`, `get_training_plan`, `create_training_plan`, `list_training_calendar`, `schedule_workout`, `remove_scheduled_workout`, `list_custom_exercises`, and `create_custom_exercise`.

Calendar, plan, and custom-exercise write tools default to `dryRun: true`. A live calendar deletion additionally requires `confirm: true`.

## Build

```bash
cargo build --release
```

The executable is at `target/release/coros-workout-mcp-rs`. The bundled exercise catalog is in `data/exercises.json`. Set `COROS_CATALOG_PATH` to use a catalog elsewhere.

## Configure an MCP client

```json
{
  "mcpServers": {
    "coros-workout": {
      "command": "/absolute/path/coros-workout-mcp-rs/target/release/coros-workout-mcp-rs",
      "env": {
        "COROS_EMAIL": "you@example.com",
        "COROS_PASSWORD": "your-password",
        "COROS_REGION": "eu"
      }
    }
  }
}
```

Authentication is stored at `~/.config/coros-workout-mcp/auth.json` with owner-only permissions on Unix. Logging in via this unsupported consumer API can invalidate an active COROS web session.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```
