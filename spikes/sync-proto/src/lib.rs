//! Minimal offline-first sync prototype for the Epiora v0.1 spike (spec §7.3).
//!
//! Unit of change: (record, field, value, device, lamport, base_version).
//! The server is the ordering authority; devices hold a high-water mark.
//! A change conflicts when its base_version is older than the field's
//! current version and the value differs.

use std::collections::HashMap;

/// Device id the server stamps on conflict resolutions.
pub const SERVER_DEVICE: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Change {
    pub record: u64,
    pub field: String,
    pub value: String,
    pub device: u32,
    pub lamport: u64,
    pub base_version: u64,
}

#[derive(Debug)]
pub struct PushResult {
    pub accepted: usize,
    /// Ids of the conflicts this push raised (indexes into `Server::conflicts`).
    pub conflicts: Vec<u64>,
}

#[derive(Debug)]
pub struct Conflict {
    pub id: u64,
    pub record: u64,
    pub field: String,
    /// The change currently applied on the server.
    pub existing: Change,
    /// The rejected incoming change; kept, not applied.
    pub incoming: Change,
    /// The winning value, once resolved.
    pub resolved: Option<String>,
}

/// An applied change with its server-assigned sequence and field version.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub seq: u64,
    pub change: Change,
    pub version: u64,
}

#[derive(Debug, Default)]
pub struct Server {
    /// (record, field) -> (version, value). Version counts applied writes.
    fields: HashMap<(u64, String), (u64, String)>,
    log: Vec<LogEntry>,
    conflicts: Vec<Conflict>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a batch of changes in deterministic (lamport, device) order.
    pub fn apply(&mut self, mut changes: Vec<Change>) -> PushResult {
        changes.sort_by_key(|c| (c.lamport, c.device));
        let mut result = PushResult {
            accepted: 0,
            conflicts: Vec::new(),
        };
        for ch in changes {
            let key = (ch.record, ch.field.clone());
            let (version, current) = self.fields.get(&key).cloned().unwrap_or_default();
            if version > 0 && ch.base_version < version && ch.value != current {
                let existing = self
                    .log
                    .iter()
                    .rev()
                    .find(|e| e.change.record == ch.record && e.change.field == ch.field)
                    .expect("versioned field has a log entry")
                    .change
                    .clone();
                let id = self.conflicts.len() as u64;
                self.conflicts.push(Conflict {
                    id,
                    record: ch.record,
                    field: ch.field.clone(),
                    existing,
                    incoming: ch,
                    resolved: None,
                });
                result.conflicts.push(id);
            } else {
                let version = version + 1;
                self.fields.insert(key, (version, ch.value.clone()));
                let seq = self.log.len() as u64;
                self.log.push(LogEntry {
                    seq,
                    change: ch,
                    version,
                });
                result.accepted += 1;
            }
        }
        result
    }

    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    /// Resolve a conflict: the winning value becomes the field's value and is
    /// appended to the log so pulling devices converge on it.
    pub fn resolve(&mut self, conflict_id: u64, winner: &str) {
        let conflict = self
            .conflicts
            .iter_mut()
            .find(|c| c.id == conflict_id)
            .expect("unknown conflict id");
        conflict.resolved = Some(winner.to_string());
        let key = (conflict.record, conflict.field.clone());
        let (version, _) = self.fields.get(&key).cloned().unwrap_or_default();
        let lamport = conflict.existing.lamport.max(conflict.incoming.lamport) + 1;
        let version = version + 1;
        self.fields.insert(key, (version, winner.to_string()));
        let seq = self.log.len() as u64;
        self.log.push(LogEntry {
            seq,
            change: Change {
                record: conflict.record,
                field: conflict.field.clone(),
                value: winner.to_string(),
                device: SERVER_DEVICE,
                lamport,
                base_version: version - 1,
            },
            version,
        });
    }

    /// All applied changes, in server sequence order.
    pub fn entries(&self) -> &[LogEntry] {
        &self.log
    }

    pub fn value(&self, record: u64, field: &str) -> Option<&str> {
        self.fields
            .get(&(record, field.to_string()))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug)]
pub struct Device {
    pub id: u32,
    /// Simulated wall clock (ms). Ordering never reads it; it exists so tests
    /// can prove ordering does not depend on wall time.
    pub wall_ms: i64,
    lamport: u64,
    /// (record, field) -> (server version seen, value).
    fields: HashMap<(u64, String), (u64, String)>,
    pending: Vec<Change>,
    /// Last server sequence number this device has applied.
    hwm: u64,
}

impl Device {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            wall_ms: 0,
            lamport: 0,
            fields: HashMap::new(),
            pending: Vec::new(),
            hwm: 0,
        }
    }

    pub fn edit(&mut self, record: u64, field: &str, value: &str) {
        self.lamport += 1;
        let key = (record, field.to_string());
        let base_version = self.fields.get(&key).map(|(v, _)| *v).unwrap_or(0)
            + self
                .pending
                .iter()
                .filter(|c| c.record == record && c.field == field)
                .count() as u64;
        self.fields
            .entry(key)
            .and_modify(|(_, v)| *v = value.to_string())
            .or_insert((0, value.to_string()));
        self.pending.push(Change {
            record,
            field: field.to_string(),
            value: value.to_string(),
            device: self.id,
            lamport: self.lamport,
            base_version,
        });
    }

    /// Push all pending changes; the queue is drained whether or not the
    /// server accepted them (conflicted writes live on in `Server::conflicts`).
    pub fn push(&mut self, server: &mut Server) -> PushResult {
        server.apply(std::mem::take(&mut self.pending))
    }

    /// Apply every server log entry past this device's high-water mark.
    pub fn pull(&mut self, server: &Server) {
        for entry in server.entries().iter().skip(self.hwm as usize) {
            self.fields.insert(
                (entry.change.record, entry.change.field.clone()),
                (entry.version, entry.change.value.clone()),
            );
            self.lamport = self.lamport.max(entry.change.lamport);
            self.hwm = entry.seq + 1;
        }
    }

    pub fn value(&self, record: u64, field: &str) -> Option<&str> {
        self.fields
            .get(&(record, field.to_string()))
            .map(|(_, v)| v.as_str())
    }

    /// Changes made locally but not yet pushed.
    pub fn pending(&self) -> &[Change] {
        &self.pending
    }

    /// Drop the local queue without pushing (tests only: simulates a device
    /// abandoning offline edits).
    pub fn discard_pending(&mut self) {
        self.pending.clear();
    }
}
