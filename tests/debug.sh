#!/bin/bash
# Debug helper for clawparty tests
# Creates the special '0#Agent' record in the agents table

set -euo pipefail

DB_PATH="${CLAWPARTY_DB:-$HOME/.clawparty/ztm.db}"

# Ensure the database file exists
if [[ ! -f "$DB_PATH" ]]; then
  echo "[ERROR] Database not found at $DB_PATH"
  exit 1
fi

# Insert or replace the system agent record
sqlite3 "$DB_PATH" <<'SQL'
INSERT INTO agents (
    agent_name, display_name, directory, config_path,
    workspace_dir, port, pid, status, created_at, updated_at, config_json, error_msg
) VALUES (
    '0#Agent', '0#Agent',
    '/Users/caishu/.clawparty/.zeroclaw',
    '/Users/caishu/.clawparty/.zeroclaw/config.toml',
    '/Users/caishu/.clawparty/.zeroclaw/workspace',
    42617, NULL, 'running',
    strftime('%s','now'), strftime('%s','now'),
    NULL, NULL
)
ON CONFLICT(agent_name) DO UPDATE SET
    display_name   = excluded.display_name,
    directory      = excluded.directory,
    config_path    = excluded.config_path,
    workspace_dir  = excluded.workspace_dir,
    port           = excluded.port,
    status         = excluded.status,
    updated_at     = strftime('%s','now');
SQL

echo "[OK] 0#Agent record ensured in $DB_PATH"
