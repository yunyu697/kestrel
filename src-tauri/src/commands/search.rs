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
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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
        let subtitle = if c.description.is_empty() { c.target.clone() } else { c.description.clone() };
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
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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

pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
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
            if let Some(&idx) = store.by_path.get(*path) {
                let e = &store.entries[idx];
                hits.push(SearchHit {
                    id: e.path.clone(),
                    kind: if e.is_dir { "folder".into() } else { "file".into() },
                    title: e.name.clone(),
                    subtitle: e.dir.clone(),
                    path: Some(e.path.clone()),
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

    // 3. Applications.
    if settings.search_apps {
        let apps = st.apps.read();
        hits.extend(app_hits(&apps, &lower, 12, &settings.language));
    }

    // 4. Built-in commands and user-defined custom commands.
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

    // 5. Files and folders.
    let parsed = search::parse(raw);
    if !parsed.terms.is_empty() || parsed.exts.len() > 0 || parsed.kind.is_some() || parsed.in_path.is_some()
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
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen = std::collections::HashSet::new();
    hits.retain(|h| seen.insert(h.id.clone()));
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
        hist.sort_by(|a, b| b.at.cmp(&a.at));
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
