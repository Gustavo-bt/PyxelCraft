#[allow(clippy::wildcard_imports)]
use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

impl JavaClient {
    pub fn handle_keep_alive(&self, player: &Player, keep_alive: &SKeepAlive) {
        {
            let mut pending = self.pending_keep_alives.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(pos) = pending.iter().position(|(id, _)| *id == keep_alive.keep_alive_id) {
                pending.swap_remove(pos);
            }
        }
        self.wait_for_keep_alive.store(false, Ordering::Relaxed);
        let now_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis() as i64,
            Err(_) => return,
        };
        let sent_ms = keep_alive.keep_alive_id;
        if sent_ms > 0 && now_ms >= sent_ms {
            let diff = now_ms - sent_ms;
            if diff < 5000 {
                player.ping.store(diff as u32, Ordering::Relaxed);
            }
        }
    }
}
