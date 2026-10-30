mod usage;

use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use usage::Cache;

const HELP: &str = "usage: waybar_claude [--cache FILE] [--watch] <show|detail|record>\n  show    print waybar JSON from the cache (--watch: keep running, print on change)\n  detail  print 5h usage and reset as short text for a hover drawer\n  record  read Claude Code statusline JSON on stdin and update the cache\n";

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn default_cache() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| "/tmp".into());
    base.join("waybar_claude/usage.json")
}

fn load(path: &PathBuf) -> Option<Cache> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn save(path: &PathBuf, cache: &Cache) {
    let Ok(json) = serde_json::to_string(cache) else {
        return;
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        std::fs::rename(&tmp, path).ok();
    }
}

fn watch_loop(path: &PathBuf, view: fn(Option<&Cache>, i64) -> serde_json::Value) {
    let mut last = String::new();
    loop {
        let out = view(load(path).as_ref(), now()).to_string();
        if out != last {
            let mut stdout = std::io::stdout().lock();
            if writeln!(stdout, "{out}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                return;
            }
            last = out;
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut cache_path = None;
    let mut command = None;
    let mut watch = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--watch" => watch = true,
            "--cache" => cache_path = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            _ => command = Some(arg),
        }
    }
    let path = cache_path.unwrap_or_else(default_cache);
    match command.as_deref() {
        Some(name @ ("show" | "detail")) => {
            let view = if name == "show" {
                usage::view
            } else {
                usage::detail
            };
            if watch {
                watch_loop(&path, view);
            } else {
                println!("{}", view(load(&path).as_ref(), now()));
            }
        }
        Some("record") => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).ok();
            if let Some(cache) = Cache::from_statusline(&input, now())
                && !load(&path).is_some_and(|old| old.is_fresh_copy_of(&cache))
            {
                save(&path, &cache);
            }
        }
        _ => {
            eprint!("{HELP}");
            std::process::exit(2);
        }
    }
}
