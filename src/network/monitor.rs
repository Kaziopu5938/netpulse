use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::network::adapter;
use crate::state::Sample;

struct Prev {
    received: u64,
    sent: u64,
    at: Instant,
}

/// Turns the monotonically increasing interface byte counters into bytes/second.
pub struct Monitor {
    prev: HashMap<u32, Prev>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            prev: HashMap::new(),
        }
    }

    /// `selected` is an interface GUID; `None` aggregates every eligible adapter.
    pub fn sample(&mut self, selected: Option<&str>) -> Sample {
        let now = Instant::now();
        let mut sample = Sample::default();
        let mut seen = HashSet::new();
        let mut names = Vec::new();

        adapter::with_rows(|rows| {
            for row in rows {
                if !adapter::is_up(row) {
                    continue;
                }
                if let Some(guid) = selected {
                    if adapter::guid_string(&row.InterfaceGuid) != guid {
                        continue;
                    }
                }
                // A user picked adapter is used verbatim; the aggregate skips
                // virtual interfaces so traffic is not counted twice.
                if selected.is_none() && !adapter::is_eligible(row) {
                    continue;
                }

                sample.connected = true;
                let name = adapter::alias(row);
                if !name.is_empty() && !names.contains(&name) {
                    names.push(name);
                }

                let key = row.InterfaceIndex;
                seen.insert(key);
                let previous = self.prev.insert(
                    key,
                    Prev {
                        received: row.InOctets,
                        sent: row.OutOctets,
                        at: now,
                    },
                );
                if let Some(previous) = previous {
                    let seconds = (now - previous.at).as_secs_f64();
                    if seconds > 0.05 {
                        sample.down_bps +=
                            row.InOctets.saturating_sub(previous.received) as f64 / seconds;
                        sample.up_bps +=
                            row.OutOctets.saturating_sub(previous.sent) as f64 / seconds;
                    }
                }
            }
        });

        if names.len() == 1 {
            sample.adapter = names.remove(0);
        } else if names.len() > 1 {
            sample.adapter = format!("{} adapters", names.len());
        } else if let Some(guid) = selected {
            sample.adapter = format!("Adapter {guid} (disconnected)");
        } else {
            sample.adapter = "No active adapter".to_string();
        }

        sample.down_bps = sample.down_bps.max(0.0);
        sample.up_bps = sample.up_bps.max(0.0);
        self.prev.retain(|key, _| seen.contains(key));
        sample
    }
}
