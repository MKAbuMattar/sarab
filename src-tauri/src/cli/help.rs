//! `sarab help` and `sarab <command> --help`, laid out like `aws help`:
//! NAME, SYNOPSIS, DESCRIPTION, OPTIONS and EXAMPLES.

struct Doc {
    name: &'static str,
    synopsis: &'static str,
    summary: &'static str,
    about: &'static str,
    options: &'static [(&'static str, &'static str)],
    examples: &'static [(&'static str, &'static str)],
}

const DISPLAY: (&str, &str) = (
    "--display N",
    "Only display N, counting from 0: Display 1 in `sarab status` is --display 0. Without it, every display.",
);

const DOCS: &[Doc] = &[
    Doc {
        name: "set",
        synopsis: "sarab set <file | folder | url | library id> [--display N]",
        summary: "Play a wallpaper",
        about: "Plays a video, GIF, picture, web page, web address, YouTube link, Sarab package folder, \
                Wallpaper Engine folder, or a wallpaper already in the library by its id. A file that is not in the library yet is added to it first. \
                AVI, WMV and MPEG videos are converted to MP4 when ffmpeg is installed.",
        options: &[DISPLAY],
        examples: &[
            ("Play a video on every display", "sarab set C:\\Wallpapers\\dunes.mp4"),
            ("Play a web address on the second display", "sarab set https://example.com --display 1"),
        ],
    },
    Doc {
        name: "close",
        synopsis: "sarab close [--display N]",
        summary: "Remove the wallpaper",
        about: "Closes the wallpaper and puts back the desktop picture that was there before.",
        options: &[DISPLAY],
        examples: &[("Clear the first display", "sarab close --display 0")],
    },
    Doc {
        name: "pause",
        synopsis: "sarab pause",
        summary: "Pause every wallpaper",
        about: "Pauses every wallpaper until you run `sarab resume` or `sarab play`, whatever the pause rules say.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "play",
        synopsis: "sarab play",
        summary: "Play, ignoring the pause rules",
        about: "Plays every wallpaper even when a pause rule would rest it, until you run `sarab resume`.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "resume",
        synopsis: "sarab resume",
        summary: "Go back to the pause rules",
        about: "Undoes `sarab pause` or `sarab play`: wallpapers play or rest by the pause rules again.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "toggle",
        synopsis: "sarab toggle",
        summary: "Pause, or go back to the pause rules",
        about: "Pauses every wallpaper, or, when they are paused, goes back to the pause rules. Handy for a hotkey.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "next",
        synopsis: "sarab next",
        summary: "Change to another wallpaper",
        about: "Plays another wallpaper from the library, from the category Sarab changes wallpapers through.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "volume",
        synopsis: "sarab volume <0-100>",
        summary: "Set the wallpaper volume",
        about: "Sets the volume of video and web wallpapers. 0 is muted.",
        options: &[],
        examples: &[("Mute every wallpaper", "sarab volume 0")],
    },
    Doc {
        name: "prop",
        synopsis: "sarab prop <key>=<value> [--display N]",
        summary: "Change a wallpaper setting",
        about: "Sets one value the wallpaper offers, as listed in its properties.json, on the wallpaper that plays.",
        options: &[DISPLAY],
        examples: &[("Set a color property", "sarab prop schemecolor=0.2,0.4,0.5")],
    },
    Doc {
        name: "import",
        synopsis: "sarab import <package.zip>",
        summary: "Add a Sarab package to the library",
        about: "Unpacks a Sarab package (a zip with sarab.json) into the library.",
        options: &[],
        examples: &[("Import a package", "sarab import C:\\Downloads\\rain.zip")],
    },
    Doc {
        name: "preset",
        synopsis: "sarab preset <id>",
        summary: "Download a built-in wallpaper",
        about: "Downloads one of the built-in 4K wallpapers into the library: nasa-spinning-earth-4k \
                or nasa-iss-earth-view-4k.",
        options: &[],
        examples: &[("Get the spinning Earth", "sarab preset nasa-spinning-earth-4k")],
    },
    Doc {
        name: "status",
        synopsis: "sarab status [--json]",
        summary: "Show what each display plays",
        about: "Prints Sarab's version, then one line per display: its size, its wallpaper, and whether \
                it plays or why it rests.",
        options: &[("--json", "Print everything Sarab knows as JSON, for scripts.")],
        examples: &[("Read the state from a script", "sarab status --json")],
    },
    Doc {
        name: "screenshot",
        synopsis: "sarab screenshot <file.png> [--display N]",
        summary: "Save a picture of a wallpaper",
        about: "Saves what a display's wallpaper shows as a PNG. Give a full path.",
        options: &[("--display N", "The display to capture, counting from 0: Display 1 in `sarab status` is --display 0. The first one by default.")],
        examples: &[("Capture the first display", "sarab screenshot C:\\shots\\desk.png")],
    },
    Doc {
        name: "settings",
        synopsis: "sarab settings export <file.json> | sarab settings import <file.json>",
        summary: "Save or load Sarab's settings",
        about: "export writes every setting to a JSON file; give a full path. import reads one back and \
                applies it at once. A library folder that does not exist on this PC is left as it is.",
        options: &[],
        examples: &[
            ("Back up your settings", "sarab settings export C:\\backup\\sarab-settings.json"),
            ("Load them on another PC", "sarab settings import D:\\sarab-settings.json"),
        ],
    },
    Doc {
        name: "ui",
        synopsis: "sarab ui",
        summary: "Open the Sarab window",
        about: "Opens the window, or brings it to the front.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "check-update",
        synopsis: "sarab check-update",
        summary: "Look for a new version",
        about: "Checks for a new version on the update channel set in Settings. Nothing is installed.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "install-update",
        synopsis: "sarab install-update",
        summary: "Install the new version",
        about: "Downloads the new version, checks its signature, installs it and restarts Sarab.",
        options: &[],
        examples: &[],
    },
    Doc {
        name: "quit",
        synopsis: "sarab quit",
        summary: "Close Sarab",
        about: "Closes every wallpaper and quits. When Sarab is set to keep the last frame, the desktop \
                keeps it as its picture.",
        options: &[],
        examples: &[],
    },
];

/// The text for `sarab help [command]`, or an error naming the unknown command.
pub fn help(topic: Option<&str>) -> Result<String, String> {
    let Some(name) = topic else {
        return Ok(overview());
    };
    let d = DOCS
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| format!("no command called {name}. Run `sarab help` for the list."))?;
    let mut out = format!(
        "NAME\n    {} - {}\n\nSYNOPSIS\n    {}\n\nDESCRIPTION\n{}\n",
        d.name,
        d.summary,
        d.synopsis,
        wrap(&squash(d.about), 4)
    );
    if !d.options.is_empty() {
        out += "\nOPTIONS\n";
        for (flag, what) in d.options {
            out += &format!("    {flag}\n{}\n", wrap(what, 8));
        }
    }
    if !d.examples.is_empty() {
        out += "\nEXAMPLES\n";
        for (what, cmd) in d.examples {
            out += &format!("    {what}:\n\n        {cmd}\n\n");
        }
    }
    Ok(out.trim_end().to_string() + "\n")
}

fn overview() -> String {
    let width = DOCS.iter().map(|d| d.name.len()).max().unwrap_or(0);
    let mut out = String::from(
        "NAME\n    sarab - animated wallpapers that rest when you work\n\n\
         SYNOPSIS\n    sarab <command> [options]\n    sarab help <command>\n    sarab --version\n\n\
         DESCRIPTION\n    Runs the commands below in the Sarab that is open, and starts it first if it is not.\n\
         \n    Without a command, sarab starts and opens its window.\n\nCOMMANDS\n",
    );
    for d in DOCS {
        out += &format!("    {:width$}  {}\n", d.name, d.summary);
    }
    out += "\nOPTIONS\n    --version\n        Print the versions of Sarab, the system and the web engine.\n";
    out += "\nSee `sarab help <command>` or `sarab <command> --help` for one command.\n";
    out
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Wraps text at 88 columns under an indent.
fn wrap(text: &str, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut lines = vec![];
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && indent + line.len() + 1 + word.len() > 88 {
            lines.push(format!("{pad}{line}"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line += word;
    }
    if !line.is_empty() {
        lines.push(format!("{pad}{line}"));
    }
    lines.join("\n")
}

/// `help`, `--help`, `-h`, `help <command>` and `<command> --help` all name a help topic.
pub fn wants_help(args: &[String]) -> Option<Option<&str>> {
    let first = args.first()?.as_str();
    if matches!(first, "help" | "--help" | "-h") {
        return Some(args.get(1).map(String::as_str));
    }
    args.iter()
        .any(|a| a == "--help" || a == "-h")
        .then_some(Some(first))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn every_command_has_help() {
        for d in DOCS {
            let text = help(Some(d.name)).unwrap();
            assert!(
                text.starts_with(&format!("NAME\n    {} - ", d.name)),
                "{text}"
            );
            assert!(text.contains("SYNOPSIS") && text.contains("DESCRIPTION"));
            assert!(text.lines().all(|l| l.chars().count() <= 100), "{text}");
            // Every documented command is one the parser knows.
            if !matches!(
                d.name,
                "set" | "prop" | "volume" | "import" | "preset" | "screenshot" | "settings"
            ) {
                assert!(
                    crate::cli::parse(&a(d.name)).unwrap().is_some(),
                    "{}",
                    d.name
                );
            }
        }
        assert!(help(None).unwrap().contains("COMMANDS"));
        assert!(help(Some("nope")).is_err());
    }

    #[test]
    fn help_topics() {
        assert_eq!(wants_help(&a("help")), Some(None));
        assert_eq!(wants_help(&a("--help")), Some(None));
        assert_eq!(wants_help(&a("help set")), Some(Some("set")));
        assert_eq!(wants_help(&a("set --help")), Some(Some("set")));
        assert_eq!(wants_help(&a("status -h")), Some(Some("status")));
        assert_eq!(wants_help(&a("set C:/a.mp4")), None);
        assert_eq!(wants_help(&[]), None);
    }
}
