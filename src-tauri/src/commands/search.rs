//! The search command: merges files, apps, commands, web engines and the calculator.

use crate::apps::builtin_commands;
use crate::calc;
use crate::model::{SearchHit, SearchResponse};
use crate::search::{self, Scorer};
use crate::state::AppState;
use serde_json::json;
use std::sync::Arc;
use tauri::State;

fn command_hits(query_lower: &str, limit: usize, lang: &str) -> Vec<SearchHit> {
    let mut out = Vec::new();
    for cmd in builtin_commands(lang) {
        let mut best: Option<f64> = None;
        if let Some(s) = search::match_score(&cmd.name.to_lowercase(), query_lower) {
            best = Some(best.map_or(s, |b: f64| b.max(s)));
        }
        if let Some(s) = search::match_score(&cmd.keyword, query_lower) {
            best = Some(best.map_or(s, |b: f64| b.max(s)));
        }
        if let Some(s) = best {
            out.push(SearchHit {
                id: cmd.id.clone(),
                kind: "command".into(),
                title: cmd.name.clone(),
                subtitle: cmd.description.clone(),
                path: None,
                icon: None,
                score: s * 0.96 + 60.0,
                badge: Some(cmd.group.clone()),
                meta: Some(json!({ "glyph": cmd.icon, "commandId": cmd.id })),
            });
        }
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}

fn custom_command_hits(
    cmds: &[crate::model::CustomCommand],
    query_lower: &str,
    limit: usize,
    lang: &str,
) -> Vec<SearchHit> {
    let mut out = Vec::new();
    for c in cmds {
        if !c.enabled {
            continue;
        }
        let mut best: Option<f64> = None;
        if let Some(s) = search::match_score(&c.name.to_lowercase(), query_lower) {
            best = Some(best.map_or(s, |b: f64| b.max(s)));
        }
        if let Some(s) = search::match_score(&c.keyword.to_lowercase(), query_lower) {
            best = Some(best.map_or(s, |b: f64| b.max(s)));
        }
        let Some(s) = best else { continue };
        let id = format!("custom:{}", c.id);
        let subtitle = if c.description.is_empty() {
            c.target.clone()
        } else {
            c.description.clone()
        };
        out.push(SearchHit {
            id: id.clone(),
            kind: "command".into(),
            title: c.name.clone(),
            subtitle,
            path: None,
            icon: None,
            score: s * 0.96 + 80.0,
            badge: Some(crate::i18n::tr(lang, "badge.custom").into()),
            meta: Some(json!({ "glyph": "terminal", "commandId": id })),
        });
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}

fn app_hits(
    apps: &[crate::model::AppEntry],
    query_lower: &str,
    limit: usize,
    lang: &str,
) -> Vec<SearchHit> {
    let mut out: Vec<SearchHit> = Vec::new();
    for app in apps {
        let name_lower = app.name.to_lowercase();
        let Some(base) = search::match_score(&name_lower, query_lower) else {
            continue;
        };
        out.push(SearchHit {
            id: app.id.clone(),
            kind: "app".into(),
            title: app.name.clone(),
            subtitle: if app.source == "desktop" {
                crate::i18n::tr(lang, "badge.desktop").into()
            } else {
                crate::i18n::tr(lang, "badge.app").into()
            },
            path: Some(app.lnk.clone()),
            icon: None,
            score: base + 140.0,
            badge: None,
            meta: Some(json!({ "target": app.target, "source": app.source })),
        });
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}

fn web_hit(engines: &[crate::model::WebEngine], raw: &str, lang: &str) -> Option<SearchHit> {
    let (head, rest) = raw.split_once(' ')?;
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let engine = engines
        .iter()
        .find(|e| e.enabled && e.keyword.eq_ignore_ascii_case(head))?;
    let encoded = urlencode(rest);
    let url = engine.url.replace("{query}", &encoded);
    Some(SearchHit {
        id: format!("web:{}:{}", engine.id, rest),
        kind: "web".into(),
        title: crate::i18n::tr(lang, "web.search_title")
            .replace("{engine}", &engine.name)
            .replace("{q}", rest),
        subtitle: url.clone(),
        path: None,
        icon: None,
        score: 5000.0,
        badge: Some(engine.name.clone()),
        meta: Some(json!({ "url": url, "engine": engine.id })),
    })
}

/// One row for a typed command line.
///
/// It sits above everything else: a line the user typed to run is not a
/// suggestion, and burying it under file matches is what makes a launcher feel
/// like it "did not take" the input. The resolved executable goes into `path`
/// so the row gets the program's real icon.
fn run_hit(plan: crate::run::Plan, lang: &str) -> SearchHit {
    let ok = plan.target.is_some();
    let subtitle = match (&plan.detail, &plan.missing) {
        (Some(d), _) => d.clone(),
        (None, Some(name)) => crate::i18n::tr(lang, "run.not_found").replace("{name}", name),
        _ => String::new(),
    };
    SearchHit {
        id: format!("run:{}", plan.title.to_lowercase()),
        kind: "run".into(),
        title: plan.title.clone(),
        subtitle,
        path: plan.path,
        icon: None,
        score: if ok { 7500.0 } else { 1500.0 },
        badge: Some(crate::i18n::tr(lang, "badge.run").into()),
        meta: Some(json!({ "raw": plan.title, "ok": ok })),
    }
}

pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Result order: installed applications, then favorites, then anything the
/// user has opened before, then everything else. `hits` must already be sorted
/// by score: a stable pass over the tier keeps the strongest match on top
/// inside each group, so there is no second comparator to keep in sync.
fn rank_hits(
    hits: &mut [SearchHit],
    fav_paths: &std::collections::HashSet<String>,
    usage: &std::collections::HashMap<String, u32>,
) {
    let tier = |h: &SearchHit| -> u8 {
        // A typed command line outranks every suggestion below it: the user
        // has already said what to do, so nothing to guess.
        if h.kind == "run" {
            0
        } else if h.kind == "app" {
            1
        } else if h
            .path
            .as_deref()
            .is_some_and(|p| fav_paths.contains(&p.to_lowercase()))
        {
            2
        } else if h
            .path
            .as_deref()
            .is_some_and(|p| usage.get(p).copied().unwrap_or(0) > 0)
        {
            3
        } else {
            4
        }
    };
    hits.sort_by_key(tier);
}

fn do_search(st: &Arc<AppState>, query: &str, limit: Option<usize>) -> SearchResponse {
    let started = std::time::Instant::now();
    let settings = st.settings.read().clone();
    let max = limit.unwrap_or(settings.max_results as usize).clamp(5, 300);
    let raw = query.trim();
    let lower = raw.to_lowercase();

    let mut hits: Vec<SearchHit> = Vec::new();
    let mut total_matched = 0usize;
    let mut hint: Option<String> = None;

    if raw.is_empty() {
        // Suggestions: favorites, then most-used items, then a few commands.
        // Disabled favorites are skipped but keep their slot in the list.
        for fav in settings.favorites.iter().filter(|f| f.enabled).take(8) {
            hits.push(SearchHit {
                id: format!("fav:{}", fav.path),
                kind: "favorite".into(),
                title: fav.name.clone(),
                subtitle: fav.path.clone(),
                path: Some(fav.path.clone()),
                icon: None,
                score: 1000.0,
                badge: Some(crate::i18n::tr(&settings.language, "badge.favorite").into()),
                meta: None,
            });
        }
        let usage = st.usage.read();
        let mut recent: Vec<(&String, &u32)> = usage.iter().collect();
        recent.sort_by(|a, b| b.1.cmp(a.1));
        let store = st.index.store.read();
        for (path, count) in recent.iter().take(10) {
            if let Some(&idx) = store.by_path.get(path.as_str()) {
                let e = &store.entries[idx];
                hits.push(SearchHit {
                    id: e.path.to_string(),
                    kind: if e.is_dir {
                        "folder".into()
                    } else {
                        "file".into()
                    },
                    title: e.name.to_string(),
                    subtitle: e.dir().to_string(),
                    path: Some(e.path.to_string()),
                    icon: None,
                    score: 900.0 - (**count as f64) * 0.1,
                    badge: Some(crate::i18n::tr(&settings.language, "badge.frequent").into()),
                    meta: None,
                });
            }
        }
        drop(store);
        drop(usage);

        for cmd in builtin_commands(&settings.language).into_iter().take(4) {
            hits.push(SearchHit {
                id: cmd.id.clone(),
                kind: "command".into(),
                title: cmd.name.clone(),
                subtitle: cmd.description.clone(),
                path: None,
                icon: None,
                score: 800.0,
                badge: Some(cmd.group.clone()),
                meta: Some(json!({ "glyph": cmd.icon, "commandId": cmd.id })),
            });
        }

        if !st.index.ready.load(std::sync::atomic::Ordering::Relaxed) {
            hint = Some(crate::i18n::tr(&settings.language, "hint.indexing").into());
        }
        let took = started.elapsed().as_secs_f64() * 1000.0;
        return SearchResponse {
            hits,
            took_ms: took,
            total_matched,
            query: raw.to_string(),
            hint,
        };
    }

    // 1. Web engine keyword (e.g. "g rust").
    if let Some(w) = web_hit(&settings.web_engines, raw, &settings.language) {
        hits.push(w);
    }

    // 2. Calculator.
    if settings.calculator_enabled {
        if let Some(result) = calc::evaluate(raw) {
            hits.push(SearchHit {
                id: "calc".into(),
                kind: "calc".into(),
                title: result.display.clone(),
                subtitle: result.detail.clone(),
                path: None,
                icon: None,
                score: 6000.0,
                badge: Some(crate::i18n::tr(&settings.language, "badge.calc").into()),
                meta: Some(json!({ "value": result.value, "display": result.display })),
            });
        }
    }

    // 3. A typed command line: the Run dialog's whole job. Resolved before the
    //    file search so the row exists even when nothing is indexed yet.
    let run_plan = if settings.run_enabled {
        crate::run::plan(raw)
    } else {
        None
    };
    if let Some(plan) = &run_plan {
        // A bare name that already resolved to an installed application would
        // otherwise appear twice — once as the app, once as itself — and the
        // second row only adds a path nobody asked to see.
        let covered = plan
            .path
            .as_deref()
            .filter(|_| {
                matches!(
                    plan.target,
                    Some(crate::run::Target::Program { ref args, .. }) if args.is_empty()
                )
            })
            .is_some_and(|exe| {
                st.apps
                    .read()
                    .iter()
                    .any(|a| a.target.eq_ignore_ascii_case(exe))
            });
        if !covered {
            hits.push(run_hit(plan.clone(), &settings.language));
        }
    }

    // 4. Applications.
    if settings.search_apps {
        let apps = st.apps.read();
        hits.extend(app_hits(&apps, &lower, 12, &settings.language));
    }

    // 5. Built-in commands and user-defined custom commands.
    if settings.search_commands {
        hits.extend(command_hits(&lower, 8, &settings.language));
        if !settings.custom_commands.is_empty() {
            hits.extend(custom_command_hits(
                &settings.custom_commands,
                &lower,
                6,
                &settings.language,
            ));
        }
    }

    // 6. Files and folders.
    let parsed = search::parse(raw);
    if !parsed.terms.is_empty()
        || !parsed.exts.is_empty()
        || parsed.kind.is_some()
        || parsed.in_path.is_some()
    {
        let usage = st.usage.read();
        let scorer = Scorer {
            query: &parsed,
            usage: &usage,
            match_path: settings.match_path,
            pinyin: settings.pinyin_search,
            fuzzy: settings.fuzzy_search,
        };
        let store = st.index.store.read();
        let (file_hits, total) = search::search_entries(&store.entries, &scorer, max);
        total_matched = total;
        drop(store);
        drop(usage);
        hits.extend(file_hits);
        if !st.index.ready.load(std::sync::atomic::Ordering::Relaxed) {
            hint = Some(crate::i18n::tr(&settings.language, "hint.indexing_partial").into());
        }
    }

    // De-duplicate by id while keeping the highest score.
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut seen = std::collections::HashSet::new();
    hits.retain(|h| seen.insert(h.id.clone()));

    // Rank: apps, favorites, previously opened, everything else. The stable
    // tier pass preserves the score order created above inside each group.
    let fav_paths: std::collections::HashSet<String> = settings
        .favorites
        .iter()
        .filter(|f| f.enabled)
        .map(|f| f.path.to_lowercase())
        .collect();
    let usage = st.usage.read();
    rank_hits(&mut hits, &fav_paths, &usage);
    drop(usage);
    hits.truncate(max);

    let took = started.elapsed().as_secs_f64() * 1000.0;
    SearchResponse {
        hits,
        took_ms: took,
        total_matched,
        query: raw.to_string(),
        hint,
    }
}

#[tauri::command]
pub async fn search(
    state: State<'_, Arc<AppState>>,
    query: String,
    limit: Option<usize>,
) -> Result<SearchResponse, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || do_search(&st, &query, limit))
        .await
        .map_err(|e| e.to_string())
}

/// Record that a query was used (drives the history suggestions).
#[tauri::command]
pub fn record_query(state: State<'_, Arc<AppState>>, query: String) {
    let q = query.trim().to_string();
    if q.is_empty() || q.len() > 120 {
        return;
    }
    let settings = state.settings.read().clone();
    if !settings.history_enabled {
        return;
    }
    {
        let mut hist = state.history.write();
        if let Some(existing) = hist.iter_mut().find(|h| h.query == q) {
            existing.count += 1;
            existing.at = chrono::Local::now().timestamp();
        } else {
            hist.push(crate::model::HistoryEntry {
                query: q,
                at: chrono::Local::now().timestamp(),
                count: 1,
            });
        }
        hist.sort_by_key(|h| std::cmp::Reverse(h.at));
        hist.truncate(settings.history_limit as usize);
    }
    state.save_history();
}

#[tauri::command]
pub fn history_list(state: State<'_, Arc<AppState>>) -> Vec<crate::model::HistoryEntry> {
    state.history.read().clone()
}

#[tauri::command]
pub fn history_clear(state: State<'_, Arc<AppState>>) {
    state.history.write().clear();
    state.save_history();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn hit(id: &str, kind: &str, path: Option<&str>, score: f64) -> SearchHit {
        SearchHit {
            id: id.into(),
            kind: kind.into(),
            title: id.into(),
            subtitle: String::new(),
            path: path.map(str::to_string),
            icon: None,
            score,
            badge: None,
            meta: None,
        }
    }

    fn ids(hits: &[SearchHit]) -> Vec<&str> {
        hits.iter().map(|h| h.id.as_str()).collect()
    }

    fn by_score(hits: &mut [SearchHit]) {
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    }

    #[test]
    fn apps_rank_above_a_better_file_match() {
        let mut hits = vec![
            hit("file", "file", Some(r"C:/docs/we.pdf"), 1000.0),
            hit("app", "app", Some(r"C:/apps/we.lnk"), 300.0),
        ];
        by_score(&mut hits);
        rank_hits(&mut hits, &HashSet::new(), &HashMap::new());
        assert_eq!(ids(&hits), vec!["app", "file"]);
    }

    #[test]
    fn favorite_then_used_then_plain_keeps_score_inside_a_tier() {
        let mut hits = vec![
            hit("plain-high", "file", Some(r"C:/d/plain.txt"), 900.0),
            hit("used-low", "file", Some(r"C:/d/used.txt"), 700.0),
            hit("fav-low", "file", Some(r"C:/d/fav.txt"), 600.0),
            hit("plain-low", "file", Some(r"C:/d/other.txt"), 500.0),
            hit("used-high", "file", Some(r"C:/d/used2.txt"), 800.0),
        ];
        by_score(&mut hits);
        let favs: HashSet<String> = [r"c:/d/fav.txt".to_string()].into_iter().collect();
        let usage: HashMap<String, u32> = [
            (r"C:/d/used.txt".to_string(), 3),
            (r"C:/d/used2.txt".to_string(), 1),
        ]
        .into_iter()
        .collect();
        rank_hits(&mut hits, &favs, &usage);
        assert_eq!(
            ids(&hits),
            vec![
                "fav-low",
                "used-high",
                "used-low",
                "plain-high",
                "plain-low"
            ]
        );
    }

    #[test]
    fn a_run_hit_carries_the_command_and_the_executable_it_landed_on() {
        let plan = crate::run::plan("cmd /k echo hi").expect("cmd 应被识别");
        let h = run_hit(plan, "zh");
        assert_eq!(h.kind, "run");
        assert_eq!(h.badge.as_deref(), Some("运行"));
        let meta = h.meta.expect("meta");
        assert_eq!(meta["raw"], "cmd /k echo hi");
        assert_eq!(meta["ok"], true);
        assert!(
            h.path
                .as_deref()
                .is_some_and(|p| p.to_lowercase().ends_with("cmd.exe")),
            "path 应指向 cmd.exe，实际 {:?}",
            h.path
        );
    }

    /// A name that cannot be resolved still gets a row when the input was
    /// unmistakably a command, and that row has to say why.
    #[test]
    fn an_unresolvable_command_explains_itself() {
        let plan = crate::run::plan(">definitely-not-installed-xyz").expect("显式调用");
        let h = run_hit(plan, "zh");
        assert_eq!(h.meta.as_ref().and_then(|m| m["ok"].as_bool()), Some(false));
        assert!(
            h.subtitle.contains("definitely-not-installed-xyz"),
            "{}",
            h.subtitle
        );
        assert!(h.subtitle.starts_with("Windows"), "{}", h.subtitle);
    }

    #[test]
    fn a_typed_command_outranks_an_application() {
        let mut hits = vec![
            hit("app", "app", Some(r"C:/apps/notepad.lnk"), 9000.0),
            hit("run", "run", Some(r"C:/Windows/notepad.exe"), 100.0),
        ];
        by_score(&mut hits);
        rank_hits(&mut hits, &HashSet::new(), &HashMap::new());
        assert_eq!(ids(&hits), vec!["run", "app"]);
    }

    #[test]
    fn favorite_paths_are_matched_case_insensitively() {
        let mut hits = vec![hit("fav", "file", Some(r"C:/Docs/Plan.docx"), 100.0)];
        let favs: HashSet<String> = [r"c:/docs/plan.docx".to_string()].into_iter().collect();
        rank_hits(&mut hits, &favs, &HashMap::new());
        assert_eq!(ids(&hits), vec!["fav"]);
    }

    #[test]
    fn commands_and_folders_stay_in_the_last_tier() {
        let mut hits = vec![
            hit("cmd", "command", None, 1200.0),
            hit("folder", "folder", Some(r"C:/d"), 900.0),
            hit("used", "file", Some(r"C:/d/used.txt"), 400.0),
        ];
        by_score(&mut hits);
        let usage: HashMap<String, u32> = [(r"C:/d/used.txt".to_string(), 1)].into_iter().collect();
        rank_hits(&mut hits, &HashSet::new(), &usage);
        assert_eq!(ids(&hits), vec!["used", "cmd", "folder"]);
    }
}
