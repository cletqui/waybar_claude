use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const ICON: &str = "\u{ee0d}";
const STALE_AFTER: i64 = 30 * 60;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Window {
    pub used_percentage: f64,
    pub resets_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct Cache {
    pub fetched_at: i64,
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
}

fn window(v: Option<&Value>) -> Option<Window> {
    let v = v?;
    Some(Window {
        used_percentage: v.get("used_percentage")?.as_f64()?,
        resets_at: v.get("resets_at").and_then(Value::as_i64),
    })
}

impl Cache {
    pub fn from_statusline(input: &str, now: i64) -> Option<Cache> {
        let v: Value = serde_json::from_str(input).ok()?;
        let limits = v.get("rate_limits")?;
        let five_hour = window(limits.get("five_hour"));
        let seven_day = window(limits.get("seven_day"));
        if five_hour.is_none() && seven_day.is_none() {
            return None;
        }
        Some(Cache {
            fetched_at: now,
            five_hour,
            seven_day,
        })
    }
}

const REWRITE_AFTER: i64 = 30;

impl Cache {
    pub fn is_fresh_copy_of(&self, new: &Cache) -> bool {
        self.five_hour == new.five_hour
            && self.seven_day == new.seven_day
            && new.fetched_at - self.fetched_at < REWRITE_AFTER
    }
}

impl Window {
    fn expired(&self, now: i64) -> bool {
        self.resets_at.is_some_and(|r| r <= now)
    }

    fn percent(&self, now: i64) -> f64 {
        if self.expired(now) {
            0.0
        } else {
            self.used_percentage.clamp(0.0, 100.0)
        }
    }
}

fn duration(secs: i64) -> String {
    let (d, h, m) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h{m:02}m")
    } else {
        format!("{}m", m.max(1))
    }
}

fn bar(pct: f64) -> String {
    let filled = (pct / 10.0).round() as usize;
    format!("{}{}", "▰".repeat(filled), "▱".repeat(10 - filled))
}

fn line(label: &str, w: &Window, now: i64) -> String {
    let pct = w.percent(now);
    let tail = match w.resets_at {
        Some(r) if r > now => format!("  resets in {}", duration(r - now)),
        Some(_) => "  window reset".to_string(),
        None => String::new(),
    };
    format!("{label}  {} {pct:>3.0}%{tail}", bar(pct))
}

fn level(pct: f64) -> &'static str {
    if pct >= 80.0 {
        "crit"
    } else if pct >= 50.0 {
        "warn"
    } else {
        "ok"
    }
}

pub fn view(cache: Option<&Cache>, now: i64) -> Value {
    let Some(cache) = cache else {
        return json!({
            "text": ICON,
            "tooltip": "Claude usage\nNo data yet, open Claude Code once.",
            "class": ["claude", "idle"],
            "percentage": 0,
        });
    };
    let windows = [("5h", cache.five_hour), ("7d", cache.seven_day)];
    let mut rows = vec!["Claude usage".to_string()];
    let mut peak: f64 = 0.0;
    for (label, w) in &windows {
        if let Some(w) = w {
            rows.push(line(label, w, now));
            peak = peak.max(w.percent(now));
        }
    }
    let age = (now - cache.fetched_at).max(0);
    rows.push(format!("updated {} ago", duration(age)));
    json!({
        "text": ICON,
        "tooltip": rows.join("\n"),
        "class": classes(peak, age),
        "percentage": peak.round() as u8,
    })
}

fn classes(peak: f64, age: i64) -> Vec<&'static str> {
    let mut class = vec!["claude", level(peak)];
    if peak >= 90.0 {
        class.push("hot");
    }
    if age > STALE_AFTER {
        class.push("stale");
    }
    class
}

pub fn detail(cache: Option<&Cache>, now: i64) -> Value {
    let Some(w) = cache.and_then(|c| c.five_hour) else {
        return json!({ "text": "", "class": ["claude", "idle"] });
    };
    let pct = w.percent(now);
    let reset = match w.resets_at {
        Some(r) if r > now => format!(" · {}", duration(r - now)),
        _ => String::new(),
    };
    let age = cache.map_or(0, |c| (now - c.fetched_at).max(0));
    json!({
        "text": format!("{pct:.0}%{reset}"),
        "class": classes(pct, age),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    fn cache(five: f64, seven: f64) -> Cache {
        Cache {
            fetched_at: NOW - 60,
            five_hour: Some(Window {
                used_percentage: five,
                resets_at: Some(NOW + 2 * 3600 + 13 * 60),
            }),
            seven_day: Some(Window {
                used_percentage: seven,
                resets_at: Some(NOW + 4 * 86400 + 3 * 3600),
            }),
        }
    }

    #[test]
    fn parses_statusline_rate_limits() {
        let input = r#"{"rate_limits":{"five_hour":{"used_percentage":42.5,"resets_at":99},"seven_day":{"used_percentage":10,"resets_at":100}}}"#;
        let c = Cache::from_statusline(input, NOW).unwrap();
        assert_eq!(c.five_hour.unwrap().used_percentage, 42.5);
        assert_eq!(c.seven_day.unwrap().resets_at, Some(100));
        assert_eq!(c.fetched_at, NOW);
    }

    #[test]
    fn statusline_without_limits_is_ignored() {
        assert!(Cache::from_statusline(r#"{"model":{}}"#, NOW).is_none());
        assert!(Cache::from_statusline("not json", NOW).is_none());
        assert!(Cache::from_statusline(r#"{"rate_limits":{}}"#, NOW).is_none());
    }

    #[test]
    fn color_follows_the_busiest_window() {
        assert_eq!(view(Some(&cache(10.0, 20.0)), NOW)["class"][1], "ok");
        assert_eq!(view(Some(&cache(10.0, 60.0)), NOW)["class"][1], "warn");
        assert_eq!(view(Some(&cache(85.0, 20.0)), NOW)["class"][1], "crit");
    }

    #[test]
    fn expired_window_counts_as_zero() {
        let mut c = cache(95.0, 20.0);
        c.five_hour.as_mut().unwrap().resets_at = Some(NOW - 1);
        let v = view(Some(&c), NOW);
        assert_eq!(v["class"][1], "ok");
        assert!(v["tooltip"].as_str().unwrap().contains("window reset"));
    }

    #[test]
    fn tooltip_lists_every_window_and_age() {
        let v = view(Some(&cache(42.0, 18.0)), NOW);
        let t = v["tooltip"].as_str().unwrap();
        assert!(t.contains("5h  ▰▰▰▰▱▱▱▱▱▱  42%  resets in 2h13m"), "{t}");
        assert!(t.contains("7d  ▰▰▱▱▱▱▱▱▱▱  18%  resets in 4d 3h"), "{t}");
        assert!(t.contains("updated 1m ago"), "{t}");
        assert_eq!(v["text"], ICON);
    }

    #[test]
    fn old_cache_is_marked_stale() {
        let mut c = cache(10.0, 10.0);
        c.fetched_at = NOW - STALE_AFTER - 1;
        assert_eq!(view(Some(&c), NOW)["class"][2], "stale");
    }

    #[test]
    fn unchanged_recent_cache_is_not_rewritten() {
        let old = cache(10.0, 10.0);
        let mut new = cache(10.0, 10.0);
        new.fetched_at = old.fetched_at + 5;
        assert!(old.is_fresh_copy_of(&new));
        new.fetched_at = old.fetched_at + 31;
        assert!(!old.is_fresh_copy_of(&new));
        let mut changed = cache(11.0, 10.0);
        changed.fetched_at = old.fetched_at + 5;
        assert!(!old.is_fresh_copy_of(&changed));
    }

    #[test]
    fn hot_class_from_ninety_percent() {
        assert_eq!(view(Some(&cache(91.0, 0.0)), NOW)["class"][2], "hot");
        assert_eq!(
            view(Some(&cache(89.0, 0.0)), NOW)["class"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn detail_shows_five_hour_usage_and_reset() {
        let v = detail(Some(&cache(37.0, 64.0)), NOW);
        assert_eq!(v["text"], "37% · 2h13m");
        assert_eq!(v["class"][1], "ok");
        assert_eq!(detail(None, NOW)["text"], "");
    }

    #[test]
    fn missing_cache_is_idle() {
        assert_eq!(view(None, NOW)["class"][1], "idle");
    }
}
