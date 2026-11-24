#![forbid(unsafe_code)]

//! In-process notification queue with auto-expiry (Blueprint 6.7).

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub summary: String,
    pub body: String,
    pub created_at: Instant,
    pub ttl: Duration,
}

#[derive(Default)]
pub struct NotificationQueue {
    next_id: u32,
    items: Vec<Notification>,
}

impl NotificationQueue {
    pub fn push(
        &mut self,
        summary: impl Into<String>,
        body: impl Into<String>,
        ttl_secs: u64,
    ) -> u32 {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.items.push(Notification {
            id,
            summary: summary.into(),
            body: body.into(),
            created_at: Instant::now(),
            ttl: Duration::from_secs(ttl_secs),
        });
        id
    }

    pub fn prune_expired(&mut self) {
        self.items.retain(|n| n.created_at.elapsed() < n.ttl);
    }

    pub fn active(&self) -> &[Notification] {
        &self.items
    }
}
