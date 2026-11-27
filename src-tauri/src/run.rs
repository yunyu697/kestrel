//! The Windows "Run" dialog, as one row in the result list.
//!
//! What the dialog accepts, in the order it accepts it:
//!
//! * a `shell:` folder or namespace item (`shell:startup`, `shell:AppsFolder\…`),
//! * a URL or a registered protocol (`https://…`, `ms-settings:`, `mailto:`),
//! * a path — absolute, relative to the profile, `~`, `%VAR%`, UNC or a bare
//!   drive letter — which opens as a file or a folder,
//! * a program, with or without arguments: `notepad C:\a.txt`, `cmd /k ipconfig`.
//!   The name is looked up the way the dialog looks it up: `App Paths` first,
//!   then `PATH` with `PATHEXT`.
//!
//! and it remembers what was run in `RunMRU`, the same key the dialog uses, so
//! the two share their history instead of each keeping a private one.

use crate::win::reg::{self, Root};
use crate::win::shell;
use std::path::{Path, PathBuf};

const HISTORY_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\RunMRU";
const SHELL_FOLDERS_USER: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";
const SHELL_FOLDERS_MACHINE: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";
const APP_PATHS_USER: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";
const APP_PATHS_MACHINE: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
const APP_PATHS_WOW: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths";

/// How many entries `RunMRU` keeps. The dialog uses one value per letter of
/// the alphabet, so this is not a choice.
const MRU_CAP: usize = 26;

/// What to hand to the shell.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Program {
        exe: String,
        args: String,
        cwd: Option<String>,
    },
    Path(String),
    Url(String),
    /// A `shell:` address the shell has to resolve itself.
    Shell(String),
}

/// One row's worth of decision about a typed command.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Shown as the row title — normally what was typed.
    pub title: String,
    /// Where it resolves to, when that is known.
    pub detail: Option<String>,
    /// The name that could not be found; the caller turns this into a sentence.
    pub missing: Option<String>,
    pub target: Option<Target>,
    /// An executable or folder behind the row, used for its icon.
    pub path: Option<String>,
}

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(r"C:\"))
}

/// Expand `%VAR%` references the way the dialog does.
///
/// An unknown name is left alone: echoing `%FOO%` back is more useful than
/// silently deleting it, and it keeps `100%` from eating the rest of a path.
pub fn expand_env(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) if j > 0 => {
                let name = &after[..j];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Split `"quoted program" args` into the program and the rest of the line.
///
/// The remainder is kept verbatim: it is passed on as the shell's parameter
/// string, where its own quotes still mean something.
pub fn split_command(s: &str) -> (String, String) {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return (rest[..end].to_string(), rest[end + 1..].trim().to_string());
        }
        return (rest.to_string(), String::new());
    }
    match t.find(char::is_whitespace) {
        Some(i) => (t[..i].to_string(), t[i..].trim().to_string()),
        None => (t.to_string(), String::new()),
    }
}

/// The `scheme` of `scheme:rest`, if the prefix really is one.
///
/// A single letter is not: `C:\dir` would otherwise read as the protocol `c`.
fn scheme_of(s: &str) -> Option<&str> {
    let i = s.find(':')?;
    if i < 2 {
        return None;
    }
    let sch = &s[..i];
    let mut chars = sch.chars();
    if !chars.next()?.is_ascii_alphabetic() {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
        return None;
    }
    Some(sch)
}

fn is_drive_only(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

fn is_absolute_path(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
    {
        return true;
    }
    s.starts_with(r"\\") || s.starts_with('/')
}

/// Turn what was typed into an existing location, or `None`.
///
/// Relative input is resolved against the profile folder, which is where the
/// dialog itself starts.
pub fn existing_path(raw: &str) -> Option<String> {
    let expanded = expand_env(raw);
    let s = expanded.trim();
    // `"C:\Program Files\x.txt"` is one quoted path, not a program plus a flag.
    let s = if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].trim()
    } else {
        s
    };
    if s.is_empty() {
        return None;
    }
    let candidate = if is_drive_only(s) {
        PathBuf::from(format!("{s}\\"))
    } else if let Some(rest) = s.strip_prefix('~') {
        home().join(rest.trim_start_matches(['\\', '/']))
    } else if is_absolute_path(s) {
        PathBuf::from(s)
    } else {
        home().join(s)
    };
    if candidate.exists() {
        Some(candidate.to_string_lossy().into_owned())
    } else {
        None
    }
}

fn strip_quotes(s: &str) -> &str {
    s.trim().trim_matches('"').trim()
}

/// `App Paths` — the table the dialog consults before `PATH`, so `chrome` and
/// `msedge` work without their directory being searchable.
fn app_paths(name: &str) -> Option<(String, Option<String>)> {
    let mut names = vec![name.to_string()];
    if Path::new(name).extension().is_none() {
        names.push(format!("{name}.exe"));
    }
    let tables = [
        (Root::CurrentUser, APP_PATHS_USER),
        (Root::LocalMachine, APP_PATHS_MACHINE),
        (Root::LocalMachine, APP_PATHS_WOW),
    ];
    for (root, base) in tables {
        for n in &names {
            let key = format!("{base}\\{n}");
            let Some(exe) = reg::read_string(root, &key, "") else {
                continue;
            };
            let exe = expand_env(strip_quotes(&exe));
            if exe.is_empty() {
                continue;
            }
            // The entry survives an uninstall and then points at nothing; a row
            // that cannot start anything is worse than no row.
            if !Path::new(&exe).exists() {
                continue;
            }
            let cwd = reg::read_string(root, &key, "Path")
                .map(|c| expand_env(strip_quotes(&c)))
                .filter(|c| Path::new(c).is_dir());
            return Some((exe, cwd));
        }
    }
    None
}

fn path_exts() -> Vec<String> {
    std::env::var("PATHEXT")
        .ok()
        .map(|v| {
            v.split(';')
                .map(|e| e.trim().to_ascii_lowercase())
                .filter(|e| e.starts_with('.'))
                .collect::<Vec<_>>()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()])
}

fn search_path(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let has_ext = Path::new(name).extension().is_some();
    let exts = path_exts();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if has_ext {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        } else {
            for e in &exts {
                let p = dir.join(format!("{name}{e}"));
                if p.is_file() {
                    return Some(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    None
}

/// Whether the text after a program name looks like arguments rather than the
/// rest of a search phrase.
///
/// This is the one place the launcher cannot copy the dialog. The dialog only
/// ever runs what is typed, so `code review` starts VS Code on a file called
/// "review". A launcher also searches, and there "code review" is two words to
/// look for — treating it as a command would open an editor nobody asked for.
/// Arguments that carry a flag, a path, a drive, a variable or a number are
/// unambiguous; plain words are not, and `>` states the intent explicitly.
fn looks_like_arguments(args: &str) -> bool {
    args.is_empty()
        || args.starts_with('-')
        || args.starts_with('/')
        || args.starts_with('"')
        || args.starts_with('~')
        || args.starts_with('.')
        || args.contains('\\')
        || args.contains('/')
        || args.contains(':')
        || args.contains('%')
        || args.chars().any(|c| c.is_ascii_digit())
}

/// Extensions the shell starts as a program rather than opening as a document.
///
/// The difference matters for `Ctrl+Shift+Enter`: a path that is a program can
/// be elevated, a document cannot.
fn is_program_file(p: &str) -> bool {
    Path::new(p)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| {
            matches!(
                e.as_str(),
                "exe" | "com" | "bat" | "cmd" | "ps1" | "msi" | "msc" | "cpl"
            )
        })
}

/// Resolve a program name to something startable.
fn resolve_program(name: &str) -> Option<(String, Option<String>)> {
    if name.contains('\\') || name.contains('/') {
        return existing_path(name).map(|p| (p, None));
    }
    if let Some(hit) = app_paths(name) {
        return Some(hit);
    }
    search_path(name).map(|p| (p, None))
}

/// Resolve the name after `shell:` against the shell's own folder table.
///
/// The names are the registry value names of `Shell Folders` ("Startup",
/// "Common Startup", "Fonts", …), matched case-insensitively — that is exactly
/// the set the dialog accepts, including the ones with a space in them.
pub fn shell_folder(name: &str) -> Option<String> {
    let want = name.trim().trim_start_matches('\\');
    if want.is_empty() {
        return None;
    }
    for (root, key) in [
        (Root::CurrentUser, SHELL_FOLDERS_USER),
        (Root::LocalMachine, SHELL_FOLDERS_MACHINE),
    ] {
        for (k, v) in reg::values(root, key) {
            if k.eq_ignore_ascii_case(want) {
                let v = expand_env(&v);
                if Path::new(&v).exists() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// The commands the dialog would offer, most recent first.
///
/// Each value carries a trailing `\1` marker that the dialog writes; it is not
/// part of the command and must not come back as one.
pub fn history() -> Vec<String> {
    let order = reg::read_string(Root::CurrentUser, HISTORY_KEY, "MRUList").unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for ch in order.chars().take(MRU_CAP) {
        let Some(v) = reg::read_string(Root::CurrentUser, HISTORY_KEY, &ch.to_string()) else {
            continue;
        };
        let v = v.trim_end_matches("\\1").trim().to_string();
        if !v.is_empty() && !out.iter().any(|e| e.eq_ignore_ascii_case(&v)) {
            out.push(v);
        }
    }
    out
}

/// Newest first, no duplicate of the same command, capped.
fn push_mru(list: &mut Vec<String>, cmd: &str, cap: usize) {
    list.retain(|c| !c.eq_ignore_ascii_case(cmd));
    list.insert(0, cmd.to_string());
    list.truncate(cap);
}

fn write_mru(list: &[String]) {
    for i in 0..MRU_CAP {
        let name = ((b'a' + i as u8) as char).to_string();
        match list.get(i) {
            Some(v) => {
                reg::write_string(Root::CurrentUser, HISTORY_KEY, &name, &format!("{v}\\1"));
            }
            None => {
                reg::delete_value(Root::CurrentUser, HISTORY_KEY, &name);
            }
        }
    }
    let order: String = (0..list.len().min(MRU_CAP))
        .map(|i| (b'a' + i as u8) as char)
        .collect();
    reg::write_string(Root::CurrentUser, HISTORY_KEY, "MRUList", &order);
}

/// Record a command in the dialog's own history, so `Win+R` offers it next.
pub fn remember(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() || cmd.len() > 240 {
        return;
    }
    let mut list = history();
    push_mru(&mut list, cmd, MRU_CAP);
    write_mru(&list);
}

/// The most recent history entry extending `prefix` — the dialog's inline
/// completion. Only a longer entry counts: completing a command to itself is
/// not a suggestion.
pub fn complete(prefix: &str) -> Option<String> {
    let p = prefix.trim();
    if p.len() < 2 {
        return None;
    }
    let lower = p.to_ascii_lowercase();
    history_cached()
        .into_iter()
        .find(|h| h.len() > p.len() && h.to_ascii_lowercase().starts_with(&lower))
}

/// `history()` behind a short cache.
///
/// Completion runs on every keystroke that nothing else resolved, and reading
/// 27 registry values each time is a cost paid for a list that changes only
/// when a command is run. Three seconds is longer than a burst of typing and
/// shorter than anyone's memory of what they just ran.
type HistoryCache = parking_lot::Mutex<Option<(std::time::Instant, Vec<String>)>>;

fn history_cached() -> Vec<String> {
    use once_cell::sync::Lazy;
    use std::time::{Duration, Instant};
    static CACHE: Lazy<HistoryCache> = Lazy::new(|| parking_lot::Mutex::new(None));
    let mut slot = CACHE.lock();
    if let Some((at, list)) = slot.as_ref() {
        if at.elapsed() < Duration::from_secs(3) {
            return list.clone();
        }
    }
    let list = history();
    *slot = Some((Instant::now(), list.clone()));
    list
}

/// Input that is unambiguously meant as a command, so a failure to resolve it
/// is worth showing. A two-word search phrase is not one of these.
fn command_shaped(s: &str) -> bool {
    if s.starts_with('>') || s.starts_with('"') {
        return true;
    }
    if s.contains('\\') || s.contains('%') || s.starts_with("./") || s.starts_with("../") {
        return true;
    }
    if s.starts_with('~') {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    [
        ".exe", ".com", ".bat", ".cmd", ".ps1", ".msi", ".lnk", ".cpl", ".msc",
    ]
    .iter()
    .any(|e| lower.ends_with(e))
}

fn shell_plan(title: &str, full: &str, rest: &str) -> Plan {
    match shell_folder(rest) {
        Some(p) => Plan {
            title: title.to_string(),
            detail: Some(p.clone()),
            missing: None,
            target: Some(Target::Path(p.clone())),
            path: Some(p),
        },
        None => Plan {
            title: title.to_string(),
            detail: Some(full.to_string()),
            missing: None,
            target: Some(Target::Shell(full.to_string())),
            path: None,
        },
    }
}

/// Decide what a typed line should do. `None` means "this is not a command",
/// and the normal search results speak for themselves.
pub fn plan(raw: &str) -> Option<Plan> {
    plan_at(raw, 0)
}

fn plan_at(raw: &str, depth: u8) -> Option<Plan> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let explicit = trimmed.starts_with('>');
    let body = if explicit {
        trimmed[1..].trim_start()
    } else {
        trimmed
    };
    if body.is_empty() {
        return None;
    }
    let expanded = expand_env(body);

    // `shell:` before the protocol test: "shell" is syntactically a scheme.
    // `get` rather than a byte slice: the input may be CJK, and slicing at 6
    // would land inside a character.
    if expanded
        .get(..6)
        .is_some_and(|p| p.eq_ignore_ascii_case("shell:"))
    {
        return Some(shell_plan(body, &expanded, &expanded[6..]));
    }
    if expanded.starts_with("::{") {
        return Some(Plan {
            title: body.to_string(),
            detail: Some(expanded.clone()),
            missing: None,
            target: Some(Target::Shell(expanded)),
            path: None,
        });
    }

    if scheme_of(&expanded).is_some() {
        return Some(Plan {
            title: body.to_string(),
            detail: Some(expanded.clone()),
            missing: None,
            target: Some(Target::Url(expanded)),
            path: None,
        });
    }

    // A path first, taken whole: `C:\Program Files\x.txt` contains a space and
    // splitting it on that space is how the dialog fails on the same input.
    if let Some(p) = existing_path(&expanded) {
        return Some(if is_program_file(&p) {
            Plan {
                title: body.to_string(),
                detail: Some(p.clone()),
                missing: None,
                target: Some(Target::Program {
                    exe: p.clone(),
                    args: String::new(),
                    cwd: None,
                }),
                path: Some(p),
            }
        } else {
            Plan {
                title: body.to_string(),
                detail: Some(p.clone()),
                missing: None,
                target: Some(Target::Path(p.clone())),
                path: Some(p),
            }
        });
    }

    let (prog, args) = split_command(&expanded);
    if !prog.is_empty() && prog.is_ascii() && (looks_like_arguments(&args) || explicit) {
        if let Some((exe, cwd)) = resolve_program(&prog) {
            let detail = if args.is_empty() {
                exe.clone()
            } else {
                format!("{exe} {args}")
            };
            return Some(Plan {
                title: body.to_string(),
                detail: Some(detail),
                missing: None,
                target: Some(Target::Program {
                    exe: exe.clone(),
                    args,
                    cwd,
                }),
                path: Some(exe),
            });
        }
    }

    if explicit || command_shaped(body) {
        return Some(Plan {
            title: body.to_string(),
            detail: None,
            missing: Some(prog.clone()),
            target: None,
            path: None,
        });
    }

    // Last: the dialog's inline completion from its own history ("not" →
    // "notepad C:\notes.txt"). One level only — the completion itself is not
    // completed again.
    if depth == 0 {
        if let Some(hit) = complete(body) {
            if let Some(mut p) = plan_at(&hit, 1) {
                p.title = hit;
                return Some(p);
            }
        }
    }
    None
}

/// Run it. `admin` is what `Ctrl+Shift+Enter` asks for.
pub fn execute(target: &Target, admin: bool) -> bool {
    match target {
        Target::Program { exe, args, cwd } => {
            let a = if args.is_empty() {
                None
            } else {
                Some(args.as_str())
            };
            if admin {
                shell::run_as_admin_args(exe, a, cwd.as_deref())
            } else {
                shell::spawn_program_unelevated(exe, a, cwd.as_deref())
            }
        }
        // A document or a folder has no elevation to ask for; the request is
        // honoured only where it means something.
        Target::Path(p) => shell::open_path_unelevated(p),
        Target::Url(u) => shell::open_url(u),
        Target::Shell(s) => shell::open_shell_target(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_vars_expand_and_unknown_ones_survive() {
        assert_eq!(expand_env("no vars here"), "no vars here");
        let windir = std::env::var("WINDIR").expect("WINDIR on every Windows");
        assert_eq!(
            expand_env(r"%WINDIR%\system32"),
            format!(r"{windir}\system32")
        );
        assert_eq!(expand_env("%KESTREL_NO_SUCH_VAR%"), "%KESTREL_NO_SUCH_VAR%");
        // A stray '%' must not swallow the rest of the line.
        assert_eq!(expand_env(r"100% done"), r"100% done");
    }

    #[test]
    fn a_quoted_program_keeps_its_arguments_verbatim() {
        assert_eq!(
            split_command(r#""C:\Program Files\App\app.exe" --flag "a b""#),
            (
                r"C:\Program Files\App\app.exe".to_string(),
                r#"--flag "a b""#.to_string()
            )
        );
        assert_eq!(
            split_command(r"notepad C:\a.txt"),
            ("notepad".to_string(), r"C:\a.txt".to_string())
        );
        assert_eq!(split_command("calc"), ("calc".to_string(), String::new()));
    }

    #[test]
    fn a_drive_letter_is_not_a_protocol() {
        assert_eq!(scheme_of(r"C:\Windows"), None);
        assert_eq!(scheme_of(r"D:"), None);
        assert_eq!(scheme_of("https://example.com"), Some("https"));
        assert_eq!(scheme_of("ms-settings:display"), Some("ms-settings"));
        assert_eq!(scheme_of("mailto:a@b.c"), Some("mailto"));
    }

    #[test]
    fn only_real_commands_are_shaped_like_one() {
        // An ordinary two-word search must not be offered as a failed command.
        assert!(!command_shaped("annual report"));
        assert!(command_shaped(r"notepad C:\a.txt"));
        assert!(command_shaped("> whatever"));
        assert!(command_shaped("%TEMP%"));
        assert!(command_shaped("setup.exe"));
    }

    /// The dialog starts relative paths in the profile folder.
    #[test]
    fn a_relative_path_resolves_against_the_profile() {
        let home = home();
        let dir = home.join("kestrel-run-test").join("nested");
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(
            existing_path(r"kestrel-run-test\nested").map(|p| p.to_lowercase()),
            Some(dir.to_string_lossy().to_lowercase())
        );
        let _ = std::fs::remove_dir_all(home.join("kestrel-run-test"));
    }

    #[test]
    fn a_bare_drive_letter_means_its_root() {
        let got = existing_path("C:").expect("C: exists");
        assert!(got.to_lowercase().starts_with("c:\\"), "got {got}");
    }

    /// `cmd.exe` is in `System32`, which is on `PATH` on every Windows install.
    #[test]
    fn a_bare_name_is_found_on_the_path() {
        let (exe, _) = resolve_program("cmd").expect("cmd 应在 PATH 上");
        assert!(
            exe.to_ascii_lowercase().ends_with(".exe"),
            "找不到可执行文件: {exe}"
        );
        assert!(Path::new(&exe).is_file(), "解析结果不是文件: {exe}");
    }

    #[test]
    fn shell_names_resolve_through_the_folder_table() {
        let startup = shell_folder("startup").expect("Startup 文件夹应存在");
        assert!(Path::new(&startup).is_dir(), "not a folder: {startup}");
        // Case and the space in the registry name both have to be tolerated.
        assert!(shell_folder("Startup").is_some());
        assert!(shell_folder("Common Startup").is_some());
        assert!(shell_folder("no-such-shell-folder").is_none());
    }

    #[test]
    fn the_newest_command_goes_first_and_does_not_repeat() {
        let mut list = vec!["b".to_string(), "a".to_string()];
        push_mru(&mut list, "c", 26);
        assert_eq!(list, vec!["c", "b", "a"]);
        push_mru(&mut list, "a", 26);
        assert_eq!(list, vec!["a", "c", "b"]);
    }

    #[test]
    fn history_is_capped() {
        let mut list: Vec<String> = Vec::new();
        for i in 0..40 {
            push_mru(&mut list, &format!("cmd {i}"), MRU_CAP);
        }
        assert_eq!(list.len(), MRU_CAP);
        assert_eq!(list[0], "cmd 39");
    }

    #[test]
    fn a_shell_command_becomes_its_folder() {
        let p = plan("shell:startup").expect("shell:startup 应被识别");
        assert!(matches!(p.target, Some(Target::Path(_))));
        assert!(p
            .detail
            .as_deref()
            .is_some_and(|d| d.to_lowercase().contains("startup")));
    }

    #[test]
    fn a_program_with_arguments_keeps_them_apart() {
        let p = plan(r"cmd /k echo hi").expect("cmd 应被识别");
        match p.target {
            Some(Target::Program { args, .. }) => assert_eq!(args, "/k echo hi"),
            other => panic!("不是程序: {other:?}"),
        }
    }

    #[test]
    fn an_unknown_command_is_reported_only_when_it_looks_like_one() {
        assert!(plan("annual report").is_none());
        let p = plan(r">no-such-program-xyz").expect("显式调用必须给出行");
        assert!(p.target.is_none());
        assert_eq!(p.missing.as_deref(), Some("no-such-program-xyz"));
    }

    /// An executable typed by path has to be classified as a program, or
    /// `Ctrl+Shift+Enter` would silently do nothing on it.
    #[test]
    fn an_executable_path_is_a_program_and_a_document_is_not() {
        let exe = std::env::temp_dir().join(format!("kestrel-run-{}.exe", std::process::id()));
        std::fs::write(&exe, b"not really an exe").expect("temp exe");
        let p = plan(&exe.to_string_lossy()).expect("exe 应被识别");
        assert!(
            matches!(p.target, Some(Target::Program { .. })),
            "got {:?}",
            p.target
        );

        let doc = std::env::temp_dir().join(format!("kestrel-run-{}.txt", std::process::id()));
        std::fs::write(&doc, b"hello").expect("temp txt");
        let p = plan(&doc.to_string_lossy()).expect("txt 应被识别");
        assert!(
            matches!(p.target, Some(Target::Path(_))),
            "got {:?}",
            p.target
        );
        let _ = std::fs::remove_file(&exe);
        let _ = std::fs::remove_file(&doc);
    }

    /// The dialog would start `code review` as VS Code with an argument, and a
    /// launcher must not: two plain words are a search, not a command.
    #[test]
    fn two_plain_words_are_not_a_command() {
        assert!(!looks_like_arguments("review"));
        assert!(looks_like_arguments("/k dir"));
        assert!(looks_like_arguments(r"C:\a.txt"));
        assert!(looks_like_arguments("8.8.8.8"));
        assert!(looks_like_arguments(""));
    }

    /// A round trip through the helper's argument encoding. Windows file names
    /// cannot contain a newline, which is what makes the three-field packing
    /// unambiguous.
    #[test]
    fn the_unelevated_helper_payload_survives_the_trip() {
        let token = crate::win::shell::encode_run_payload(
            r"C:\Program Files\App\app.exe",
            Some(r#"--flag "a b""#),
            Some(r"C:\work"),
        );
        let (program, args, cwd) = crate::win::shell::decode_run_payload(&token).expect("decode");
        assert_eq!(program, r"C:\Program Files\App\app.exe");
        assert_eq!(args.as_deref(), Some(r#"--flag "a b""#));
        assert_eq!(cwd.as_deref(), Some(r"C:\work"));

        let bare = crate::win::shell::encode_run_payload("calc.exe", None, None);
        let (_, args, cwd) = crate::win::shell::decode_run_payload(&bare).expect("decode");
        assert!(args.is_none() && cwd.is_none());
    }

    /// Resolution against the machine this is running on. Ignored by default —
    /// it reads the registry and the file system, so it is a manual check that
    /// prints where each shape of input lands:
    ///
    /// ```text
    /// cargo test --lib resolves_what_a_real_machine_has -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads the real registry and file system"]
    fn resolves_what_a_real_machine_has() {
        for raw in [
            "notepad",
            r"notepad C:\Windows\win.ini",
            "cmd /k ipconfig",
            "shell:startup",
            "shell:fonts",
            r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
            "%TEMP%",
            r"C:\Windows",
            "C:",
            "..",
            "~",
            "https://example.com",
            "ms-settings:",
            "calc",
            "code",
            "code review",
            ">no-such-program-xyz",
            r"\\localhost\c$",
            "mspaint",
        ] {
            match plan(raw) {
                Some(p) => println!(
                    "{raw:>46} => {:?}\n{:>46}    detail={:?} missing={:?}",
                    p.target, "", p.detail, p.missing
                ),
                None => println!("{raw:>46} => (not treated as a command)"),
            }
        }
    }

    #[test]
    fn a_url_stays_a_url() {
        let p = plan("https://example.com/a b").expect("URL 应被识别");
        assert!(matches!(p.target, Some(Target::Url(_))));
    }
}
