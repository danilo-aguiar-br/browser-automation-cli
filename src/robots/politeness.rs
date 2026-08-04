// SPDX-License-Identifier: MIT OR Apache-2.0
//! Crawl-delay politeness (host-gated sleep between HTTP scrapes).
//!
//! `robotstxt` crate only exposes allow/disallow. Crawl-delay is a common
//! non-standard directive; we parse it from the robots body and gate GETs
//! per origin so parallel batch/crawl never stampede one host.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use url::Url;

/// Process-wide last-hit clock per origin (`scheme://host[:port]`).
static LAST_HIT: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

/// Process-wide crawl-delay (seconds) learned from robots.txt per origin.
static DELAYS: Mutex<Option<HashMap<String, f64>>> = Mutex::new(None);

/// Parse `Crawl-delay` seconds for `user_agent` from a robots.txt body.
///
/// Walks groups: prefers a group whose User-agent matches our product token
/// or `*`. Returns the first positive delay found in the best-matching group.
pub fn parse_crawl_delay_secs(robots_body: &str, user_agent: &str) -> Option<f64> {
    let ua_token = user_agent
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or(user_agent)
        .to_ascii_lowercase();

    let mut best_star: Option<f64> = None;
    let mut best_specific: Option<f64> = None;
    let mut group_is_star = false;
    let mut group_is_specific = false;
    let mut group_delay: Option<f64> = None;

    let flush = |group_is_star: bool,
                 group_is_specific: bool,
                 group_delay: Option<f64>,
                 best_star: &mut Option<f64>,
                 best_specific: &mut Option<f64>| {
        if let Some(d) = group_delay {
            if group_is_specific {
                *best_specific = Some(d);
            } else if group_is_star {
                *best_star = Some(d);
            }
        }
    };

    for raw in robots_body.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "user-agent" => {
                // New group starts after a previous UA only when we already had
                // a delay or disallow rules; simple: if we see UA after delay,
                // flush previous group.
                if group_delay.is_some() || group_is_star || group_is_specific {
                    // Only flush when starting a new group after content.
                    // Detect new group: if current group already has UA flags
                    // and we already recorded a delay, flush.
                    if group_delay.is_some() {
                        flush(
                            group_is_star,
                            group_is_specific,
                            group_delay,
                            &mut best_star,
                            &mut best_specific,
                        );
                        group_is_star = false;
                        group_is_specific = false;
                        group_delay = None;
                    }
                }
                let v = value.to_ascii_lowercase();
                if v == "*" {
                    group_is_star = true;
                } else if v.contains(&ua_token) || ua_token.contains(&v) {
                    group_is_specific = true;
                }
            }
            "crawl-delay" => {
                if let Ok(d) = value.parse::<f64>() {
                    if d.is_finite() && d > 0.0 {
                        group_delay = Some(d.min(3600.0));
                    }
                }
            }
            _ => {}
        }
    }
    flush(
        group_is_star,
        group_is_specific,
        group_delay,
        &mut best_star,
        &mut best_specific,
    );
    best_specific.or(best_star)
}

/// Origin key for politeness maps (`scheme://host[:port]`).
pub fn origin_key(url: &str) -> Option<String> {
    let u = Url::parse(url).ok()?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return None;
    }
    Some(u.origin().ascii_serialization())
}

/// Remember crawl-delay for an origin (seconds).
pub fn remember_crawl_delay(origin: &str, delay_secs: f64) {
    if !(delay_secs.is_finite() && delay_secs > 0.0) {
        return;
    }
    let mut guard = DELAYS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(origin.to_string(), delay_secs);
}

/// Effective delay seconds: max(robots delay, XDG scrape_min_delay_ms/1000).
pub fn effective_delay_secs(origin: &str) -> f64 {
    let robots = {
        let guard = DELAYS.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .and_then(|m| m.get(origin).copied())
            .unwrap_or(0.0)
    };
    let floor_ms = crate::xdg::resolve_scrape_min_delay_ms();
    let floor = (floor_ms as f64) / 1000.0;
    robots.max(floor)
}

/// Apply XDG jitter ratio to a base delay (ratio 0 = identity).
///
/// Uses a cheap thread-local LCG so tests stay deterministic when ratio is 0.
pub fn apply_jitter(delay_secs: f64, ratio: f64) -> f64 {
    if delay_secs <= 0.0 || !(ratio.is_finite()) || ratio <= 0.0 {
        return delay_secs.max(0.0);
    }
    let ratio = ratio.clamp(0.0, 1.0);
    // Factor in [1-ratio, 1+ratio]
    let u = jitter_unit();
    let factor = (1.0 - ratio) + (2.0 * ratio * u);
    (delay_secs * factor).max(0.0)
}

fn jitter_unit() -> f64 {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    STATE.with(|s| {
        let mut x = s.get();
        if x == 0 {
            x = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9e37_79b9_7f4a_7c15)
                | 1;
        }
        // LCG (Numerical Recipes)
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        s.set(x);
        (x as f64) / (u64::MAX as f64)
    })
}

/// Sleep until the origin is polite enough for another GET.
///
/// Returns the delay seconds that were applied (0 if none).
pub async fn wait_origin(url: &str) -> f64 {
    let Some(origin) = origin_key(url) else {
        return 0.0;
    };
    let base = effective_delay_secs(&origin);
    let ratio = crate::xdg::resolve_scrape_delay_jitter_ratio();
    let delay = apply_jitter(base, ratio);
    if delay <= 0.0 {
        // Still record hit for future floor delays.
        mark_hit(&origin);
        return 0.0;
    }
    let sleep_for = {
        let mut guard = LAST_HIT.lock().unwrap_or_else(|e| e.into_inner());
        let map = guard.get_or_insert_with(HashMap::new);
        if let Some(prev) = map.get(&origin) {
            let elapsed = prev.elapsed();
            let need = Duration::from_secs_f64(delay);
            if elapsed < need {
                need - elapsed
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        }
    };
    if !sleep_for.is_zero() {
        tokio::time::sleep(sleep_for).await;
    }
    mark_hit(&origin);
    delay
}

fn mark_hit(origin: &str) {
    let mut guard = LAST_HIT.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(origin.to_string(), Instant::now());
}

/// Snapshot remembered delay for envelope diagnostics.
pub fn remembered_delay_secs(url: &str) -> Option<f64> {
    let origin = origin_key(url)?;
    let guard = DELAYS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|m| m.get(&origin).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_star_crawl_delay() {
        let body = "User-agent: *\nCrawl-delay: 2.5\nDisallow:\n";
        let d = parse_crawl_delay_secs(body, "browser-automation-cli/0.1.6").unwrap();
        assert!((d - 2.5).abs() < 0.001);
    }

    #[test]
    fn prefers_specific_ua_group() {
        let body = "User-agent: *\nCrawl-delay: 10\n\nUser-agent: browser-automation-cli\nCrawl-delay: 1\n";
        let d = parse_crawl_delay_secs(body, "browser-automation-cli/0.1.6 local-scrape").unwrap();
        assert!((d - 1.0).abs() < 0.001);
    }

    #[test]
    fn jitter_zero_is_identity() {
        assert!((apply_jitter(2.0, 0.0) - 2.0).abs() < 1e-9);
        assert!((apply_jitter(0.0, 0.5) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn jitter_ratio_stays_in_band() {
        for _ in 0..32 {
            let j = apply_jitter(10.0, 0.2);
            assert!((8.0 - 1e-9..=12.0 + 1e-9).contains(&j), "j={j}");
        }
    }

    #[test]
    fn origin_key_https() {
        assert_eq!(
            origin_key("https://Example.com:443/path").as_deref(),
            Some("https://example.com")
        );
    }
}
