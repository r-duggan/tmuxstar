use chrono::Local;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(name = "tmuxstar", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Git {
        #[arg(long)]
        path:Option<String>,
        #[arg(long, default_value="white")]
        label_fg: String,
        #[arg(long, default_value=" ")]
        icon: String,
    },
    Time {
        #[arg(long, default_value="%Y-%m-%d %I:%M%p")]
        format: String,
        #[arg(long, default_value="󰸗 ")]
        icon: String,
    },
}

fn print_time(format: &str, icon: &str) {
    let now = Local::now();
    let s = now.format(format).to_string();
    if icon.is_empty() {
        print!("{s}");
    } else {
        print!("{icon}{s}");
    }
}

fn git_ok(path: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", path])
        .args(args)
        .output()
        .ok()?;                    // could not spawn → None
    if !out.status.success() {
        return None;               // non-zero exit → None
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn is_repo(path: &str) -> bool {
    Command::new("git").args(["-C", path, "rev-parse", "--is-inside-working-tree"])
    .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn repo_root_name(path: &str) -> Option<String> {
    let root = git_ok(path, &["rev-parse", "--show-toplevel"])?;
    Some(Path::new(&root).file_name()?.to_string_lossy().to_string())
}

fn head_name(path: &str) -> Option<String> {
    if let Some(mut h) = git_ok(path, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        if h == "HEAD" {
            if let Some(d) = git_ok(path, &["describe", "--contains", "--all", "HEAD"]) {
                h = d;
            }
        }
        if !h.is_empty() {
            return Some(h);
        }
    }

    None
}

fn repo_state(path: &str) -> &'static str {
    // Run: git -C <path> status --porcelain
    let out = match std::process::Command::new("git")
        .args(["-C", path, "status", "--porcelain"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return "clean", // if git can't run here, treat as clean/none
    };

    let s = String::from_utf8_lossy(&out.stdout);
    if s.lines().any(|l| matches!(l.get(0..2), Some("UU" | "AA" | "DD" | "AU" | "UD" | "UA" | "DU"))) {
        return "conflict";
    }
    if s.lines().any(|l| l.starts_with("??")) {
        return "untracked";
    }
    if s.lines().any(|l| l.chars().next().map(|c| "MRADC".contains(c)).unwrap_or(false)) {
        return "staged";
    }
    if s.lines().any(|l| l.chars().nth(1).map(|c| "MRADC D".contains(c)).unwrap_or(false)) {
        return "unstaged";
    }
    "clean"
}

fn state_color_fg(state: &str) -> &'static str {
    match state {
        "conflict" | "unstaged" => "#ff6b6b",
        "staged"                => "#f1fa8c",
        "untracked"             => "#bd93f9",
        "clean"                 => "#50fa7b",
        _                       => "white",
    }
}

fn tmux_fg(color: &str) -> String {
    format!("#[fg={}]", color)
}

fn print_git(path: &str, label_fg: &str, icon: &str) {
    if !is_repo(path) {
        return;
    }
    let Some(project) = repo_root_name(path) else { return; };
    let Some(branch)  = head_name(path)      else { return; };

    let state  = repo_state(path);
    let c_icon = state_color_fg(state); // hex like "#50fa7b"

    let out = format!(
        "{icon_col}{icon}{restore}{project}({branch})",
        icon_col = tmux_fg(c_icon),
        icon     = icon,
        restore  = tmux_fg(label_fg),
        project  = project,
        branch   = branch,
    );

    println!("{out}");
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Git { path, label_fg, icon } => {
            let p = path.unwrap_or_else(|| ".".into());
            print_git(&p, &label_fg, &icon);
        }
        Cmd::Time { format, icon } => {
            print_time(&format, &icon);
        }
    }
}

