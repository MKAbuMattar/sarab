use crate::cli::{self, Command};
use crate::os::platform as os;
use serde_json::Value;
use std::time::{Duration, SystemTime};

pub fn run(args: &[String]) -> i32 {
    let exe = std::env::current_exe()
        .map(|p| p.with_extension("exe"))
        .unwrap_or_default();
    match cli::parse(args) {
        Err(e) => {
            eprintln!("sarab: {e}");
            2
        }
        Ok(Some(Command::Status)) => status(&exe, args.iter().any(|a| a == "--json")),
        _ => match std::process::Command::new(&exe).args(args).spawn() {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("sarab: could not start {}: {e}", exe.display());
                1
            }
        },
    }
}

fn status(exe: &std::path::Path, json: bool) -> i32 {
    let file = crate::core::settings::config_dir().join("status.json");
    let read = || -> Option<Value> { serde_json::from_slice(&std::fs::read(&file).ok()?).ok() };
    let running = read()
        .and_then(|v| v["pid"].as_u64())
        .is_some_and(|p| os::is_sarab(p as u32));
    if !running {
        eprintln!("Sarab is not running. Start it with: sarab");
        return 1;
    }
    let before = std::fs::metadata(&file).and_then(|m| m.modified()).ok();
    if std::process::Command::new(exe)
        .arg("status")
        .spawn()
        .is_ok()
    {
        let until = SystemTime::now() + Duration::from_millis(1500);
        while SystemTime::now() < until
            && std::fs::metadata(&file).and_then(|m| m.modified()).ok() == before
        {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let Some(v) = read() else {
        eprintln!("sarab: status.json is unreadable");
        return 1;
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
    } else {
        print!("{}", status_text(&v, |k| crate::ui_text("en", k)));
    }
    0
}

pub fn status_text(v: &Value, text: impl Fn(&str) -> String) -> String {
    let manual = match v["manual"].as_bool() {
        Some(true) => ", paused by you",
        Some(false) => ", pause rules off",
        None => "",
    };
    let mut out = format!(
        "Sarab {} (pid {}){manual}\n",
        v["version"].as_str().unwrap_or("?"),
        v["pid"]
    );
    for (i, d) in v["displays"].as_array().into_iter().flatten().enumerate() {
        let r: Vec<i64> = d["rect"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
            .collect();
        let size = match r.as_slice() {
            [l, t, rr, b] => format!("{}x{}", rr - l, b - t),
            _ => "?".into(),
        };
        let what = match (d["title"].as_str(), d["kind"].as_str()) {
            (Some(t), Some(k)) => format!("{t} ({k})"),
            (Some(t), None) => t.to_string(),
            _ => "no wallpaper".into(),
        };
        let state = if let Some(e) = d["error"].as_str() {
            format!("error: {e}")
        } else if d["wallpaper"].is_null() {
            String::new()
        } else {
            text(&format!(
                "reason.{}",
                d["reason"].as_str().unwrap_or("none")
            ))
        };
        out += &format!("Display {}  {size}  {what}", i + 1);
        if !state.is_empty() {
            out += &format!("  {state}");
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_reads_like_a_sentence() {
        let v = serde_json::json!({
            "pid": 42, "version": "0.0.7", "manual": null,
            "displays": [
                { "rect": [0, 0, 1920, 1080], "wallpaper": "sea", "title": "Sea at dusk", "kind": "video", "reason": "none", "error": null },
                { "rect": [1920, 0, 4480, 1440], "wallpaper": "orbit", "title": "Orbit", "kind": "web", "reason": "covered", "error": null },
                { "rect": [-1920, 0, 0, 1080], "wallpaper": null, "title": null, "kind": null, "reason": "none", "error": null }
            ]
        });
        let t = status_text(&v, |k| format!("<{k}>"));
        assert_eq!(
            t,
            "Sarab 0.0.7 (pid 42)\n\
             Display 1  1920x1080  Sea at dusk (video)  <reason.none>\n\
             Display 2  2560x1440  Orbit (web)  <reason.covered>\n\
             Display 3  1920x1080  no wallpaper\n"
        );
        let paused =
            serde_json::json!({ "pid": 1, "version": "0.0.7", "manual": true, "displays": [] });
        assert_eq!(
            status_text(&paused, |k| k.into()),
            "Sarab 0.0.7 (pid 1), paused by you\n"
        );
    }
}
