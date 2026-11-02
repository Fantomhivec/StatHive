use chrono::Local;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

const MAX_LINES: usize = 2000;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub ts: String,
    pub level: String,
    pub message: String,
}

type InfoSink = Arc<dyn Fn(&str) + Send + Sync>;

static INFO_SINK: OnceLock<Mutex<Option<InfoSink>>> = OnceLock::new();

fn sink_slot() -> &'static Mutex<Option<InfoSink>> {
    INFO_SINK.get_or_init(|| Mutex::new(None))
}

/// Extra sink for `info`/`warn` (e.g. calculator run journal while export runs).
pub fn set_info_sink(sink: Option<InfoSink>) {
    if let Ok(mut g) = sink_slot().lock() {
        *g = sink;
    }
}

fn notify_sink(message: &str) {
    if let Ok(g) = sink_slot().lock() {
        if let Some(sink) = g.as_ref() {
            sink(message);
        }
    }
}

#[derive(Clone, Default)]
pub struct AppLog {
    inner: Arc<Mutex<LogState>>,
}

#[derive(Default)]
struct LogState {
    lines: VecDeque<LogLine>,
    /// Monotonic revision — IPC can skip full copy when unchanged.
    revision: u64,
}

impl AppLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn info(&self, message: impl Into<String>) {
        self.push("info", message.into(), true);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.push("warn", message.into(), true);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.push("error", message.into(), true);
    }

    /// Push without invoking the optional info sink (avoids duplicate run-log lines).
    pub fn info_no_sink(&self, message: impl Into<String>) {
        self.push("info", message.into(), false);
    }

    fn push(&self, level: &str, message: String, mirror: bool) {
        let line = LogLine {
            ts: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.clone(),
        };
        if let Ok(mut st) = self.inner.lock() {
            if st.lines.len() >= MAX_LINES {
                st.lines.pop_front();
            }
            st.lines.push_back(line);
            st.revision = st.revision.wrapping_add(1);
        }
        if mirror {
            notify_sink(&message);
        }
    }

    /// Если `known_revision` совпадает — `lines: None` (клиент оставляет буфер).
    pub fn snapshot_if_changed(&self, known_revision: u64) -> (Option<Vec<LogLine>>, u64) {
        let Ok(st) = self.inner.lock() else {
            return (Some(Vec::new()), 0);
        };
        if known_revision == st.revision && known_revision != 0 {
            return (None, st.revision);
        }
        (
            Some(st.lines.iter().cloned().collect()),
            st.revision,
        )
    }

    pub fn clear(&self) {
        if let Ok(mut st) = self.inner.lock() {
            st.lines.clear();
            st.revision = st.revision.wrapping_add(1);
        }
    }
}
