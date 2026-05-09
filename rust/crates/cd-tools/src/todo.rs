//! `todo` — persistent session checkpointing.
//!
//! Stored as one JSON file per session. No global state, no async.
//! The CLI / agent loop provides a session id and calls
//! [`handle_todo`] with one of: list | add | update | remove | clear.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

impl TodoList {
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        // At most one in_progress at a time.
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::InProgress)
            .count()
            <= 1
    }
}

#[derive(Debug, Clone)]
pub struct TodoStore {
    root: PathBuf,
}

impl TodoStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, session: &str) -> PathBuf {
        self.root.join(format!("{session}.todo.json"))
    }

    pub fn load(&self, session: &str) -> std::io::Result<TodoList> {
        let p = self.path_for(session);
        if !p.exists() {
            return Ok(TodoList::default());
        }
        let bytes = fs::read(&p)?;
        let list: TodoList = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(list)
    }

    pub fn save(&self, session: &str, list: &TodoList) -> std::io::Result<()> {
        fs::create_dir_all(&self.root)?;
        let p = self.path_for(session);
        let json = serde_json::to_vec_pretty(list)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(p, json)
    }
}

#[must_use]
pub fn todo_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "todo",
            "description": "Persistent session todos. Operations: list | add | update | remove | clear. At most one item may be `in_progress` at a time.",
            "parameters": {
                "type": "object",
                "required": ["op"],
                "properties": {
                    "op":      {"type": "string", "enum": ["list","add","update","remove","clear"]},
                    "id":      {"type": "string"},
                    "content": {"type": "string"},
                    "status":  {"type": "string", "enum": ["pending","in_progress","completed"]}
                },
                "additionalProperties": false
            }
        }
    })
}

/// Apply `args` against the given list and return the new list + a
/// short message. Caller is responsible for persistence (so tests
/// stay hermetic). See [`TodoStore`] for the standard on-disk layout.
pub fn handle_todo(list: TodoList, args: &Value) -> Result<(TodoList, String), String> {
    let op = args.get("op").and_then(Value::as_str).ok_or("missing `op`")?;
    let mut list = list;
    let msg = match op {
        "list" => format!("{} item(s)", list.items.len()),
        "clear" => {
            let n = list.items.len();
            list.items.clear();
            format!("cleared {n} item(s)")
        }
        "add" => {
            let id = args.get("id").and_then(Value::as_str).ok_or("add requires `id`")?;
            let content = args.get("content").and_then(Value::as_str).ok_or("add requires `content`")?;
            let status = parse_status(args.get("status")).unwrap_or(TodoStatus::Pending);
            if list.items.iter().any(|i| i.id == id) {
                return Err(format!("duplicate id `{id}`"));
            }
            list.items.push(TodoItem { id: id.into(), content: content.into(), status });
            "added".into()
        }
        "update" => {
            let id = args.get("id").and_then(Value::as_str).ok_or("update requires `id`")?;
            let item = list.items.iter_mut().find(|i| i.id == id).ok_or_else(|| format!("unknown id `{id}`"))?;
            if let Some(c) = args.get("content").and_then(Value::as_str) {
                item.content = c.into();
            }
            if let Some(s) = parse_status(args.get("status")) {
                item.status = s;
            }
            "updated".into()
        }
        "remove" => {
            let id = args.get("id").and_then(Value::as_str).ok_or("remove requires `id`")?;
            let before = list.items.len();
            list.items.retain(|i| i.id != id);
            if list.items.len() == before {
                return Err(format!("unknown id `{id}`"));
            }
            "removed".into()
        }
        other => return Err(format!("unknown op `{other}`")),
    };
    if !list.is_consistent() {
        return Err("invariant violated: more than one todo is `in_progress`".into());
    }
    Ok((list, msg))
}

fn parse_status(v: Option<&Value>) -> Option<TodoStatus> {
    v.and_then(Value::as_str).and_then(|s| match s {
        "pending" => Some(TodoStatus::Pending),
        "in_progress" => Some(TodoStatus::InProgress),
        "completed" => Some(TodoStatus::Completed),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_update_remove_roundtrip() {
        let (l, _) = handle_todo(TodoList::default(), &json!({"op":"add","id":"a","content":"one"})).unwrap();
        let (l, _) = handle_todo(l, &json!({"op":"add","id":"b","content":"two","status":"in_progress"})).unwrap();
        assert_eq!(l.items.len(), 2);
        let (l, _) = handle_todo(l, &json!({"op":"update","id":"b","status":"completed"})).unwrap();
        assert_eq!(l.items[1].status, TodoStatus::Completed);
        let (l, _) = handle_todo(l, &json!({"op":"remove","id":"a"})).unwrap();
        assert_eq!(l.items.len(), 1);
    }

    #[test]
    fn duplicate_add_rejected() {
        let (l, _) = handle_todo(TodoList::default(), &json!({"op":"add","id":"x","content":"t"})).unwrap();
        let r = handle_todo(l, &json!({"op":"add","id":"x","content":"t"}));
        assert!(r.is_err());
    }

    #[test]
    fn in_progress_invariant_enforced() {
        let l = TodoList::default();
        let (l, _) = handle_todo(l, &json!({"op":"add","id":"a","content":"x","status":"in_progress"})).unwrap();
        let err = handle_todo(l, &json!({"op":"add","id":"b","content":"y","status":"in_progress"})).unwrap_err();
        assert!(err.contains("invariant"));
    }

    #[test]
    fn store_persist_roundtrip() {
        let dir = std::env::temp_dir().join(format!("cd-tools-todo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = TodoStore::new(&dir);
        let (l, _) = handle_todo(TodoList::default(), &json!({"op":"add","id":"a","content":"t"})).unwrap();
        store.save("sess1", &l).unwrap();
        let loaded = store.load("sess1").unwrap();
        assert_eq!(loaded.items.len(), 1);
        let missing = store.load("does-not-exist").unwrap();
        assert_eq!(missing.items.len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_enum_covers_ops() {
        let s = todo_schema();
        let ops = s["function"]["parameters"]["properties"]["op"]["enum"].clone();
        assert_eq!(ops, json!(["list","add","update","remove","clear"]));
    }
}
