use std::fmt::Write as _;
use std::io::{self, BufRead, Write};
use std::path::Path;

use anstyle::{Ansi256Color, AnsiColor, Color, Style};

use crate::error::WrathError;
use crate::inspect::ProcessInfo;
use crate::process::{KillPlan, KillResult};

const ACCENT: Style = AnsiColor::Red.on_default().bold();
const TITLE: Style = Style::new().bold();
const RUNTIME: Style = AnsiColor::Cyan.on_default().bold();
const LABEL: Style = Style::new().dimmed();
const WARN: Style = AnsiColor::Yellow.on_default().bold();
const ORANGE: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(208))))
    .bold();

const FLAME_RED: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(196))))
    .bold();

/// Yellow core drawn on the orange body, so quadrant blocks stay rounded.
const CORE: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(220))))
    .bg_color(Some(Color::Ansi256(Ansi256Color(208))));

/// Fire drawn next to the final message.
#[rustfmt::skip]
const FLAME: [&str; 5] = [
    "    ▙",
    "  ▟▖▟█▖",
    " ▟█████",
    "▐██▗▖██▌",
    " ▀███▛▘",
];
/// Color of each cell of `FLAME`, same shape: R = red, O = orange, Y = yellow core.
#[rustfmt::skip]
const FLAME_COLORS: [&str; 5] = [
    "    R",
    "  RRRRR",
    " RROORR",
    "RROYYORR",
    " RRRRRR",
];
/// Flame rows the two parts of the final message are printed next to.
const FLAME_TEXT_ROWS: (usize, usize) = (3, 4);
/// Space between the flame and the message.
const FLAME_GAP: &str = "   ";

/// Longest command line displayed before truncation.
const MAX_COMMAND_LEN: usize = 110;
/// Number of distinct child names listed before summarizing.
const MAX_CHILD_GROUPS: usize = 4;

/// Pluralize "process" for the given count.
fn processes(count: usize) -> String {
    if count == 1 {
        "1 process".to_string()
    } else {
        format!("{count} processes")
    }
}

/// Format a byte count in human readable units.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Format a duration in seconds, keeping the two most significant units.
pub fn format_duration(secs: u64) -> String {
    let (days, hours, minutes, seconds) = (
        secs / 86_400,
        secs % 86_400 / 3600,
        secs % 3600 / 60,
        secs % 60,
    );

    match (days, hours, minutes) {
        (0, 0, 0) => format!("{seconds}s"),
        (0, 0, _) => format!("{minutes}m {seconds}s"),
        (0, _, _) => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h"),
    }
}

/// Shorten `text` to `max` characters, ending with an ellipsis if cut.
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}\u{2026}")
}

/// Summarize child names: "3 (node ×2, esbuild)".
pub fn summarize_children(children: &[String]) -> String {
    let mut groups: Vec<(&str, usize)> = Vec::new();
    for name in children {
        match groups.iter_mut().find(|(n, _)| n == name) {
            Some((_, count)) => *count += 1,
            None => groups.push((name, 1)),
        }
    }

    let mut listed: Vec<String> = groups
        .iter()
        .take(MAX_CHILD_GROUPS)
        .map(|(name, count)| match count {
            1 => (*name).to_string(),
            n => format!("{name} \u{d7}{n}"),
        })
        .collect();
    if groups.len() > MAX_CHILD_GROUPS {
        listed.push(format!("+{} more", groups.len() - MAX_CHILD_GROUPS));
    }

    format!("{} ({})", children.len(), listed.join(", "))
}

/// Append a "label  value" line to a card.
fn push_row(out: &mut String, label: &str, value: &str) {
    let _ = writeln!(out, "    {LABEL}{label:<10}{LABEL:#} {value}");
}

fn display_path(path: Option<&Path>) -> Option<String> {
    path.map(|p| p.display().to_string())
        .filter(|p| !p.is_empty())
}

/// Render the detailed card of a process.
pub fn render_card(info: &ProcessInfo) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "  {ACCENT}\u{25cf}{ACCENT:#} {TITLE}{}{TITLE:#}  {RUNTIME}{}{RUNTIME:#}  {LABEL}PID {}{LABEL:#}",
        info.name, info.runtime, info.pid
    );

    if let Some(entry) = &info.entry {
        let value = match &entry.package {
            Some(package) => format!("{} {LABEL}({package}){LABEL:#}", entry.value),
            None => entry.value.clone(),
        };
        push_row(&mut out, entry.kind, &value);
    }
    if !info.command.is_empty() {
        push_row(
            &mut out,
            "Command",
            &truncate(&info.command, MAX_COMMAND_LEN),
        );
    }
    if let Some(exe) = display_path(info.exe.as_deref()) {
        push_row(&mut out, "Path", &exe);
    }
    if let Some(cwd) = display_path(info.cwd.as_deref()) {
        push_row(&mut out, "Directory", &cwd);
    }
    if let Some(user) = &info.user {
        push_row(&mut out, "User", user);
    }
    push_row(
        &mut out,
        "Memory",
        &format!(
            "{}   {LABEL}Uptime{LABEL:#} {}",
            format_bytes(info.memory_bytes),
            format_duration(info.uptime_secs)
        ),
    );
    if !info.children.is_empty() {
        push_row(&mut out, "Children", &summarize_children(&info.children));
    }
    out
}

/// Render the header and the card of every targeted process.
pub fn render_plan(plan: &KillPlan, infos: &[ProcessInfo]) -> String {
    let header = match plan.port {
        Some(port) => format!("Port {TITLE}{port}{TITLE:#} is held by:"),
        None => format!(
            "Found {} matching {TITLE}'{}'{TITLE:#}:",
            processes(plan.roots.len()),
            plan.process_name
        ),
    };

    let mut out = format!("{ACCENT}[>]{ACCENT:#} {header}\n");
    for info in infos {
        out.push('\n');
        out.push_str(&render_card(info));
    }
    out
}

/// Style matching a letter of `FLAME_COLORS`, if any.
fn flame_style(letter: char) -> Option<Style> {
    match letter {
        'R' => Some(FLAME_RED),
        'O' => Some(ORANGE),
        'Y' => Some(CORE),
        _ => None,
    }
}

/// Render one row of the flame, coloring each cell from `FLAME_COLORS`.
fn render_flame_row(row: usize) -> String {
    let mut colors = FLAME_COLORS[row].chars();
    FLAME[row]
        .chars()
        .map(|ch| match colors.next().and_then(flame_style) {
            Some(style) => format!("{style}{ch}{style:#}"),
            None => ch.to_string(),
        })
        .collect()
}

/// Render the final success message, next to the flame.
pub fn render_result(result: &KillResult) -> String {
    let (headline_row, outcome_row) = FLAME_TEXT_ROWS;
    let row_width = |i: usize| FLAME[i].chars().count();
    let width = (0..FLAME.len()).map(row_width).max().unwrap_or(0);

    (0..FLAME.len())
        .map(|i| {
            let flame = render_flame_row(i);
            let pad = " ".repeat(width - row_width(i));

            if i == headline_row {
                format!(
                    "{flame}{pad}{FLAME_GAP}{TITLE}{}{TITLE:#}",
                    result.headline()
                )
            } else if i == outcome_row {
                format!("{flame}{pad}{FLAME_GAP}{}", result.outcome())
            } else {
                flame
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a note telling the user the confirmation was bypassed.
pub fn render_skipped(total: usize) -> String {
    format!(
        "{WARN}[!]{WARN:#} Confirmation skipped (--yes), obliterating {}.",
        processes(total)
    )
}

/// Render an error, or a softer note when the user aborted.
pub fn render_error(error: &WrathError) -> String {
    match error {
        WrathError::Aborted => format!("{WARN}[-]{WARN:#} {error}."),
        _ => format!("{ACCENT}[x]{ACCENT:#} {error}"),
    }
}

/// Ask a yes/no question, defaulting to "no".
pub fn confirm(
    input: &mut impl BufRead,
    output: &mut impl Write,
    total: usize,
) -> io::Result<bool> {
    write!(
        output,
        "\n{WARN}[?]{WARN:#} Obliterate {}? {LABEL}[y/N]{LABEL:#} ",
        processes(total)
    )?;
    output.flush()?;

    let mut answer = String::new();
    input.read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::inspect::{EntryPoint, Runtime};

    fn plain(text: &str) -> String {
        anstream::adapter::strip_str(text).to_string()
    }

    fn sample_info() -> ProcessInfo {
        ProcessInfo {
            pid: 12345,
            name: "node.exe".into(),
            runtime: Runtime::Node,
            entry: Some(EntryPoint {
                kind: "Script",
                value: "node_modules/vite/bin/vite.js".into(),
                package: Some("vite".into()),
            }),
            command: "node vite.js dev".into(),
            exe: Some(PathBuf::from("/usr/bin/node")),
            cwd: Some(PathBuf::from("/app")),
            user: Some("aymer".into()),
            memory_bytes: 88_290_000,
            uptime_secs: 7980,
            children: vec!["esbuild".into(), "cmd".into(), "esbuild".into()],
        }
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(88_290_000), "84.2 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
        assert_eq!(format_bytes(u64::MAX), "16777216.0 TB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(42), "42s");
        assert_eq!(format_duration(312), "5m 12s");
        assert_eq!(format_duration(7980), "2h 13m");
        assert_eq!(format_duration(3 * 86_400 + 4 * 3600 + 59), "3d 4h");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exactly10!", 10), "exactly10!");
        assert_eq!(truncate("this is too long", 8), "this is\u{2026}");
        assert_eq!(
            truncate("\u{e9}\u{e9}\u{e9}\u{e9}", 3),
            "\u{e9}\u{e9}\u{2026}"
        );
    }

    #[test]
    fn test_summarize_children_groups_and_caps() {
        let names: Vec<String> = ["a", "b", "a", "c", "d", "e", "f"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            summarize_children(&names),
            "7 (a \u{d7}2, b, c, d, +2 more)"
        );
    }

    #[test]
    fn test_render_card_contains_details() {
        let card = plain(&render_card(&sample_info()));
        assert!(card.contains("node.exe  Node.js  PID 12345"));
        assert!(card.contains("Script     node_modules/vite/bin/vite.js (vite)"));
        assert!(card.contains("Command    node vite.js dev"));
        assert!(card.contains("Directory  /app"));
        assert!(card.contains("User       aymer"));
        assert!(card.contains("84.2 MB   Uptime 2h 13m"));
        assert!(card.contains("Children   3 (esbuild \u{d7}2, cmd)"));
    }

    #[test]
    fn test_render_card_minimal() {
        let info = ProcessInfo {
            entry: None,
            command: String::new(),
            exe: None,
            cwd: Some(PathBuf::new()),
            user: None,
            children: vec![],
            ..sample_info()
        };
        let card = plain(&render_card(&info));
        assert!(!card.contains("Script"));
        assert!(!card.contains("Command"));
        assert!(!card.contains("Directory"));
        assert!(!card.contains("Children"));
        assert!(card.contains("Memory"));
    }

    #[test]
    fn test_render_plan_headers() {
        let mut plan = KillPlan {
            process_name: "node".into(),
            port: Some(3000),
            roots: vec![sysinfo::Pid::from_u32(1)],
            pids: vec![],
        };
        let by_port = plain(&render_plan(&plan, &[sample_info()]));
        assert!(by_port.starts_with("[>] Port 3000 is held by:"));
        assert!(by_port.contains("PID 12345"));

        plan.port = None;
        plan.roots.push(sysinfo::Pid::from_u32(2));
        let by_name = plain(&render_plan(&plan, &[]));
        assert_eq!(by_name, "[>] Found 2 processes matching 'node':\n");
    }

    #[test]
    fn test_render_result_draws_flame_next_to_message() {
        let result = KillResult {
            process_name: "node".into(),
            total_killed: 3,
            port: Some(3000),
        };
        let expected = [
            "    ▙",
            "  ▟▖▟█▖",
            " ▟█████",
            "▐██▗▖██▌   Wrath unleashed: Process 'node' (3 processes)",
            " ▀███▛▘    using port 3000 has been obliterated.",
        ]
        .join("\n");
        assert_eq!(plain(&render_result(&result)), expected);
    }

    #[test]
    fn test_flame_colors_match_flame_shape() {
        for (row, (drawing, colors)) in FLAME.iter().zip(FLAME_COLORS).enumerate() {
            let drawing: Vec<char> = drawing.chars().collect();
            let colors: Vec<char> = colors.chars().collect();
            assert_eq!(drawing.len(), colors.len(), "row {row}: widths differ");

            for (col, (cell, color)) in drawing.iter().zip(&colors).enumerate() {
                match cell {
                    ' ' => assert_eq!(*color, ' ', "row {row}, col {col}: color on empty cell"),
                    _ => assert!(
                        flame_style(*color).is_some(),
                        "row {row}, col {col}: expected R, O or Y, got {color:?}"
                    ),
                }
            }
        }
    }

    #[test]
    fn test_flame_style_letters() {
        assert_eq!(flame_style('R'), Some(FLAME_RED));
        assert_eq!(flame_style('O'), Some(ORANGE));
        assert_eq!(flame_style('Y'), Some(CORE));
        assert_eq!(flame_style(' '), None);
        assert_eq!(flame_style('x'), None);
    }

    #[test]
    fn test_render_skipped_and_errors() {
        assert!(plain(&render_skipped(1)).contains("obliterating 1 process."));
        assert_eq!(
            plain(&render_error(&WrathError::Aborted)),
            "[-] Aborted, nothing was killed."
        );
        assert_eq!(
            plain(&render_error(&WrathError::NoProcessOnPort(80))),
            "[x] No process found using port 80"
        );
    }

    #[test]
    fn test_confirm_answers() {
        for (answer, expected) in [
            ("y\n", true),
            ("YES\r\n", true),
            (" yes ", true),
            ("n\n", false),
            ("\n", false),
            ("", false),
            ("maybe\n", false),
        ] {
            let mut output = Vec::new();
            let result = confirm(&mut answer.as_bytes(), &mut output, 2).expect("io ok");
            assert_eq!(result, expected, "answer {answer:?}");
            assert!(
                plain(&String::from_utf8_lossy(&output)).contains("Obliterate 2 processes? [y/N]")
            );
        }
    }
}
