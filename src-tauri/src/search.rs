//! Query parsing and ranking.

use crate::index::Entry;
use crate::model::SearchHit;
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct ParsedQuery {
    pub terms: Vec<String>,
    /// Per-term bitmap of required ASCII letters (0 = no pruning possible,
    /// e.g. non-ASCII terms) used to cheaply reject entries.
    pub term_masks: Vec<u32>,
    pub exts: Vec<String>,
    pub kind: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub in_path: Option<String>,
}

/// Bitmap of ASCII letters (bit 0 = 'a') required by a lowercase term.
fn term_letter_mask(term: &str) -> u32 {
    if !term.is_ascii() {
        return 0;
    }
    let mut m = 0u32;
    for &b in term.as_bytes() {
        if b.is_ascii_lowercase() {
            m |= 1 << (b - b'a');
        }
    }
    m
}

fn parse_size(v: &str) -> Option<u64> {
    let v = v.trim().to_lowercase();
    let (num, mult) = if let Some(n) = v.strip_suffix("kb").or_else(|| v.strip_suffix("k")) {
        (n, 1024u64)
    } else if let Some(n) = v.strip_suffix("mb").or_else(|| v.strip_suffix("m")) {
        (n, 1024 * 1024)
    } else if let Some(n) = v.strip_suffix("gb").or_else(|| v.strip_suffix("g")) {
        (n, 1024 * 1024 * 1024)
    } else if let Some(n) = v.strip_suffix('b') {
        (n, 1)
    } else {
        (v.as_str(), 1)
    };
    num.trim().parse::<f64>().ok().map(|n| (n * mult as f64) as u64)
}

fn parse_date(v: &str) -> Option<i64> {
    let v = v.trim().to_lowercase();
    let now = chrono::Local::now();
    let days = match v.as_str() {
        "today" | "今天" => Some(0),
        "yesterday" | "昨天" => Some(1),
        "week" | "本周" | "7d" => Some(7),
        "month" | "本月" | "30d" => Some(30),
        "year" | "今年" | "365d" => Some(365),
        _ => None,
    };
    if let Some(d) = days {
        let t = now - chrono::Duration::days(d);
        return Some(t.timestamp());
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&v, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0).map(|x| x.and_local_timezone(chrono::Local).unwrap().timestamp());
    }
    if let Ok(y) = v.parse::<i32>() {
        if (1990..=2100).contains(&y) {
            return chrono::NaiveDate::from_ymd_opt(y, 1, 1)
                .map(|x| x.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap().timestamp());
        }
    }
    None
}

pub fn parse(raw: &str) -> ParsedQuery {
    let mut q = ParsedQuery::default();
    for token in raw.split_whitespace() {
        let lower = token.to_lowercase();
        if let Some(v) = lower.strip_prefix("ext:") {
            for e in v.split(',') {
                let e = e.trim().trim_start_matches('.');
                if !e.is_empty() {
                    q.exts.push(e.to_string());
                }
            }
        } else if let Some(v) = lower.strip_prefix("type:") {
            q.kind = Some(v.to_string());
        } else if let Some(v) = lower.strip_prefix("size:") {
            if let Some(rest) = v.strip_prefix('>') {
                q.min_size = parse_size(rest);
            } else if let Some(rest) = v.strip_prefix('<') {
                q.max_size = parse_size(rest);
            } else if let Some((a, b)) = v.split_once("..").or_else(|| v.split_once('-')) {
                q.min_size = parse_size(a);
                q.max_size = parse_size(b);
            } else if let Some(n) = parse_size(v) {
                q.min_size = Some(n.saturating_sub(1024));
                q.max_size = Some(n + 1024);
            }
        } else if let Some(v) = lower.strip_prefix("dm:") {
            if let Some(rest) = v.strip_prefix('>') {
                q.after = parse_date(rest);
            } else if let Some(rest) = v.strip_prefix('<') {
                q.before = parse_date(rest);
            } else if let Some(t) = parse_date(v) {
                q.after = Some(t);
            }
        } else if let Some(v) = lower.strip_prefix("in:").or_else(|| lower.strip_prefix("path:")) {
            q.in_path = Some(v.trim_matches('"').to_string());
        } else {
            q.terms.push(lower);
        }
    }
    q.term_masks = q.terms.iter().map(|t| term_letter_mask(t)).collect();
    q
}

fn is_boundary(prev: u8) -> bool {
    matches!(prev, b' ' | b'_' | b'-' | b'.' | b'(' | b'[' | b'+' | b',' | b'&' | b'\\' | b'/')
}

/// Keep the highest of the current best and a candidate score.
fn merge_best(best: &mut Option<f64>, v: Option<f64>) {
    if let Some(v) = v {
        *best = Some(best.map_or(v, |b| b.max(v)));
    }
}

/// Case-insensitive substring search over raw bytes. ASCII letters are folded;
/// non-ASCII bytes (UTF-8 continuation / CJK) only match by exact equality,
/// which is safe because multi-byte UTF-8 never collides with ASCII letters.
fn find_ci(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if hay.len() < needle.len() {
        return None;
    }
    let f = needle[0].to_ascii_lowercase();
    let fu = needle[0].to_ascii_uppercase();
    let mut start = 0usize;
    while start + needle.len() <= hay.len() {
        let b = hay[start];
        if b == f || b == fu {
            let mut ok = true;
            for (i, &nc) in needle.iter().enumerate().skip(1) {
                let hb = hay[start + i];
                if hb != nc && hb.to_ascii_lowercase() != nc {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Some(start);
            }
        }
        start += 1;
    }
    None
}

/// Case-insensitive substring check without allocation.
fn contains_ci(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    // For non-ASCII needles the byte-exact scan is still correct.
    h.len() >= n.len() && find_ci(h, n).is_some()
}

/// Score how well `needle` (lowercase) matches `hay` (any case — matching is
/// ASCII-case-insensitive so callers never need to allocate a lowercase copy).
/// Higher is better.
///
/// `fuzzy` enables the subsequence fallback: without it only a real
/// substring matches. The setting used to be ignored, so "fuzzy search" was
/// always on.
pub fn match_score(hay: &str, needle: &str) -> Option<f64> {
    match_score_ex(hay, needle, true)
}

pub fn match_score_ex(hay: &str, needle: &str, fuzzy: bool) -> Option<f64> {
    if needle.is_empty() {
        return Some(0.0);
    }
    if hay.is_empty() {
        return None;
    }
    if hay.len() == needle.len() && hay.eq_ignore_ascii_case(needle) {
        return Some(1000.0);
    }
    let hay_len = hay.len() as f64;
    let hb = hay.as_bytes();
    let nb = needle.as_bytes();

    if let Some(pos) = find_ci(hb, nb) {
        let base = if pos == 0 {
            900.0
        } else if is_boundary(hb[pos - 1]) {
            830.0
        } else {
            730.0
        };
        return Some(base - pos.min(80) as f64 * 0.6 - (hay_len - needle.len() as f64) * 0.35);
    }

    // Non-ASCII (CJK) queries only match contiguously — pinyin covers the rest.
    if !needle.is_ascii() {
        return None;
    }
    if !fuzzy {
        return None;
    }

    // Subsequence match on raw bytes (ASCII needle cannot match inside a multi-byte char).
    if nb.len() > hb.len() {
        return None;
    }
    let mut hi = 0usize;
    let mut prev: Option<usize> = None;
    let mut score = 480.0f64;
    for &nc in nb {
        let nc = nc.to_ascii_lowercase();
        let mut found = None;
        while hi < hb.len() {
            let hbc = hb[hi].to_ascii_lowercase();
            if hbc == nc {
                found = Some(hi);
                hi += 1;
                break;
            }
            hi += 1;
        }
        let idx = found?;
        match prev {
            Some(p) if idx == p + 1 => score += 14.0,
            Some(p) => score -= ((idx - p - 1).min(12) as f64) * 2.5,
            None => {
                score -= (idx.min(24) as f64) * 3.0;
                if idx == 0 {
                    score += 45.0;
                } else if is_boundary(hb[idx - 1]) {
                    score += 28.0;
                }
            }
        }
        prev = Some(idx);
    }
    score -= hay_len.min(160.0) * 0.2;
    Some(score.max(1.0))
}

pub struct Scorer<'a> {
    pub query: &'a ParsedQuery,
    pub usage: &'a HashMap<String, u32>,
    pub match_path: bool,
    pub pinyin: bool,
    pub fuzzy: bool,
}

impl<'a> Scorer<'a> {
    pub fn score(&self, e: &Entry) -> Option<f64> {
        let q = self.query;
        if let Some(kind) = &q.kind {
            match kind.as_str() {
                "file" if e.is_dir => return None,
                "folder" | "dir" if !e.is_dir => return None,
                _ => {}
            }
        }
        if !q.exts.is_empty() && (e.is_dir || !q.exts.iter().any(|x| x.as_str() == &*e.ext)) {
            return None;
        }
        if let Some(min) = q.min_size {
            if e.size < min {
                return None;
            }
        }
        if let Some(max) = q.max_size {
            if e.size > max {
                return None;
            }
        }
        if let Some(after) = q.after {
            if e.mtime < after {
                return None;
            }
        }
        if let Some(before) = q.before {
            if e.mtime > before {
                return None;
            }
        }
        if let Some(p) = &q.in_path {
            if !contains_ci(e.dir(), p) && !contains_ci(&e.path, p) {
                return None;
            }
        }

        let mut total = 0.0f64;
        for (ti, term) in q.terms.iter().enumerate() {
            let tmask = q.term_masks.get(ti).copied().unwrap_or(0);
            let mut best: Option<f64> = None;
            // Letters required by the term must be present in name/pinyin for
            // those matches to be possible; otherwise skip straight to path
            // matching. One AND prunes most entries with zero string work.
            if tmask == 0 || (e.mask & tmask) == tmask {
                merge_best(&mut best, match_score_ex(e.lower(), term, self.fuzzy));
                if best.is_none() && self.pinyin && !e.py.is_empty() {
                    merge_best(&mut best, match_score_ex(&e.py, term, self.fuzzy).map(|s| s * 0.94));
                }
                if best.is_none() && self.pinyin && !e.pyf.is_empty() {
                    merge_best(&mut best, match_score_ex(&e.pyf, term, self.fuzzy).map(|s| s * 0.86));
                }
            }
            if best.is_none() && self.match_path {
                merge_best(&mut best, match_score_ex(&e.path, term, self.fuzzy).map(|s| s * 0.62));
            }
            total += best?;
        }

        let usage_boost = self.usage.get(&*e.path).copied().unwrap_or(0).min(24) as f64 * 3.2;
        let depth = e.path.matches('\\').count() as f64;
        let depth_penalty = depth.min(12.0) * 1.6;
        let dir_bonus = if e.is_dir { 8.0 } else { 0.0 };

        Some(total + usage_boost - depth_penalty + dir_bonus)
    }
}

pub fn kind_of(e: &Entry) -> &'static str {
    if e.is_dir {
        "folder"
    } else {
        "file"
    }
}

pub fn entry_to_hit(e: &Entry, score: f64) -> SearchHit {
    SearchHit {
        id: e.path.to_string(),
        kind: kind_of(e).to_string(),
        title: e.name.to_string(),
        subtitle: e.dir().to_string(),
        path: Some(e.path.to_string()),
        icon: None,
        score,
        badge: if e.is_dir { None } else { Some(e.ext.to_string()) },
        meta: None,
    }
}

pub fn search_entries(
    entries: &[Entry],
    scorer: &Scorer,
    limit: usize,
) -> (Vec<SearchHit>, usize) {
    let matched: Vec<(usize, f64)> = entries
        .par_iter()
        .enumerate()
        .filter_map(|(i, e)| scorer.score(e).map(|s| (i, s)))
        .collect();
    let total = matched.len();

    let mut best: Vec<(usize, f64)> = matched;
    if best.len() > limit * 4 {
        best.par_sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        best.truncate(limit * 4);
    }
    best.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let hits = best
        .into_iter()
        .take(limit)
        .map(|(i, s)| entry_to_hit(&entries[i], s))
        .collect();
    (hits, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// A tiny tree with the three name shapes that matter: an all-lowercase
    /// name (no lowercase copy is stored), a mixed-case one (a copy is), and a
    /// CJK one (pinyin fields exist).
    fn tree(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kestrel-search-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs")).expect("docs");
        std::fs::write(dir.join("README.TXT"), b"hello").expect("file");
        std::fs::write(dir.join("docs").join("plain.md"), b"hi").expect("file");
        std::fs::write(dir.join("docs").join("会议记录.txt"), b"hi").expect("file");
        dir
    }

    fn entries(dir: &Path) -> Vec<Entry> {
        ["README.TXT", r"docs\plain.md", r"docs\会议记录.txt", "docs"]
            .iter()
            .map(|p| crate::index::make_entry(&dir.join(p)).expect("entry"))
            .collect()
    }

    fn run(dir: &Path, raw: &str, match_path: bool) -> Vec<SearchHit> {
        let q = parse(raw);
        let usage = HashMap::new();
        let scorer = Scorer {
            query: &q,
            usage: &usage,
            match_path,
            pinyin: true,
            fuzzy: true,
        };
        let entries = entries(dir);
        search_entries(&entries, &scorer, 20).0
    }

    fn titles(hits: &[SearchHit]) -> Vec<&str> {
        hits.iter().map(|h| h.title.as_str()).collect()
    }

    #[test]
    fn matches_a_mixed_case_name_case_insensitively() {
        let dir = tree("upper");
        let hits = run(&dir, "readme", true);
        assert!(titles(&hits).contains(&"README.TXT"), "got {:?}", titles(&hits));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The lowercase copy is only stored when it differs from the name; this
    /// is the branch where it does not exist and the name has to be used.
    #[test]
    fn matches_a_name_that_is_already_lowercase() {
        let dir = tree("lower");
        let hits = run(&dir, "plain", true);
        assert!(titles(&hits).contains(&"plain.md"), "got {:?}", titles(&hits));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn matches_a_cjk_name_by_its_pinyin_initials() {
        let dir = tree("pinyin");
        let hits = run(&dir, "hyjl", true);
        assert!(titles(&hits).contains(&"会议记录.txt"), "got {:?}", titles(&hits));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `in:` reads the parent directory, which is now sliced out of the path
    /// rather than stored — the filter has to keep behaving identically.
    #[test]
    fn filters_by_parent_directory() {
        let dir = tree("inpath");
        let hits = run(&dir, "in:docs", true);
        let got = titles(&hits);
        assert!(got.contains(&"plain.md"), "got {got:?}");
        assert!(got.contains(&"会议记录.txt"), "got {got:?}");
        assert!(!got.contains(&"README.TXT"), "README is not under docs: {got:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn filters_by_extension() {
        let dir = tree("ext");
        let hits = run(&dir, "ext:md", true);
        let got = titles(&hits);
        assert_eq!(got, vec!["plain.md"], "got {got:?}");

        // Extensions are stored lowercased, so an uppercase name still matches.
        let hits = run(&dir, "ext:txt", true);
        assert!(titles(&hits).contains(&"README.TXT"), "got {:?}", titles(&hits));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hit_shows_its_parent_directory_as_the_subtitle() {
        let dir = tree("hit");
        let entries = entries(&dir);
        let e = entries.iter().find(|e| &*e.name == "plain.md").expect("entry");
        let hit = entry_to_hit(e, 1.0);
        assert_eq!(hit.title, "plain.md");
        assert_eq!(hit.subtitle, e.dir());
        assert!(hit.subtitle.ends_with("docs"), "subtitle was {:?}", hit.subtitle);
        assert_eq!(hit.path.as_deref(), Some(&*e.path));
        assert_eq!(hit.badge.as_deref(), Some("md"));
    }

    /// Run real queries against a real index file. Skipped unless
    /// `KESTREL_INDEX` points at one, because it needs ~700k entries to say
    /// anything useful.
    ///
    /// ```text
    /// KESTREL_INDEX=%APPDATA%\com.kestrel.launcher\index.bin cargo test --lib \
    ///   smoke_against_a_real_index -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "needs KESTREL_INDEX pointing at a real index.bin"]
    fn smoke_against_a_real_index() {
        let path = match std::env::var("KESTREL_INDEX") {
            Ok(p) => p,
            Err(_) => return,
        };
        let index = crate::index::Index::new();
        let t0 = std::time::Instant::now();
        assert!(crate::index::load(&index, Path::new(&path)).is_some(), "load failed");
        println!("loaded {} entries in {:?}", index.count(), t0.elapsed());

        let usage = HashMap::new();
        for raw in ["a", "report", "会议", "hyjl", "in:downloads", "ext:rs log", "设置"] {
            let q = parse(raw);
            let scorer = Scorer {
                query: &q,
                usage: &usage,
                match_path: true,
                pinyin: true,
                fuzzy: true,
            };
            let store = index.store.read();
            let t = std::time::Instant::now();
            let (hits, total) = search_entries(&store.entries, &scorer, 10);
            println!(
                "{raw:>16} -> {total} matched, top {} in {:?}: {:?}",
                hits.len(),
                t.elapsed(),
                hits.iter().take(3).map(|h| &h.title).collect::<Vec<_>>()
            );
        }
    }
}
