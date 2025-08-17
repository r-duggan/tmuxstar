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
    Left {
        #[arg(long)]
        path:Option<String>,
        #[arg(long, default_value="white")]
        label_fg: String,
        #[arg(long, default_value=" ")]
        icon: String,
    },
    Right,
}

fn print_time() {
    let now = Local::now();
    let formatted = now.format("%Y-%m-%d %I:%M:%S%p").to_string();
    println!("{}", formatted);
}

fn git_ok(path: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(["-C", path]).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    if s.is_empty() { None } else { Some(s) }
}

fn is_repo(path: &str) -> bool {
    Command::new("git").args(["-C", path, "rev-parse", "--is-inside-working-tree"])
    .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn repo_root_name(path: &str) -> Option<String> {
    let root = git_ok(path, &["rev-parse", "--show-top-level"])?;
    Some(Path::new(&root).file_name()?.to_string_lossy().to_string())
}

fn head_name(path: &str) -> Option<String> {
    if let Some(mut h) = git_ok(path, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        if h == "HEAD" || h.is_empty() {
            if let Some(descr) = git_ok(path, &["describe", "--contains", "--all", "HEAD"]) {
                h = descr;
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

fn print_left(path: &str, label_fg: &str, icon: &str) {
    if !is_repo(path) {
        print!("");
        return;
    }
    let Some(project) = repo_root_name(path) else { println!(""); return;};
    let Some(branch) = head_name(path) else { println!("");return;};

    let state = repo_state(path);
    let c_icon = state_color_fg(state);

    let out = format!(
    "{icon_color}{icon}{restore}{project}({branch})",
        icon_color = tmux_fg(c_icon),
        icon = icon,
        restore = tmux_fg(label_fg),
        project = project,
        branch = branch
    );

    println!("{out}")
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Left { path, label_fg, icon } => {
            let p = path.unwrap_or_else(|| ".".into());
            print_left(&p, &label_fg, &icon);
        }
        Cmd::Right => {
            print_time();
        }
    }
}

