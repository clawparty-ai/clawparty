use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Transaction};

/// Initialize the clawparty.db (separate from ztm.db to avoid concurrency conflicts).
pub fn init_clawparty_db(data_dir: &str) -> anyhow::Result<()> {
    let db_path = format!("{}/clawparty.db", data_dir);
    let conn = Connection::open(&db_path)?;

    // WAL mode for safe concurrent readers + single writer
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id       TEXT    UNIQUE NOT NULL,
            agent_name    TEXT    NOT NULL,
            group_id      TEXT,
            parent_id     TEXT,
            title         TEXT    NOT NULL,
            short_title   TEXT,
            description   TEXT,
            ai_description TEXT,
            status        TEXT    NOT NULL DEFAULT 'pending',
            progress      INTEGER NOT NULL DEFAULT 0,
            priority      TEXT    NOT NULL DEFAULT 'normal',
            dependencies  TEXT,            -- JSON array
            task_number   INTEGER,
            result_summary TEXT,
            prompt        TEXT,
            is_pipeline   INTEGER NOT NULL DEFAULT 0,
            pipeline_definition TEXT,    -- JSON array
            created_at    REAL    NOT NULL,
            updated_at    REAL    NOT NULL,
            started_at    REAL,
            completed_at  REAL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_agent    ON tasks(agent_name);
        CREATE INDEX IF NOT EXISTS idx_tasks_group    ON tasks(group_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_parent   ON tasks(parent_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_status   ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_tasks_task_id  ON tasks(task_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_number   ON tasks(agent_name, task_number);

        CREATE TABLE IF NOT EXISTS task_events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id     TEXT    NOT NULL,
            event_type  TEXT    NOT NULL,
            from_status TEXT,
            to_status   TEXT,
            progress    INTEGER,
            message     TEXT,
            timestamp   REAL    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_task_events_task      ON task_events(task_id);
        CREATE INDEX IF NOT EXISTS idx_task_events_timestamp ON task_events(timestamp);

        CREATE TABLE IF NOT EXISTS task_analysis_log (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_name        TEXT    NOT NULL,
            group_id          TEXT,
            last_analyzed_at  REAL    NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_tal_agent  ON task_analysis_log(agent_name);
        CREATE INDEX IF NOT EXISTS idx_tal_group  ON task_analysis_log(group_id);

        CREATE TABLE IF NOT EXISTS kanban_configs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_name  TEXT    NOT NULL,
            group_id    TEXT,
            name        TEXT,
            prompt      TEXT,
            config      TEXT,               -- JSON
            updated_at  REAL    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_kanban_agent ON kanban_configs(agent_name);
        CREATE INDEX IF NOT EXISTS idx_kanban_group ON kanban_configs(group_id);

        -- Agents table (migrated from ztm.db)
        CREATE TABLE IF NOT EXISTS agents (
            agent_name      TEXT PRIMARY KEY,
            display_name    TEXT,
            description     TEXT,
            directory       TEXT NOT NULL,
            config_path     TEXT NOT NULL,
            workspace_dir   TEXT NOT NULL,
            port            INTEGER NOT NULL,
            pid             INTEGER,
            status          TEXT NOT NULL DEFAULT 'stopped',
            created_at      REAL    NOT NULL,
            updated_at      REAL    NOT NULL,
            config_json     TEXT,
            error_msg       TEXT,
            deleted         INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
        CREATE INDEX IF NOT EXISTS idx_agents_deleted ON agents(deleted);

        -- Group chats table (migrated from ztm.db)
        CREATE TABLE IF NOT EXISTS group_chats (
            group_id      TEXT PRIMARY KEY,
            group_name    TEXT    NOT NULL,
            owner_agent   TEXT    NOT NULL,
            members       TEXT    NOT NULL,
            created_at    REAL    NOT NULL,
            updated_at    REAL    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_groupchats_owner ON group_chats(owner_agent);

        -- Chat log table (migrated from ztm.db)
        CREATE TABLE IF NOT EXISTS chat_log (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            time        REAL    NOT NULL,
            mesh        TEXT    NOT NULL,
            chat_type   TEXT    NOT NULL,
            chat_id     TEXT    NOT NULL,
            chat_name   TEXT,
            creator     TEXT,
            sender      TEXT    NOT NULL,
            event       TEXT    NOT NULL,
            content     TEXT,
            members     TEXT,
            session_id  TEXT,
            muted       INTEGER NOT NULL DEFAULT 0,
            msg_type    TEXT    NOT NULL DEFAULT 'response'
        );
        CREATE INDEX IF NOT EXISTS idx_chatlog_chat ON chat_log(chat_type, chat_id, time);",


    )?;

    Ok(())
}

fn open_db(data_dir: &str) -> anyhow::Result<Connection> {
    let db_path = format!("{}/clawparty.db", data_dir);
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    Ok(conn)
}

/// A task row as stored in clawparty.db.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub task_id: String,
    pub agent_name: String,
    pub group_id: Option<String>,
    pub parent_id: Option<String>,
    pub title: String,
    pub short_title: Option<String>,
    pub description: Option<String>,
    pub ai_description: Option<String>,
    pub status: String,
    pub progress: i32,
    pub priority: String,
    pub dependencies: Vec<String>,
    pub task_number: Option<i32>,
    pub result_summary: Option<String>,
    pub prompt: Option<String>,
    pub is_pipeline: bool,
    pub pipeline_definition: Vec<serde_json::Value>,
    pub created_at: f64,
    pub updated_at: f64,
    pub started_at: Option<f64>,
    pub completed_at: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subtasks: Vec<Task>,
}

/// Used for creating or updating tasks.
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct TaskUpdate {
    pub task_id: Option<String>,
    pub agent_name: Option<String>,
    pub group_id: Option<String>,
    pub parent_id: Option<String>,
    pub title: Option<String>,
    pub short_title: Option<String>,
    pub description: Option<String>,
    pub ai_description: Option<String>,
    pub status: Option<String>,
    pub progress: Option<i32>,
    pub priority: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub task_number: Option<i32>,
    pub result_summary: Option<String>,
    pub prompt: Option<String>,
    pub is_pipeline: Option<bool>,
    pub pipeline_definition: Option<Vec<serde_json::Value>>,
    pub started_at: Option<f64>,
    pub completed_at: Option<f64>,
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let deps_str: Option<String> = row.get("dependencies")?;
    let deps: Vec<String> = deps_str.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();

    let pipe_str: Option<String> = row.get("pipeline_definition")?;
    let pipe: Vec<serde_json::Value> = pipe_str.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();

    Ok(Task {
        task_id: row.get("task_id")?,
        agent_name: row.get("agent_name")?,
        group_id: row.get("group_id")?,
        parent_id: row.get("parent_id")?,
        title: row.get("title")?,
        short_title: row.get("short_title")?,
        description: row.get("description")?,
        ai_description: row.get("ai_description")?,
        status: row.get("status")?,
        progress: row.get("progress")?,
        priority: row.get("priority")?,
        dependencies: deps,
        task_number: row.get("task_number")?,
        result_summary: row.get("result_summary")?,
        prompt: row.get("prompt")?,
        is_pipeline: row.get::<_, i32>("is_pipeline")? != 0,
        pipeline_definition: pipe,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        started_at: row.get("started_at")?,
        completed_at: row.get("completed_at")?,
        subtasks: Vec::new(),
    })
}

/// Get tasks for an agent, optionally filtered by group_id.
/// Returns a tree (root tasks with nested subtasks).
pub fn get_tasks(data_dir: &str, agent_name: &str, group_id: Option<&str>) -> anyhow::Result<Vec<Task>> {
    let conn = open_db(data_dir)?;

    let mut stmt = conn.prepare(
        "SELECT * FROM tasks WHERE agent_name = ?1 AND (group_id = ?2 OR (?2 IS NULL AND group_id IS NULL))
         ORDER BY created_at ASC",
    )?;

    let gid = group_id.unwrap_or("");
    let rows: Vec<Task> = stmt
        .query_map(rusqlite::params![agent_name, gid], row_to_task)?
        .filter_map(|r| r.ok())
        .collect();

    // Build parent->children map and root list
    let mut task_map: HashMap<String, Task> = HashMap::new();
    let mut roots: Vec<Task> = Vec::new();

    for t in rows {
        let tid = t.task_id.clone();
        if let Some(parent) = &t.parent_id {
            if let Some(parent_task) = task_map.get_mut(parent) {
                parent_task.subtasks.push(t);
                continue;
            }
        }
        roots.push(t);
        task_map.insert(tid, Task::default()); // placeholder, won't be used for tree
    }

    // Re-populate map for subtask lookups
    let mut all: Vec<Task> = roots.clone();
    let mut i = 0;
    while i < all.len() {
        let subs = all[i].subtasks.clone();
        for s in subs {
            all.push(s);
        }
        i += 1;
    }

    Ok(roots)
}

/// Create a new task inside a transaction.
pub fn create_task(tx: &Transaction, task: &TaskUpdate) -> anyhow::Result<Task> {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    let task_id = task.task_id.clone().unwrap_or_else(|| {
        format!("task-{}-{}", t as i64, rand::random::<u16>())
    });

    // Auto-assign task_number per agent (COALESCE handles empty table)
    let next_number: i32 = tx.query_row(
        "SELECT COALESCE(MAX(task_number), 0) + 1 FROM tasks WHERE agent_name = ?1",
        rusqlite::params![task.agent_name.as_deref().unwrap_or("")],
        |row| row.get(0),
    )?;

    let deps = task.dependencies.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default());
    let pipe = task.pipeline_definition.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    tx.execute(
        "INSERT INTO tasks
         (task_id, agent_name, group_id, parent_id, title, short_title, description,
          ai_description, status, progress, priority, dependencies, task_number,
          prompt, is_pipeline, pipeline_definition, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)",
        rusqlite::params![
            &task_id,
            task.agent_name.as_deref(),
            task.group_id.as_deref(),
            task.parent_id.as_deref(),
            task.title.as_deref().unwrap_or("Untitled"),
            task.short_title.as_deref(),
            task.description.as_deref(),
            task.ai_description.as_deref(),
            task.status.as_deref().unwrap_or("pending"),
            task.progress.unwrap_or(0),
            task.priority.as_deref().unwrap_or("normal"),
            deps.as_deref(),
            next_number,
            task.prompt.as_deref(),
            if task.is_pipeline.unwrap_or(false) { 1 } else { 0 },
            pipe.as_deref(),
            t,
        ],
    )?;

    tx.execute(
        "INSERT INTO task_events (task_id, event_type, to_status, progress, message, timestamp)
         VALUES (?1, 'created', ?2, ?3, 'Task created', ?4)",
        rusqlite::params![&task_id, task.status.as_deref().unwrap_or("pending"), task.progress.unwrap_or(0), t],
    )?;

    // Return the created task
    get_task(&tx, &task_id)?.ok_or_else(|| anyhow::anyhow!("Failed to read back created task"))
}

/// Get a single task by ID.
pub fn get_task(conn: &Connection, task_id: &str) -> anyhow::Result<Option<Task>> {
    let mut stmt = conn.prepare("SELECT * FROM tasks WHERE task_id = ?1")?;
    let row = stmt.query_row(rusqlite::params![task_id], row_to_task).optional()?;
    Ok(row)
}

/// Update an existing task.
pub fn update_task(tx: &Transaction, task_id: &str, updates: &TaskUpdate) -> anyhow::Result<Option<Task>> {
    let existing = get_task(tx, task_id)?;
    if existing.is_none() {
        return Ok(None);
    }
    let existing = existing.unwrap();

    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    // Build SQL parameters inline — avoid closure borrow issues
    let mut parts: Vec<(String, Option<String>, Option<i32>, Option<f64>)> = Vec::new();
    macro_rules! p_str {
        ($field:expr, $val:expr) => {
            if $val.is_some() { parts.push((format!("{} = ?", $field), $val.clone(), None, None)); }
        };
    }
    macro_rules! p_i32 {
        ($field:expr, $val:expr) => {
            if $val.is_some() { parts.push((format!("{} = ?", $field), None, $val, None)); }
        };
    }
    macro_rules! p_f64 {
        ($field:expr, $val:expr) => {
            if $val.is_some() { parts.push((format!("{} = ?", $field), None, None, $val)); }
        };
    }

    p_str!("title", updates.title);
    p_str!("short_title", updates.short_title);
    p_str!("description", updates.description);
    p_str!("ai_description", updates.ai_description);
    p_str!("status", updates.status);
    p_i32!("progress", updates.progress);
    p_str!("priority", updates.priority);
    if updates.dependencies.is_some() {
        let s = serde_json::to_string(updates.dependencies.as_ref().unwrap()).unwrap_or_default();
        parts.push(("dependencies = ?".to_string(), Some(s), None, None));
    }
    p_i32!("task_number", updates.task_number);
    p_str!("result_summary", updates.result_summary);
    p_str!("prompt", updates.prompt);
    if let Some(v) = updates.is_pipeline {
        parts.push(("is_pipeline = ?".to_string(), Some((if v { "1" } else { "0" }).to_string()), None, None));
    }
    if updates.pipeline_definition.is_some() {
        let s = serde_json::to_string(updates.pipeline_definition.as_ref().unwrap()).unwrap_or_default();
        parts.push(("pipeline_definition = ?".to_string(), Some(s), None, None));
    }
    p_f64!("started_at", updates.started_at);
    p_f64!("completed_at", updates.completed_at);

    parts.push(("updated_at = ?".to_string(), None, None, Some(t)));

    if parts.is_empty() {
        return get_task(tx, task_id);
    }

    let fields: Vec<String> = parts.iter().map(|(s, _, _, _)| s.clone()).collect();
    let sql = format!("UPDATE tasks SET {} WHERE task_id = ?", fields.join(", "));
    let mut stmt = tx.prepare(&sql)?;
    for (i, (_, sval, ival, fval)) in parts.iter().enumerate() {
        if let Some(ref s) = sval {
            stmt.raw_bind_parameter(i + 1, s.as_str())?;
        } else if let Some(v) = ival {
            stmt.raw_bind_parameter(i + 1, *v)?;
        } else if let Some(v) = fval {
            stmt.raw_bind_parameter(i + 1, *v)?;
        }
    }
    stmt.raw_bind_parameter(parts.len() + 1, task_id)?;
    stmt.raw_execute()?;

    // Record status change event if applicable
    if let Some(new_status) = &updates.status {
        if new_status != &existing.status {
            tx.execute(
                "INSERT INTO task_events (task_id, event_type, from_status, to_status, progress, message, timestamp)
                 VALUES (?1, 'status_changed', ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    task_id,
                    &existing.status,
                    new_status,
                    updates.progress.unwrap_or(existing.progress),
                    "Status changed",
                    t,
                ],
            )?;
        }
    }

    get_task(tx, task_id)
}

/// Delete a task and its subtasks.
pub fn delete_task_cascade(tx: &Transaction, task_id: &str) -> anyhow::Result<bool> {
    // Find children
    let mut stmt = tx.prepare("SELECT task_id FROM tasks WHERE parent_id = ?1")?;
    let children: Vec<String> = stmt
        .query_map(rusqlite::params![task_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for child in children {
        delete_task_cascade(tx, &child)?;
    }

    tx.execute("DELETE FROM task_events WHERE task_id = ?1", rusqlite::params![task_id])?;
    let n = tx.execute("DELETE FROM tasks WHERE task_id = ?1", rusqlite::params![task_id])?;
    Ok(n > 0)
}

/// Get the last analyzed timestamp for an agent + group.
pub fn get_analysis_log(data_dir: &str, agent_name: &str, group_id: Option<&str>) -> anyhow::Result<f64> {
    let conn = open_db(data_dir)?;
    let row: Option<f64> = conn
        .query_row(
            "SELECT last_analyzed_at FROM task_analysis_log
             WHERE agent_name = ?1 AND (group_id = ?2 OR (?2 IS NULL AND group_id IS NULL))",
            rusqlite::params![agent_name, group_id.unwrap_or("")],
            |row| row.get(0),
        )
        .optional()?;
    Ok(row.unwrap_or(0.0))
}

/// Set the last analyzed timestamp inside a transaction.
pub fn set_analysis_log(tx: &Transaction, agent_name: &str, group_id: Option<&str>, timestamp: f64) -> anyhow::Result<()> {
    let existing: bool = tx.query_row(
        "SELECT COUNT(*) > 0 FROM task_analysis_log
         WHERE agent_name = ?1 AND (group_id = ?2 OR (?2 IS NULL AND group_id IS NULL))",
        rusqlite::params![agent_name, group_id.unwrap_or("")],
        |row| row.get(0),
    ).unwrap_or(false);

    if existing {
        tx.execute(
            "UPDATE task_analysis_log SET last_analyzed_at = ?1
             WHERE agent_name = ?2 AND (group_id = ?3 OR (?3 IS NULL AND group_id IS NULL))",
            rusqlite::params![timestamp, agent_name, group_id.unwrap_or("")],
        )?;
    } else {
        tx.execute(
            "INSERT INTO task_analysis_log (agent_name, group_id, last_analyzed_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![agent_name, group_id, timestamp],
        )?;
    }
    Ok(())
}

/// Migrate task data from ztm.db into clawparty.db (one-time, idempotent).
/// Returns count of migrated tasks.
pub fn migrate_from_ztm_db(data_dir: &str) -> anyhow::Result<usize> {
    let ztm_path = format!("{}/ztm.db", data_dir);
    if !std::path::Path::new(&ztm_path).exists() {
        return Ok(0);
    }

    // Check if clawparty.db already has tasks
    let mut cp_conn = open_db(data_dir)?;
    let count: i64 = cp_conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(0); // already migrated
    }

    let ztm_conn = Connection::open(&ztm_path)?;
    let mut stmt = ztm_conn.prepare("SELECT * FROM tasks ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_task)?;

    let mut migrated = 0usize;
    for row in rows {
        let task = match row {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Insert without subtasks (they will be re-linked by parent_id)
        let tx = cp_conn.transaction()?;
        let deps = serde_json::to_string(&task.dependencies).unwrap_or_default();
        let pipe = serde_json::to_string(&task.pipeline_definition).unwrap_or_default();
        tx.execute(
            "INSERT INTO tasks
             (task_id, agent_name, group_id, parent_id, title, short_title, description,
              ai_description, status, progress, priority, dependencies, task_number,
              result_summary, prompt, is_pipeline, pipeline_definition,
              created_at, updated_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            rusqlite::params![
                &task.task_id, &task.agent_name, &task.group_id, &task.parent_id,
                &task.title, &task.short_title, &task.description, &task.ai_description,
                &task.status, task.progress, &task.priority, &deps, task.task_number,
                &task.result_summary, &task.prompt,
                if task.is_pipeline { 1 } else { 0 }, &pipe,
                task.created_at, task.updated_at, task.started_at, task.completed_at,
            ],
        )?;
        tx.commit()?;
        migrated += 1;
    }

    // Migrate analysis_log
    let mut stmt = ztm_conn.prepare("SELECT * FROM task_analysis_log")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>("agent_name")?,
            row.get::<_, Option<String>>("group_id")?,
            row.get::<_, f64>("last_analyzed_at")?,
        ))
    })?;
    for row in rows {
        let (agent, gid, ts) = row?;
        cp_conn.execute(
            "INSERT INTO task_analysis_log (agent_name, group_id, last_analyzed_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![agent, gid, ts],
        )?;
    }

    ts_eprint!("[clawparty.db] Migrated {} task(s) from ztm.db", migrated);
    Ok(migrated)
}

// Helper: deserialize JSON dependencies / pipeline_definition
fn parse_json_array<T: serde::de::DeserializeOwned>(s: Option<String>) -> Vec<T> {
    s.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
}

// ── Agent management (clawparty.db) ─────────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentRecord {
    pub agent_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub directory: String,
    pub config_path: String,
    pub workspace_dir: String,
    pub port: u16,
    pub pid: Option<u64>,
    pub status: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub config_json: Option<String>,
    pub error_msg: Option<String>,
    pub deleted: bool,
}

fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<AgentRecord> {
    Ok(AgentRecord {
        agent_name: row.get("agent_name")?,
        display_name: row.get("display_name")?,
        description: row.get("description")?,
        directory: row.get("directory")?,
        config_path: row.get("config_path")?,
        workspace_dir: row.get("workspace_dir")?,
        port: row.get::<_, i64>("port")? as u16,
        pid: row.get::<_, Option<i64>>("pid")?.map(|v| v as u64),
        status: row.get("status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        config_json: row.get("config_json")?,
        error_msg: row.get("error_msg")?,
        deleted: row.get::<_, i64>("deleted")? != 0,
    })
}

pub fn get_agent(data_dir: &str, agent_name: &str) -> anyhow::Result<Option<AgentRecord>> {
    let conn = open_db(data_dir)?;
    let agent = conn
        .query_row(
            "SELECT * FROM agents WHERE agent_name = ?1 AND deleted = 0",
            rusqlite::params![agent_name],
            row_to_agent,
        )
        .optional()?;
    Ok(agent)
}

pub fn get_agent_port(data_dir: &str, agent_name: &str) -> Option<u16> {
    get_agent(data_dir, agent_name).ok()?.map(|a| a.port)
}

pub fn list_agents(data_dir: &str) -> anyhow::Result<Vec<AgentRecord>> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn.prepare(
        "SELECT * FROM agents WHERE deleted = 0 ORDER BY
         CASE WHEN agent_name = '0#Agent' THEN 0 ELSE 1 END,
         agent_name ASC",
    )?;
    let rows: Vec<AgentRecord> = stmt
        .query_map([], row_to_agent)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn create_agent(
    data_dir: &str,
    agent_name: &str,
    display_name: Option<&str>,
    description: Option<&str>,
    directory: &str,
    config_path: &str,
    workspace_dir: &str,
    port: u16,
    config_json: Option<&str>,
) -> anyhow::Result<AgentRecord> {
    let conn = open_db(data_dir)?;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    conn.execute(
        "INSERT INTO agents
         (agent_name, display_name, description, directory, config_path, workspace_dir,
          port, status, created_at, updated_at, config_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10)",
        rusqlite::params![
            agent_name,
            display_name,
            description,
            directory,
            config_path,
            workspace_dir,
            port as i64,
            "stopped",
            t,
            config_json,
        ],
    )?;

    get_agent(data_dir, agent_name)?.ok_or_else(|| anyhow::anyhow!("Agent not found after create"))
}

pub fn update_agent_status(
    data_dir: &str,
    agent_name: &str,
    status: &str,
    pid: Option<u64>,
    error_msg: Option<&str>,
) -> anyhow::Result<()> {
    let conn = open_db(data_dir)?;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    conn.execute(
        "UPDATE agents SET status = ?1, pid = ?2, error_msg = ?3, updated_at = ?4
         WHERE agent_name = ?5 AND deleted = 0",
        rusqlite::params![
            status,
            pid.map(|v| v as i64),
            error_msg,
            t,
            agent_name,
        ],
    )?;
    Ok(())
}

pub fn delete_agent(data_dir: &str, agent_name: &str) -> anyhow::Result<()> {
    let conn = open_db(data_dir)?;
    conn.execute(
        "UPDATE agents SET deleted = 1, updated_at = ?1 WHERE agent_name = ?2",
        rusqlite::params![
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
            agent_name,
        ],
    )?;
    Ok(())
}

// ── Group chat management (clawparty.db) ──────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GroupChatRecord {
    pub group_id: String,
    pub group_name: String,
    pub owner_agent: String,
    pub members: Vec<String>,
    pub created_at: f64,
    pub updated_at: f64,
}

fn row_to_groupchat(row: &rusqlite::Row) -> rusqlite::Result<GroupChatRecord> {
    let members_str: String = row.get("members")?;
    let members: Vec<String> = serde_json::from_str(&members_str).unwrap_or_default();
    Ok(GroupChatRecord {
        group_id: row.get("group_id")?,
        group_name: row.get("group_name")?,
        owner_agent: row.get("owner_agent")?,
        members,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn get_group_chat(data_dir: &str, group_id: &str) -> anyhow::Result<Option<GroupChatRecord>> {
    let conn = open_db(data_dir)?;
    let gc = conn
        .query_row(
            "SELECT * FROM group_chats WHERE group_id = ?1",
            rusqlite::params![group_id],
            row_to_groupchat,
        )
        .optional()?;
    Ok(gc)
}

pub fn list_group_chats(data_dir: &str) -> anyhow::Result<Vec<GroupChatRecord>> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn.prepare("SELECT * FROM group_chats ORDER BY created_at DESC")?;
    let rows: Vec<GroupChatRecord> = stmt
        .query_map([], row_to_groupchat)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn create_group_chat(
    data_dir: &str,
    group_id: &str,
    group_name: &str,
    owner_agent: &str,
    members: &[String],
) -> anyhow::Result<GroupChatRecord> {
    let conn = open_db(data_dir)?;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    let members_json = serde_json::to_string(members)?;

    conn.execute(
        "INSERT INTO group_chats (group_id, group_name, owner_agent, members, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        rusqlite::params![group_id, group_name, owner_agent, members_json, t],
    )?;

    get_group_chat(data_dir, group_id)?.ok_or_else(|| anyhow::anyhow!("Group chat not found after create"))
}

pub fn update_group_chat(
    data_dir: &str,
    group_id: &str,
    group_name: Option<&str>,
    members: Option<&[String]>,
) -> anyhow::Result<()> {
    let conn = open_db(data_dir)?;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    if let Some(name) = group_name {
        conn.execute(
            "UPDATE group_chats SET group_name = ?1, updated_at = ?2 WHERE group_id = ?3",
            rusqlite::params![name, t, group_id],
        )?;
    }
    if let Some(m) = members {
        let members_json = serde_json::to_string(m)?;
        conn.execute(
            "UPDATE group_chats SET members = ?1, updated_at = ?2 WHERE group_id = ?3",
            rusqlite::params![members_json, t, group_id],
        )?;
    }
    Ok(())
}

pub fn delete_group_chat(data_dir: &str, group_id: &str) -> anyhow::Result<()> {
    let conn = open_db(data_dir)?;
    conn.execute(
        "DELETE FROM group_chats WHERE group_id = ?1",
        rusqlite::params![group_id],
    )?;
    Ok(())
}

// ── Chat log (clawparty.db) ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChatLogRecord {
    pub id: i64,
    pub time: f64,
    pub mesh: String,
    pub chat_type: String,
    pub chat_id: String,
    pub chat_name: Option<String>,
    pub creator: Option<String>,
    pub sender: String,
    pub event: String,
    pub content: Option<String>,
    pub members: Option<String>,
    pub session_id: Option<String>,
    pub muted: bool,
    pub msg_type: String,
}

fn row_to_chat_log(row: &rusqlite::Row) -> rusqlite::Result<ChatLogRecord> {
    Ok(ChatLogRecord {
        id: row.get("id")?,
        time: row.get("time")?,
        mesh: row.get("mesh")?,
        chat_type: row.get("chat_type")?,
        chat_id: row.get("chat_id")?,
        chat_name: row.get("chat_name")?,
        creator: row.get("creator")?,
        sender: row.get("sender")?,
        event: row.get("event")?,
        content: row.get("content")?,
        members: row.get("members")?,
        session_id: row.get("session_id")?,
        muted: row.get::<_, i64>("muted")? != 0,
        msg_type: row.get("msg_type")?,
    })
}

pub fn get_chat_log(
    data_dir: &str,
    chat_type: &str,
    chat_id: &str,
    limit: i64,
    msg_types: &[&str],
) -> anyhow::Result<Vec<ChatLogRecord>> {
    let conn = open_db(data_dir)?;
    // Build IN clause manually for msg_types (SQLite does not support array parameters)
    let msg_types_str: Vec<String> = msg_types.iter().map(|s| format!("'{}'", s.replace("'", "''"))).collect();
    let in_clause = msg_types_str.join(",");
    let sql = format!(
        "SELECT * FROM chat_log WHERE chat_type = ?1 AND chat_id = ?2 AND msg_type IN ({}) ORDER BY time DESC LIMIT ?3",
        in_clause,
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<ChatLogRecord> = stmt
        .query_map(rusqlite::params![chat_type, chat_id, limit], row_to_chat_log)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn log_chat(
    data_dir: &str,
    mesh: &str,
    chat_type: &str,
    chat_id: &str,
    chat_name: Option<&str>,
    creator: Option<&str>,
    sender: &str,
    event: &str,
    content: Option<&str>,
    members: Option<&str>,
    session_id: Option<&str>,
    muted: bool,
    msg_type: &str,
) -> anyhow::Result<()> {
    let conn = open_db(data_dir)?;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();
    conn.execute(
        "INSERT INTO chat_log (time, mesh, chat_type, chat_id, chat_name, creator, sender, event, content, members, session_id, muted, msg_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            t, mesh, chat_type, chat_id, chat_name, creator, sender, event, content, members, session_id, muted as i64, msg_type,
        ],
    )?;
    Ok(())
}

// ── Kanban Config (clawparty.db) ────────────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct KanbanConfig {
    pub id: Option<i64>,
    pub agent_name: String,
    pub group_id: Option<String>,
    pub name: String,
    pub prompt: Option<String>,
    pub config: Option<serde_json::Value>,
    pub created_at: Option<f64>,
    pub updated_at: Option<f64>,
}

fn row_to_kanban(row: &rusqlite::Row) -> rusqlite::Result<KanbanConfig> {
    let config_str: Option<String> = row.get("config")?;
    let config = config_str.and_then(|s| serde_json::from_str(&s).ok());
    Ok(KanbanConfig {
        id: row.get("id")?,
        agent_name: row.get("agent_name")?,
        group_id: row.get("group_id")?,
        name: row.get("name")?,
        prompt: row.get("prompt")?,
        config,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn get_kanban_config(data_dir: &str, agent_name: &str, group_id: Option<&str>) -> anyhow::Result<Option<KanbanConfig>> {
    let conn = open_db(data_dir)?;
    let sql = if group_id.is_some() {
        "SELECT * FROM kanban_configs WHERE agent_name = ?1 AND group_id = ?2"
    } else {
        "SELECT * FROM kanban_configs WHERE agent_name = ?1 AND group_id IS NULL"
    };
    let row = if let Some(gid) = group_id {
        conn.query_row(sql, rusqlite::params![agent_name, gid], row_to_kanban).optional()?
    } else {
        conn.query_row(sql, rusqlite::params![agent_name], row_to_kanban).optional()?
    };
    Ok(row)
}

pub fn set_kanban_config(
    data_dir: &str,
    agent_name: &str,
    group_id: Option<&str>,
    name: Option<&str>,
    prompt: Option<&str>,
    config: Option<&serde_json::Value>,
) -> anyhow::Result<KanbanConfig> {
    let conn = open_db(data_dir)?;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    let existing = get_kanban_config(data_dir, agent_name, group_id)?;
    if let Some(ref existing) = existing {
        let sql = if group_id.is_some() {
            "UPDATE kanban_configs SET name = ?1, prompt = ?2, config = ?3, updated_at = ?4 WHERE agent_name = ?5 AND group_id = ?6"
        } else {
            "UPDATE kanban_configs SET name = ?1, prompt = ?2, config = ?3, updated_at = ?4 WHERE agent_name = ?5 AND group_id IS NULL"
        };
        let config_json = config.map(|c| serde_json::to_string(c).unwrap_or_default());
        let new_name = name.unwrap_or(&existing.name);
        let new_prompt = prompt.unwrap_or_else(|| existing.prompt.as_deref().unwrap_or(""));
        if group_id.is_some() {
            conn.execute(sql, rusqlite::params![new_name, new_prompt, config_json, t, agent_name, group_id])?;
        } else {
            conn.execute(sql, rusqlite::params![new_name, new_prompt, config_json, t, agent_name])?;
        }
    } else {
        let config_json = config.map(|c| serde_json::to_string(c).unwrap_or_default());
        conn.execute(
            "INSERT INTO kanban_configs (agent_name, group_id, name, prompt, config, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            rusqlite::params![
                agent_name,
                group_id,
                name.unwrap_or("默认看板"),
                prompt,
                config_json,
                t,
            ],
        )?;
    }

    get_kanban_config(data_dir, agent_name, group_id)?
        .ok_or_else(|| anyhow::anyhow!("Kanban config not found after set"))
}
