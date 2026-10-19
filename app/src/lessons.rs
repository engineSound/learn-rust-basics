//! Reads lessons/NN-slug/lesson.md (format: docs/SHARED-FORMAT.md).

use crate::json::Json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Lesson {
    pub slug: String,
    pub dir: PathBuf,
    pub meta: Vec<(String, Json)>,
    pub body: String,
    pub file: String,
    pub starter: String,
}

impl Lesson {
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.meta.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

pub fn valid_slug(slug: &str) -> bool {
    let mut chars = slug.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse the small YAML subset used in lesson front matter.
pub fn parse_front_matter(text: &str) -> (Vec<(String, Json)>, String) {
    let text = text.replace("\r\n", "\n");
    let Some(rest) = text.strip_prefix("---\n") else {
        return (Vec::new(), text);
    };
    let Some(end) = rest.find("\n---") else {
        return (Vec::new(), text);
    };
    let mut meta = Vec::new();
    for raw in rest[..end].lines() {
        let line = match raw.find(" #") {
            Some(i) => &raw[..i],
            None => raw,
        }
        .trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                meta.push((key.to_string(), parse_value(value.trim())));
            }
        }
    }
    let body = rest[end + 4..].trim_start_matches('-').trim_start_matches('\n').to_string();
    (meta, body)
}

fn parse_value(v: &str) -> Json {
    if v.starts_with('[') && v.ends_with(']') {
        let inner = v[1..v.len() - 1].trim();
        if inner.is_empty() {
            return Json::Arr(Vec::new());
        }
        return Json::Arr(inner.split(',').map(|s| parse_value(s.trim())).collect());
    }
    if let Ok(n) = v.parse::<f64>() {
        return Json::Num(n);
    }
    let unquoted = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(v);
    Json::str(unquoted)
}

pub fn list(lessons_dir: &Path) -> Vec<Lesson> {
    let Ok(entries) = fs::read_dir(lessons_dir) else {
        return Vec::new();
    };
    let mut slugs: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("lesson.md").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|s| valid_slug(s))
        .collect();
    slugs.sort();
    slugs.iter().filter_map(|s| load(lessons_dir, s).ok().flatten()).collect()
}

/// Load one lesson. Ok(None) if there's no such lesson; Err if it's malformed.
pub fn load(lessons_dir: &Path, slug: &str) -> Result<Option<Lesson>, String> {
    if !valid_slug(slug) {
        return Ok(None);
    }
    let dir = lessons_dir.join(slug);
    let Ok(text) = fs::read_to_string(dir.join("lesson.md")) else {
        return Ok(None);
    };
    let (meta, body) = parse_front_matter(&text);
    let file = match meta.iter().find(|(k, _)| k == "edit") {
        Some((_, Json::Str(f))) if !f.contains("..") && !f.starts_with('/') => f.clone(),
        _ => return Err(format!("lesson {slug}: front matter needs an \"edit:\" file inside the lesson folder")),
    };
    let starter = fs::read_to_string(dir.join(&file)).map_err(|e| format!("lesson {slug}: can't read {file}: {e}"))?;
    Ok(Some(Lesson { slug: slug.to_string(), dir, meta, body, file, starter }))
}

/// Everything that would make a ```visual spec (docs/SHARED-FORMAT.md, section 1) show nonsense,
/// replayed the way visual.js plays it; empty means it's fine. Used by the lesson tests and to
/// check visuals the tutor writes before the learner sees them.
pub fn visual_problems(spec: &Json) -> Vec<String> {
    let str_of = |j: &Json, k: &str| j.get(k).and_then(Json::as_str).unwrap_or("").to_string();
    let scenarios = spec.get("scenarios").map(Json::as_array).unwrap_or(&[]);
    if scenarios.is_empty() {
        return vec!["a visual needs scenarios".into()];
    }
    let top_regions = spec.get("regions").map(Json::as_array).unwrap_or(&[]);
    let mut problems = Vec::new();
    for sc in scenarios {
        let name = str_of(sc, "name");
        let place = if name.is_empty() { "scenario".to_string() } else { name };
        let regions: Vec<String> = sc.get("regions").map(Json::as_array).unwrap_or(top_regions).iter().map(|r| str_of(r, "id")).collect();
        let lines = str_of(sc, "code").split('\n').count() as f64;
        let steps = sc.get("steps").map(Json::as_array).unwrap_or(&[]);
        if regions.is_empty() {
            problems.push(format!("{place}: no regions"));
        }
        if steps.len() < 2 {
            problems.push(format!("{place}: needs at least 2 steps"));
            continue;
        }
        let mut boxes: Vec<(String, String)> = Vec::new(); // (id, region), in order
        let mut arrows: &[Json] = &[];
        for (i, step) in steps.iter().enumerate() {
            let at = format!("{place} step {}", i + 1);
            for id in step.get("drop").map(Json::as_array).unwrap_or(&[]).iter().filter_map(Json::as_str) {
                boxes.retain(|b| b.0 != id);
            }
            for set in step.get("set").map(Json::as_array).unwrap_or(&[]) {
                let id = str_of(set, "id");
                let region = set.get("in").and_then(Json::as_str).filter(|r| !r.is_empty()).map(String::from);
                match boxes.iter_mut().find(|b| b.0 == id) {
                    Some(b) => {
                        if let Some(r) = region {
                            b.1 = r;
                        }
                    }
                    None => boxes.push((id, region.unwrap_or_default())),
                }
            }
            for b in &boxes {
                if b.1.is_empty() {
                    problems.push(format!("{at}: new box {} needs \"in\"", b.0));
                } else if !regions.contains(&b.1) {
                    problems.push(format!("{at}: box {} is in unknown region {}", b.0, b.1));
                }
            }
            if let Some(a) = step.get("arrows") {
                arrows = a.as_array();
            }
            for a in arrows {
                let (from, to) = (str_of(a, "from"), str_of(a, "to"));
                if !boxes.iter().any(|b| b.0 == from) || !boxes.iter().any(|b| b.0 == to) {
                    problems.push(format!("{at}: arrow {from} → {to} points at a missing box"));
                }
            }
            let line_nums: Vec<f64> = match step.get("line") {
                Some(Json::Arr(items)) => items.iter().filter_map(Json::as_f64).collect(),
                Some(j) => j.as_f64().into_iter().collect(),
                None => vec![],
            };
            for n in line_nums {
                if !(n >= 1.0 && n <= lines) {
                    problems.push(format!("{at}: line {n} is outside the code ({lines} lines)"));
                }
            }
            if str_of(step, "say").is_empty() {
                problems.push(format!("{at}: needs \"say\""));
            }
        }
    }
    problems
}

#[cfg(test)]
const ANATOMY_KINDS: [&str; 7] = ["keyword", "name", "type", "macro", "literal", "symbol", "operator"];

/// Problems with an ```anatomy spec: each part must be found in the code, in order, and be explained.
#[cfg(test)]
pub fn anatomy_problems(spec: &Json) -> Vec<String> {
    let str_of = |j: &Json, k: &str| j.get(k).and_then(Json::as_str).unwrap_or("").to_string();
    let code = str_of(spec, "code");
    if code.is_empty() {
        return vec!["an anatomy needs \"code\"".into()];
    }
    let parts = spec.get("parts").map(Json::as_array).unwrap_or(&[]);
    if parts.is_empty() {
        return vec!["an anatomy needs \"parts\"".into()];
    }
    let mut problems = Vec::new();
    let mut at = 0;
    for (i, p) in parts.iter().enumerate() {
        let text = str_of(p, "text");
        let place = format!("part {} ({})", i + 1, Json::str(text.clone()).to_string());
        match code[at..].find(&text).filter(|_| !text.is_empty()) {
            Some(found) => at += found + text.len(),
            None => problems.push(format!("{place} is not in the code after the previous part")),
        }
        if str_of(p, "say").is_empty() {
            problems.push(format!("{place} needs \"say\""));
        }
        if !ANATOMY_KINDS.contains(&str_of(p, "kind").as_str()) {
            problems.push(format!("{place}: kind must be one of {}", ANATOMY_KINDS.join(", ")));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_strings_numbers_lists_comments() {
        let (meta, body) = parse_front_matter("---\nid: x-01  # comment\nminutes: 20\nconcepts: [a, b]\nprereqs: []\n---\n# Hi\n");
        assert_eq!(
            meta,
            vec![
                ("id".to_string(), Json::str("x-01")),
                ("minutes".to_string(), Json::Num(20.0)),
                ("concepts".to_string(), Json::Arr(vec![Json::str("a"), Json::str("b")])),
                ("prereqs".to_string(), Json::Arr(vec![])),
            ]
        );
        assert_eq!(body, "# Hi\n");
    }

    #[test]
    fn slugs_reject_path_tricks() {
        assert!(valid_slug("02-let-mut"));
        assert!(!valid_slug("../app"));
        assert!(!valid_slug(""));
        assert!(!valid_slug("Lesson"));
    }

    #[test]
    fn every_lesson_has_required_front_matter() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lessons");
        let lessons = list(&dir);
        assert!(!lessons.is_empty());
        for l in &lessons {
            for key in ["id", "track", "title", "minutes", "concepts", "edit"] {
                assert!(l.get(key).is_some(), "{} is missing {key}", l.slug);
            }
            assert_eq!(l.get("track"), Some(&Json::str("rust")));
            let kind = l.get("kind").and_then(Json::as_str).unwrap_or("core");
            // Learn first: every exercise explains before it tests (docs/SHARED-FORMAT.md, section 1).
            let has = |h: &str| l.body.lines().any(|line| line.trim_end() == h);
            assert!(has("## Learn the idea") || has("## Recall"), "{} needs a \"## Learn the idea\" (or \"## Recall\") section", l.slug);
            assert!(has("## Your turn"), "{} needs a \"## Your turn\" section", l.slug);
            if kind == "core" {
                assert!(has("## Hints"), "{} needs a \"## Hints\" section", l.slug);
            }
            assert!(["core", "drill", "stretch"].contains(&kind), "{}: unknown kind {kind}", l.slug);
            if kind != "core" {
                let parent = l.get("for");
                assert!(
                    lessons.iter().any(|o| o.get("id") == parent && o.get("kind").and_then(Json::as_str).unwrap_or("core") == "core"),
                    "{}: \"for\" must name a core lesson",
                    l.slug
                );
            }
        }
    }

    /// Every ```run example compiles and runs: the learner's first try must work.
    #[test]
    fn every_run_example_runs() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lessons");
        let data = std::env::temp_dir().join(format!("learn-run-test-{}", std::process::id()));
        let mut count = 0;
        for l in list(&dir) {
            for block in l.body.split("```run\n").skip(1) {
                let code = block.split("\n```").next().unwrap();
                let r = crate::runner::run_snippet(code, &data).unwrap();
                assert_eq!(r.get("ok").and_then(Json::as_bool), Some(true), "{}: the example doesn't run: {}", l.slug, r.to_string());
                count += 1;
            }
        }
        std::fs::remove_dir_all(&data).ok();
        assert!(count >= 10, "expected examples in the lessons, found {count}");
    }

    #[test]
    fn every_anatomy_block_points_at_real_code_in_order() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lessons");
        for l in list(&dir) {
            for block in l.body.split("```anatomy\n").skip(1) {
                let spec = crate::json::parse(block.split("\n```").next().unwrap()).unwrap_or_else(|e| panic!("{}: bad anatomy JSON: {e}", l.slug));
                assert_eq!(anatomy_problems(&spec), Vec::<String>::new(), "{}", l.slug);
            }
        }
        for l in list(&dir).iter().filter(|l| l.get("kind").and_then(Json::as_str).unwrap_or("core") == "core") {
            assert!(l.body.contains("```anatomy\n"), "{}: every core lesson takes its code apart in an ```anatomy block", l.slug);
        }
        let swapped = crate::json::parse(r#"{"code":"fn main() {}","parts":[{"text":"main","kind":"name","say":"x"},{"text":"fn","kind":"keyword","say":"y"}]}"#).unwrap();
        assert_eq!(anatomy_problems(&swapped), vec!["part 2 (\"fn\") is not in the code after the previous part".to_string()]);
    }

    /// Every ```predict answer is one of its options and exactly what the code prints.
    #[test]
    fn every_quick_check_answer_is_what_the_code_prints() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lessons");
        let data = std::env::temp_dir().join(format!("learn-predict-test-{}", std::process::id()));
        let flat = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut count = 0;
        for l in list(&dir) {
            for block in l.body.split("```predict\n").skip(1) {
                let block = block.split("\n```").next().unwrap();
                let (code, meta) = block.split_once("\n---\n").unwrap_or_else(|| panic!("{}: predict needs a --- line", l.slug));
                let field = |k: &str| meta.lines().find_map(|line| line.strip_prefix(k).and_then(|r| r.strip_prefix(':'))).unwrap_or("").trim().to_string();
                let answer = field("answer");
                assert!(field("options").split('|').any(|o| o.trim() == answer), "{}: the answer must be one of the options", l.slug);
                let r = crate::runner::run_snippet(code, &data).unwrap();
                if answer.starts_with("doesn't compile") || answer.starts_with("panics") {
                    // "doesn't compile (E0384)": it must fail, with that error code if one is named.
                    assert_eq!(r.get("ok").and_then(Json::as_bool), Some(false), "{}: the quick check says it fails, but it runs", l.slug);
                    for code in answer.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| w.len() == 5 && w.starts_with('E')) {
                        assert!(r.to_string().contains(code), "{}: expected {code}, got {}", l.slug, r.to_string());
                    }
                } else {
                    let out = r.get("output").and_then(Json::as_str).unwrap_or("").to_string();
                    assert_eq!(flat(&out), flat(&answer), "{}: the quick check claims different output", l.slug);
                }
                count += 1;
            }
        }
        std::fs::remove_dir_all(&data).ok();
        assert!(count >= 10, "expected quick checks in the lessons, found {count}");
    }

    /// Every ```visual block is well-formed, and its checks match what the compiler really does.
    #[test]
    fn every_visual_is_well_formed_and_checked() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lessons");
        let data = std::env::temp_dir().join(format!("learn-visual-test-{}", std::process::id()));
        let str_of = |j: &Json, k: &str| j.get(k).and_then(Json::as_str).unwrap_or("").to_string();
        for l in list(&dir) {
            for block in l.body.split("```visual\n").skip(1) {
                let spec = crate::json::parse(block.split("\n```").next().unwrap()).unwrap_or_else(|e| panic!("{}: bad visual JSON: {e}", l.slug));
                for sc in spec.get("scenarios").map(Json::as_array).unwrap_or(&[]) {
                    let place = format!("{} / {}", l.slug, str_of(sc, "name"));
                    let code = str_of(sc, "code");
                    let one = Json::obj(vec![("regions", spec.get("regions").cloned().unwrap_or(Json::Arr(vec![]))), ("scenarios", Json::Arr(vec![sc.clone()]))]);
                    assert_eq!(visual_problems(&one), Vec::<String>::new(), "{place}");
                    if let Some(check) = sc.get("check") {
                        let run_code = check.get("code").and_then(Json::as_str).map(String::from).unwrap_or(code.clone());
                        let full = format!("{}{}", check.get("prelude").and_then(Json::as_str).map(|p| format!("{p}\n")).unwrap_or_default(), run_code);
                        let r = crate::runner::run_snippet(&full, &data).unwrap();
                        let ok = r.get("ok").and_then(Json::as_bool) == Some(true);
                        if let Some(err) = check.get("error").and_then(Json::as_str) {
                            assert!(!ok && r.to_string().contains(err), "{place}: expected {err}, got {}", r.to_string());
                        } else {
                            let out = r.get("output").and_then(Json::as_str).unwrap_or("").trim().to_string();
                            assert_eq!(out, str_of(check, "output"), "{place}: the visual claims different output");
                        }
                    }
                }
            }
        }
        std::fs::remove_dir_all(&data).ok();
    }
}
