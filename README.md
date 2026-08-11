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

Replace `/absolute/path/coros-workout-mcp-rs` below with this repository's absolute path. The examples use environment variables so credentials do not need to be entered into a chat. Do not commit real COROS credentials to a project configuration file.

### Generic MCP configuration

Clients that use the standard `mcpServers` JSON shape can register this local stdio server as follows:

```json
{
  "mcpServers": {
    "coros-workout": {
      "command": "/absolute/path/coros-workout-mcp-rs/target/release/coros-workout-mcp-rs",
      "args": [],
      "env": {
        "COROS_EMAIL": "you@example.com",
        "COROS_PASSWORD": "your-password",
        "COROS_REGION": "eu"
      }
    }
  }
}
```

If your client inherits environment variables from its parent process, you can omit `env` and set `COROS_EMAIL`, `COROS_PASSWORD`, and `COROS_REGION` in that environment instead.

### Codex

Add the local stdio server to your Codex user configuration:

```bash
codex mcp add coros-workout \
  --env COROS_EMAIL=you@example.com \
  --env COROS_PASSWORD=your-password \
  --env COROS_REGION=eu \
  -- /absolute/path/coros-workout-mcp-rs/target/release/coros-workout-mcp-rs
```

Confirm that Codex can see it:

```bash
codex mcp list
```

Alternatively, omit the `--env` options and use the `authenticate_coros` tool after starting Codex. See the [Codex MCP command reference](https://developers.openai.com/) for current CLI behavior.

### Claude Code

Add the server for your user account (available across your local projects):

```bash
claude mcp add coros-workout --scope user \
  --env COROS_EMAIL=you@example.com \
  --env COROS_PASSWORD=your-password \
  --env COROS_REGION=eu \
  -- /absolute/path/coros-workout-mcp-rs/target/release/coros-workout-mcp-rs
```

Verify the connection with:

```bash
claude mcp list
```

Use `--scope project` instead of `--scope user` when the server definition should be shared through the project's `.mcp.json`; keep credentials out of that file. Claude's [MCP documentation](https://docs.anthropic.com/en/docs/claude-code/mcp) has the current scope and configuration details.

### Claude Desktop

Add this entry to the `mcpServers` object in Claude Desktop's configuration file. Use the full absolute executable path.

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

### OpenCode

Add the following local server to your global OpenCode configuration (`~/.config/opencode/opencode.json`) or to a project `opencode.json` file:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "coros-workout": {
        "type": "local",
        "command": [
          "/absolute/path/coros-workout-mcp-rs/target/release/coros-workout-mcp-rs"
        ],
        "environment": {
          "COROS_EMAIL": "{env:COROS_EMAIL}",
          "COROS_PASSWORD": "{env:COROS_PASSWORD}",
          "COROS_REGION": "{env:COROS_REGION}"
        }
      }
    }
  }
}
```

Set the three `COROS_*` variables in the environment that launches OpenCode, then confirm the connection:

```bash
opencode mcp list
```

OpenCode's [MCP-server documentation](https://opencode.ai/v2/docs/mcp-servers) describes the current `mcp.servers` configuration format and available local-server options.

## Authentication and safety

Authentication is stored at `~/.config/coros-workout-mcp/auth.json` with owner-only permissions on Unix. Logging in via this unsupported consumer API can invalidate an active COROS web session.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```
