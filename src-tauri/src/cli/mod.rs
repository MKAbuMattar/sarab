pub mod help;
pub mod terminal;

#[derive(Debug, PartialEq)]
pub enum Command {
    Set {
        target: String,
        display: Option<usize>,
    },
    Close {
        display: Option<usize>,
    },
    Pause,
    Play,
    Resume,
    Toggle,
    Prop {
        key: String,
        value: String,
        display: Option<usize>,
    },
    Volume(u8),
    Next,
    Import(String),
    Ui,
    Quit,
    Status,
    Preset(String),
    CheckUpdate,
    InstallUpdate,
    Screenshot {
        path: String,
        display: Option<usize>,
    },
    SettingsExport(String),
    SettingsImport(String),
}

pub const USAGE: &str = "usage: sarab [help | --version | set <file|folder|url> | close | pause | play | resume | toggle | prop <key>=<value> | volume <0-100> | next | import <zip> | ui | status | preset <id> | check-update | install-update | screenshot <file.png> | settings export|import <file> | quit] [--display N]";

/// One line in the style of `aws --version`: name/version pairs for a bug report.
pub fn version_line() -> String {
    use crate::core::settings::{self, Settings};
    let s: Settings = settings::load(&settings::config_dir().join("settings.json"));
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        a => a,
    };
    let engine = if cfg!(windows) {
        "WebView2"
    } else {
        "WebKitGTK"
    };
    let webview = tauri::webview_version()
        .map(|v| format!(" {engine}/{v}"))
        .unwrap_or_default();
    format!(
        "sarab/{} {}{webview} exe/{arch} channel/{}",
        env!("CARGO_PKG_VERSION"),
        crate::os::platform::os_label(),
        s.update_channel
    )
}

/// Answers that need no running Sarab: `--version` and help.
pub fn answer(args: &[String]) -> Option<Result<String, String>> {
    if args.first().is_some_and(|a| a == "--version") {
        return Some(Ok(version_line()
            + "
"));
    }
    help::wants_help(args).map(help::help)
}

pub fn parse(args: &[String]) -> Result<Option<Command>, String> {
    let mut display = None;
    let mut rest = vec![];
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--display" {
            let n = it.next().ok_or("--display needs a number")?;
            display = Some(
                n.parse::<usize>()
                    .map_err(|_| format!("bad display: {n}"))?,
            );
        } else if !a.starts_with("--") || a.len() == 2 {
            rest.push(a.as_str());
        }
    }
    let arg = |i: usize| {
        rest.get(i)
            .map(|s| s.to_string())
            .ok_or_else(|| USAGE.to_string())
    };
    let cmd = match rest.first().copied() {
        None => return Ok(None),
        Some("set") => Command::Set {
            target: arg(1)?,
            display,
        },
        Some("close") => Command::Close { display },
        Some("pause") => Command::Pause,
        Some("play") => Command::Play,
        Some("resume") => Command::Resume,
        Some("toggle") => Command::Toggle,
        Some("prop") => {
            let kv = arg(1)?;
            let (k, v) = kv.split_once('=').ok_or("prop needs key=value")?;
            Command::Prop {
                key: k.into(),
                value: v.into(),
                display,
            }
        }
        Some("volume") => Command::Volume(
            arg(1)?
                .parse::<u8>()
                .ok()
                .filter(|v| *v <= 100)
                .ok_or("volume is 0-100")?,
        ),
        Some("next") => Command::Next,
        Some("import") => Command::Import(arg(1)?),
        Some("ui") => Command::Ui,
        Some("quit") => Command::Quit,
        Some("status") => Command::Status,
        Some("preset") => Command::Preset(arg(1)?),
        Some("check-update") => Command::CheckUpdate,
        Some("install-update") => Command::InstallUpdate,
        Some("screenshot") => Command::Screenshot {
            path: arg(1)?,
            display,
        },
        Some("settings") => match arg(1)?.as_str() {
            "export" => Command::SettingsExport(arg(2)?),
            "import" => Command::SettingsImport(arg(2)?),
            _ => return Err("settings needs export or import, then a file".into()),
        },
        Some(other) => return Err(format!("unknown command: {other}\n{USAGE}")),
    };
    Ok(Some(cmd))
}

#[cfg(test)]
mod tests {
    use super::{Command::*, *};

    fn p(s: &str) -> Result<Option<Command>, String> {
        parse(&s.split_whitespace().map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn cli_parse() {
        assert_eq!(p(""), Ok(None));
        assert_eq!(p("--autostart"), Ok(None));
        assert_eq!(
            p("set C:/a.mp4"),
            Ok(Some(Set {
                target: "C:/a.mp4".into(),
                display: None
            }))
        );
        assert_eq!(
            p("set x --display 1"),
            Ok(Some(Set {
                target: "x".into(),
                display: Some(1)
            }))
        );
        assert_eq!(p("--display 0 close"), Ok(Some(Close { display: Some(0) })));
        assert_eq!(
            p("prop speed=2.5"),
            Ok(Some(Prop {
                key: "speed".into(),
                value: "2.5".into(),
                display: None
            }))
        );
        assert_eq!(
            p("prop msg=a=b"),
            Ok(Some(Prop {
                key: "msg".into(),
                value: "a=b".into(),
                display: None
            }))
        );
        assert_eq!(p("volume 40"), Ok(Some(Volume(40))));
        assert!(p("volume 101").is_err());
        assert!(p("set").is_err());
        assert!(p("prop novalue").is_err());
        assert!(p("set x --display two").is_err());
        assert!(p("dance").is_err());
        assert_eq!(p("toggle"), Ok(Some(Toggle)));
        assert_eq!(p("status"), Ok(Some(Status)));
        assert_eq!(p("play"), Ok(Some(Play)));
        assert_eq!(p("preset nasa-x"), Ok(Some(Preset("nasa-x".into()))));
        assert!(p("preset").is_err());
        assert_eq!(p("check-update"), Ok(Some(CheckUpdate)));
        assert_eq!(
            p("screenshot C:/out.png --display 1"),
            Ok(Some(Screenshot {
                path: "C:/out.png".into(),
                display: Some(1)
            }))
        );
        assert!(p("screenshot").is_err());
        assert_eq!(
            p("settings export C:/s.json"),
            Ok(Some(SettingsExport("C:/s.json".into())))
        );
        assert_eq!(
            p("settings import C:/s.json"),
            Ok(Some(SettingsImport("C:/s.json".into())))
        );
        assert!(p("settings").is_err());
        assert!(p("settings swap C:/s.json").is_err());
        assert!(p("settings import").is_err());
    }
}
