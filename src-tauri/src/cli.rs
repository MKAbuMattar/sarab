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
    /// Force play, ignoring every pause rule, until `resume`.
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
    /// Write status.json, including what each page reports about itself.
    Status,
}

pub const USAGE: &str = "usage: sarab [set <file|folder|url> | close | pause | play | resume | toggle | prop <key>=<value> | volume <0-100> | next | import <zip> | ui | status | quit] [--display N]";

/// `args` excludes the program name. No args means "just start" (None).
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
            // Ignore unknown --flags: autostart and the OS may pass their own.
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
    }
}
