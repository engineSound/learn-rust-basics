//! Learner profile and adaptive next step, derived from the event log.
//! Rules: docs/SHARED-FORMAT.md, section 5. learn-nodejs/app/profile.js implements the
//! same rules; docs/fixtures/ keeps the two in step.

use crate::history::iso_from_millis;
use crate::json::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const DAY: i64 = 86_400_000;
const SESSION_GAP: i64 = 30 * 60_000;
pub const REVIEW_DAYS: [i64; 5] = [1, 3, 7, 16, 35];
/// Solved on run 4 or later = struggled.
const STRUGGLE_RUNS: usize = 4;
/// This many runs on an unsolved lesson = offer a drill.
const WARMUP_RUNS: usize = 3;
const START: &str = "<!-- profile:start -->";
const END: &str = "<!-- profile:end -->";

pub struct LessonRef {
    pub id: String,
    pub title: String,
    pub concepts: Vec<String>,
    /// "core", "drill" or "stretch".
    pub kind: String,
    /// For drills and stretches: the core lesson they practise.
    pub parent: Option<String>,
}

#[derive(Default)]
struct LessonState {
    solved: bool,
    step: usize,
    due: i64,
    lapsed: bool,
    runs: usize,
    first_pass: Option<usize>,
    first_pass_ts: String,
    last_run_ts: String,
    feedback: Option<String>,
}

/// Milliseconds since the epoch for "YYYY-MM-DDTHH:MM:SS(.mmm)Z"; None if malformed.
pub fn parse_iso(ts: &str) -> Option<i64> {
    let b = ts.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| ts.get(r)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, s) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    let millis = match ts.get(19..20) {
        Some(".") => n(20..23)?,
        _ => 0,
    };
    // Days-from-civil (Howard Hinnant's algorithm).
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * DAY + ((h * 60 + mi) * 60 + s) * 1000 + millis)
}

fn str_of<'a>(e: &'a Json, key: &str) -> Option<&'a str> {
    e.get(key).and_then(Json::as_str)
}

fn day(ts: &str) -> &str {
    ts.get(..10).unwrap_or(ts)
}

fn opt_num(n: Option<usize>) -> Json {
    n.map(|v| Json::Num(v as f64)).unwrap_or(Json::Null)
}

/// Build the profile. `lessons` are in lesson order, `events` oldest first.
pub fn build_profile(track: &str, lessons: &[LessonRef], events: &[Json], now: &str) -> Json {
    let now_ms = parse_iso(now).unwrap_or(0);
    let is_run = |e: &Json| matches!(str_of(e, "kind"), Some("pass" | "fail"));
    let children_of = |id: &str| -> Vec<usize> { (0..lessons.len()).filter(|&i| lessons[i].kind != "core" && lessons[i].parent.as_deref() == Some(id)).collect() };

    // Own runs for every item; review schedule for core lessons, where a run on one of
    // the lesson's drills or stretches also counts as a review once the lesson is due.
    let states: Vec<LessonState> = lessons
        .iter()
        .map(|l| {
            let children: Vec<&str> = if l.kind == "core" { children_of(&l.id).into_iter().map(|i| lessons[i].id.as_str()).collect() } else { Vec::new() };
            let mut st = LessonState::default();
            for e in events {
                let id = str_of(e, "lesson_id").unwrap_or("");
                let mine = id == l.id;
                if str_of(e, "kind") == Some("feedback") && mine {
                    st.feedback = e.get("payload").and_then(|p| p.get("difficulty")).and_then(Json::as_str).map(String::from);
                }
                if !is_run(e) || !(mine || children.contains(&id)) {
                    continue;
                }
                let ts = str_of(e, "ts").unwrap_or("");
                let Some(t) = parse_iso(ts) else { continue };
                let pass = str_of(e, "kind") == Some("pass");
                if mine {
                    st.runs += 1;
                    st.last_run_ts = ts.to_string();
                }
                if !st.solved {
                    if mine && pass {
                        st.solved = true;
                        st.first_pass = Some(st.runs);
                        st.first_pass_ts = ts.to_string();
                        st.step = 0;
                        st.due = t + REVIEW_DAYS[0] * DAY;
                    }
                } else if l.kind == "core" && t >= st.due {
                    if pass {
                        st.step = if st.lapsed { 0 } else { (st.step + 1).min(REVIEW_DAYS.len() - 1) };
                        st.lapsed = false;
                        st.due = t + REVIEW_DAYS[st.step] * DAY;
                    } else {
                        st.lapsed = true;
                    }
                }
            }
            st
        })
        .collect();
    let reviewed = |i: usize| lessons[i].kind == "core" && states[i].solved;

    let score = |i: usize| -> Option<f64> {
        let s = &states[i];
        if s.runs == 0 {
            return None;
        }
        if !s.solved {
            return Some(0.2);
        }
        let base = match s.first_pass {
            Some(1) => 0.7,
            Some(n) if n <= 3 => 0.6,
            _ => 0.5,
        };
        let step = if reviewed(i) { s.step as f64 } else { 0.0 };
        Some((base + 0.1 * step - if s.lapsed { 0.2 } else { 0.0 }).clamp(0.0, 1.0))
    };
    let mut concepts: Vec<(String, Vec<f64>, Vec<String>)> = Vec::new();
    for (i, l) in lessons.iter().enumerate() {
        for name in &l.concepts {
            let c = match concepts.iter().position(|c| &c.0 == name) {
                Some(c) => c,
                None => {
                    concepts.push((name.clone(), Vec::new(), Vec::new()));
                    concepts.len() - 1
                }
            };
            concepts[c].2.push(l.id.clone());
            if let Some(v) = score(i) {
                concepts[c].1.push(v);
            }
        }
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for e in events.iter().filter(|e| str_of(e, "kind") == Some("fail")) {
        let Some(p) = e.get("payload") else { continue };
        let errors = p.get("errors").map(Json::as_array).unwrap_or(&[]);
        let failing = p.get("failing").map(Json::as_array).unwrap_or(&[]);
        for what in errors.iter().chain(failing).filter_map(Json::as_str) {
            *counts.entry(what.to_string()).or_default() += 1;
        }
    }
    let mut recurring: Vec<(String, usize)> = counts.into_iter().filter(|(_, n)| *n >= 2).collect();
    recurring.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    recurring.truncate(5);

    let times: Vec<i64> = events.iter().filter_map(|e| parse_iso(str_of(e, "ts")?)).collect();
    let (mut sessions, mut span, mut start, mut prev) = (0, 0i64, 0i64, None::<i64>);
    for &t in &times {
        if prev.map_or(true, |p| t - p > SESSION_GAP) {
            if let Some(p) = prev {
                span += p - start;
            }
            sessions += 1;
            start = t;
        }
        prev = Some(t);
    }
    if let Some(p) = prev {
        span += p - start;
    }
    let days: BTreeSet<String> = events.iter().filter_map(|e| str_of(e, "ts")).map(|t| day(t).to_string()).collect();
    let mut cursor = if days.contains(day(now)) { now_ms } else { now_ms - DAY };
    let mut streak = 0;
    while days.contains(day(&iso_from_millis(cursor))) {
        streak += 1;
        cursor -= DAY;
    }

    let mut feedback = [("easy", 0usize), ("right", 0), ("hard", 0)];
    for e in events.iter().filter(|e| str_of(e, "kind") == Some("feedback")) {
        let d = e.get("payload").and_then(|p| p.get("difficulty")).and_then(Json::as_str);
        if let Some(slot) = feedback.iter_mut().find(|(k, _)| Some(*k) == d) {
            slot.1 += 1;
        }
    }
    let mut solved_runs: Vec<usize> = (0..lessons.len()).filter(|&i| reviewed(i)).filter_map(|i| states[i].first_pass).collect();
    solved_runs.sort();
    let mid = solved_runs.len() / 2;
    let median = match solved_runs.len() {
        0 => Json::Null,
        n if n % 2 == 1 => Json::Num(solved_runs[mid] as f64),
        _ => Json::Num((solved_runs[mid - 1] + solved_runs[mid]) as f64 / 2.0),
    };

    let views: Vec<View> = (0..lessons.len())
        .map(|i| {
            let s = &states[i];
            View {
                i,
                status: if s.solved { "solved" } else if s.runs > 0 { "started" } else { "new" },
                due: reviewed(i).then(|| iso_from_millis(s.due)),
                due_now: reviewed(i) && now_ms >= s.due,
            }
        })
        .collect();
    let next = choose_next(lessons, &states, &views);

    Json::obj(vec![
        ("generated_at", Json::str(now)),
        ("track", Json::str(track)),
        (
            "sessions",
            Json::obj(vec![
                ("count", Json::Num(sessions as f64)),
                ("streak_days", Json::Num(streak as f64)),
                ("minutes_total", Json::Num((span as f64 / 60_000.0).round())),
                ("last_ts", events.last().and_then(|e| e.get("ts")).cloned().unwrap_or(Json::Null)),
            ]),
        ),
        (
            "pace",
            Json::obj(vec![
                ("median_runs_to_solve", median),
                ("feedback", Json::obj(feedback.iter().map(|(k, n)| (*k, Json::Num(*n as f64))).collect())),
            ]),
        ),
        (
            "lessons",
            Json::Arr(
                views
                    .iter()
                    .map(|v| {
                        let (l, s) = (&lessons[v.i], &states[v.i]);
                        Json::obj(vec![
                            ("id", Json::str(l.id.clone())),
                            ("title", Json::str(l.title.clone())),
                            ("kind", Json::str(l.kind.clone())),
                            ("for", l.parent.clone().map(Json::Str).unwrap_or(Json::Null)),
                            ("status", Json::str(v.status)),
                            ("runs", Json::Num(s.runs as f64)),
                            ("first_pass_run", opt_num(s.first_pass)),
                            ("feedback", s.feedback.clone().map(Json::Str).unwrap_or(Json::Null)),
                            ("review_step", opt_num(reviewed(v.i).then_some(s.step))),
                            ("due", v.due.clone().map(Json::Str).unwrap_or(Json::Null)),
                            ("due_now", Json::Bool(v.due_now)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "concepts",
            Json::Arr(
                concepts
                    .into_iter()
                    .map(|(name, scores, ids)| {
                        let mastery = if scores.is_empty() {
                            Json::Null
                        } else {
                            Json::Num((scores.iter().sum::<f64>() / scores.len() as f64 * 100.0).round() / 100.0)
                        };
                        Json::obj(vec![("name", Json::Str(name)), ("mastery", mastery), ("lessons", Json::Arr(ids.into_iter().map(Json::Str).collect()))])
                    })
                    .collect(),
            ),
        ),
        (
            "recurring",
            Json::Arr(recurring.into_iter().map(|(what, count)| Json::obj(vec![("what", Json::Str(what)), ("count", Json::Num(count as f64))])).collect()),
        ),
        ("next", next),
    ])
}

struct View {
    i: usize,
    status: &'static str,
    due: Option<String>,
    due_now: bool,
}

/// The adaptive next step (SHARED-FORMAT.md, section 5).
fn choose_next(lessons: &[LessonRef], states: &[LessonState], views: &[View]) -> Json {
    let pick = |i: usize, kind: &str, reason: String| {
        Json::obj(vec![("kind", Json::str(kind)), ("lesson_id", Json::str(lessons[i].id.clone())), ("title", Json::str(lessons[i].title.clone())), ("reason", Json::Str(reason))])
    };
    let children = |i: usize| -> Vec<usize> { (0..lessons.len()).filter(|&c| lessons[c].kind != "core" && lessons[c].parent.as_deref() == Some(lessons[i].id.as_str())).collect() };
    let core: Vec<usize> = (0..lessons.len()).filter(|&i| lessons[i].kind == "core").collect();
    let unsolved_child = |i: usize, kind: &str| children(i).into_iter().find(|&c| lessons[c].kind == kind && views[c].status != "solved");

    // 1. Reviews: most overdue first, practised with a drill when there is one.
    let mut due: Vec<usize> = core.iter().copied().filter(|&i| views[i].due_now).collect();
    due.sort_by(|&a, &b| views[a].due.cmp(&views[b].due));
    if let Some(&l) = due.first() {
        let since = views[l].due.as_deref().map(day).unwrap_or("").to_string();
        // Never-run first, then the one run longest ago; ties keep lesson order.
        let mut drills: Vec<usize> = children(l).into_iter().filter(|&c| lessons[c].kind == "drill").collect();
        drills.sort_by(|&a, &b| states[a].last_run_ts.cmp(&states[b].last_run_ts));
        return match drills.first() {
            Some(&d) => pick(d, "review", format!("review of \"{}\", due since {since}", lessons[l].title)),
            None => pick(l, "review", format!("due since {since}")),
        };
    }

    // 2. React to the most recently solved core lesson, unless the learner has moved on.
    let mut solved: Vec<usize> = core.iter().copied().filter(|&i| views[i].status == "solved").collect();
    solved.sort_by(|&a, &b| states[a].first_pass_ts.cmp(&states[b].first_pass_ts));
    if let Some(&last) = solved.last() {
        let pos = core.iter().position(|&i| i == last).unwrap_or(0);
        if !core[pos + 1..].iter().any(|&i| states[i].runs > 0) {
            let fb = states[last].feedback.as_deref();
            let struggled = fb == Some("hard") || (fb != Some("easy") && states[last].first_pass.unwrap_or(0) >= STRUGGLE_RUNS);
            let breezed = fb == Some("easy");
            if let (true, Some(d)) = (struggled, unsolved_child(last, "drill")) {
                return pick(d, "drill", format!("extra practice: \"{}\" was hard", lessons[last].title));
            }
            if let (true, Some(s)) = (breezed, unsolved_child(last, "stretch")) {
                return pick(s, "stretch", format!("\"{}\" was easy, so here's a harder one", lessons[last].title));
            }
        }
    }

    // 3. The current lesson: offer a warm-up drill after several unsuccessful runs.
    if let Some(&cur) = core.iter().find(|&&i| views[i].status != "solved") {
        let runs = states[cur].runs;
        if let (true, Some(d)) = (runs >= WARMUP_RUNS, unsolved_child(cur, "drill")) {
            return pick(d, "drill", format!("warm-up: \"{}\" has taken {runs} runs so far", lessons[cur].title));
        }
        return pick(cur, "lesson", if runs > 0 { "in progress".into() } else { "next new lesson".into() });
    }

    let earliest = core.iter().filter_map(|&i| views[i].due.clone()).min();
    let reason = match earliest {
        Some(d) => format!("all lessons solved; next review {}", day(&d)),
        None => "all lessons solved".to_string(),
    };
    Json::obj(vec![("kind", Json::str("done")), ("lesson_id", Json::Null), ("title", Json::Null), ("reason", Json::Str(reason))])
}

pub fn profile_markdown(p: &Json) -> String {
    let s = |j: Option<&Json>| j.and_then(Json::as_str).unwrap_or("").to_string();
    let n = |j: Option<&Json>| j.and_then(Json::as_f64).unwrap_or(0.0);
    let generated = s(p.get("generated_at"));
    let mut out = vec![
        START.to_string(),
        "<!-- Generated by the app from data/events.jsonl. Edits inside this block are overwritten. -->".to_string(),
        "## Learner profile".to_string(),
        String::new(),
        format!("Updated {} {} UTC.", generated.get(..10).unwrap_or(""), generated.get(11..16).unwrap_or("")),
        String::new(),
    ];
    let next = p.get("next");
    let (kind, title, reason) = (s(next.and_then(|x| x.get("kind"))), s(next.and_then(|x| x.get("title"))), s(next.and_then(|x| x.get("reason"))));
    let verb = match kind.as_str() {
        "review" => "review ",
        "drill" => "drill ",
        "stretch" => "stretch ",
        _ => "",
    };
    out.push(if kind == "done" { format!("**Next:** {reason}.") } else { format!("**Next:** {verb}\"{title}\" ({reason}).") });
    out.push(String::new());
    let sess = p.get("sessions");
    let count = n(sess.and_then(|x| x.get("count")));
    out.push(if count > 0.0 {
        let streak = n(sess.and_then(|x| x.get("streak_days")));
        format!(
            "**Sessions:** {}, {} min in total, streak {} {}, last on {}.",
            count,
            n(sess.and_then(|x| x.get("minutes_total"))),
            streak,
            if streak == 1.0 { "day" } else { "days" },
            day(&s(sess.and_then(|x| x.get("last_ts"))))
        )
    } else {
        "**Sessions:** none yet.".to_string()
    });
    out.push(String::new());
    let pace = p.get("pace");
    let fb = |k: &str| n(pace.and_then(|x| x.get("feedback")).and_then(|f| f.get(k)));
    let median = match pace.and_then(|x| x.get("median_runs_to_solve")).and_then(Json::as_f64) {
        Some(m) => format!("lessons solved in a median of {m} {}", if m == 1.0 { "run" } else { "runs" }),
        None => "no lessons solved yet".to_string(),
    };
    out.push(format!("**Pace:** {median}; rated too easy {}, about right {}, too hard {}.", fb("easy"), fb("right"), fb("hard")));
    out.push(String::new());
    out.push("| Exercise | Kind | Status | Runs | Solved on run | Rated | Review |".to_string());
    out.push("|----------|------|--------|------|---------------|-------|--------|".to_string());
    for l in p.get("lessons").map(Json::as_array).unwrap_or(&[]) {
        let review = if l.get("due_now").and_then(Json::as_bool) == Some(true) {
            "due now".to_string()
        } else {
            match l.get("due").and_then(Json::as_str) {
                Some(d) => format!("next {}", day(d)),
                None => "–".to_string(),
            }
        };
        let first = match l.get("first_pass_run").and_then(Json::as_f64) {
            Some(v) => v.to_string(),
            None => "–".to_string(),
        };
        let kind = match s(l.get("kind")).as_str() {
            "core" => "lesson".to_string(),
            other => other.to_string(),
        };
        let rated = l.get("feedback").and_then(Json::as_str).unwrap_or("–");
        out.push(format!("| {} | {kind} | {} | {} | {first} | {rated} | {review} |", s(l.get("title")), s(l.get("status")), n(l.get("runs"))));
    }
    out.push(String::new());
    let practiced: Vec<&Json> = p.get("concepts").map(Json::as_array).unwrap_or(&[]).iter().filter(|c| c.get("mastery").and_then(Json::as_f64).is_some()).collect();
    if practiced.is_empty() {
        out.push("No concepts practiced yet.".to_string());
    } else {
        out.push("| Concept | Mastery |".to_string());
        out.push("|---------|---------|".to_string());
        for c in practiced {
            out.push(format!("| {} | {:.2} |", s(c.get("name")), n(c.get("mastery"))));
        }
    }
    out.push(String::new());
    let recurring: Vec<String> = p
        .get("recurring")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|r| format!("`{}` ×{}", s(r.get("what")), n(r.get("count"))))
        .collect();
    out.push(if recurring.is_empty() { "**Recurring mistakes:** none yet.".to_string() } else { format!("**Recurring mistakes:** {}.", recurring.join(", ")) });
    out.push(END.to_string());
    out.join("\n")
}

/// Replace the generated block in PROGRESS.md (or insert it before "## Log").
pub fn write_progress(file: &Path, markdown: &str) -> std::io::Result<()> {
    let text = fs::read_to_string(file).unwrap_or_default();
    let next = match (text.find(START), text.find(END)) {
        (Some(a), Some(b)) if b > a => format!("{}{}{}", &text[..a], markdown, &text[b + END.len()..]),
        _ if text.contains("\n## Log") => text.replacen("\n## Log", &format!("\n{markdown}\n\n## Log"), 1),
        _ => {
            let sep = if text.is_empty() { "" } else if text.ends_with('\n') { "\n" } else { "\n\n" };
            format!("{text}{sep}{markdown}\n")
        }
    };
    if next != text {
        fs::write(file, next)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn fixture(name: &str) -> String {
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/fixtures").join(name)).unwrap()
    }

    fn lessons_from(j: &Json) -> Vec<LessonRef> {
        let text = |l: &Json, k: &str| l.get(k).and_then(Json::as_str).map(String::from);
        j.as_array()
            .iter()
            .map(|l| LessonRef {
                id: text(l, "id").unwrap(),
                title: text(l, "title").unwrap(),
                concepts: l.get("concepts").map(Json::as_array).unwrap_or(&[]).iter().filter_map(Json::as_str).map(String::from).collect(),
                kind: text(l, "kind").unwrap_or_else(|| "core".into()),
                parent: text(l, "for"),
            })
            .collect()
    }

    #[test]
    fn iso_round_trip() {
        for ts in ["1970-01-01T00:00:00.000Z", "2000-02-29T23:59:59.999Z", "2026-10-05T21:00:00.000Z"] {
            assert_eq!(iso_from_millis(parse_iso(ts).unwrap()), ts);
        }
        assert_eq!(parse_iso("2026-10-05T21:00:00Z"), parse_iso("2026-10-05T21:00:00.000Z"));
        assert_eq!(parse_iso("yesterday"), None);
    }

    #[test]
    fn matches_every_shared_case() {
        let all = parse(&fixture("profile-cases.json")).unwrap();
        let cases = all.get("cases").unwrap().as_array();
        assert!(cases.len() >= 8);
        for c in cases {
            let name = c.get("name").and_then(Json::as_str).unwrap();
            let profile = build_profile("demo", &lessons_from(c.get("lessons").unwrap()), c.get("events").unwrap().as_array(), c.get("now").and_then(Json::as_str).unwrap());
            assert_eq!(&profile, c.get("expected").unwrap(), "case {name}");
        }
        assert_eq!(profile_markdown(cases[0].get("expected").unwrap()) + "\n", fixture("profile-expected.md"));
    }

    #[test]
    fn passing_reviews_move_up_the_schedule() {
        let lessons = vec![LessonRef { id: "a".into(), title: "A".into(), concepts: vec!["c".into()], kind: "core".into(), parent: None }];
        let ev = |ts: &str| parse(&format!(r#"{{"ts":"{ts}","lesson_id":"a","kind":"pass","payload":{{"failing":[],"errors":[]}}}}"#)).unwrap();
        let reviewed = build_profile("t", &lessons, &[ev("2026-01-01T10:00:00.000Z"), ev("2026-01-02T11:00:00.000Z"), ev("2026-01-05T12:00:00.000Z")], "2026-01-06T00:00:00.000Z");
        let lesson = &reviewed.get("lessons").unwrap().as_array()[0];
        assert_eq!(lesson.get("review_step"), Some(&Json::Num(2.0)));
        assert_eq!(lesson.get("due"), Some(&Json::str("2026-01-12T12:00:00.000Z")));
        assert_eq!(reviewed.get("concepts").unwrap().as_array()[0].get("mastery"), Some(&Json::Num(0.9)));
    }

    #[test]
    fn write_progress_replaces_block_and_keeps_the_rest() {
        let file = std::env::temp_dir().join(format!("learn-prog-{}.md", std::process::id()));
        fs::write(&file, "# Progress\n\nintro\n\n## Log\n| a |\n").unwrap();
        write_progress(&file, "<!-- profile:start -->\nONE\n<!-- profile:end -->").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "# Progress\n\nintro\n\n<!-- profile:start -->\nONE\n<!-- profile:end -->\n\n## Log\n| a |\n");
        write_progress(&file, "<!-- profile:start -->\nTWO\n<!-- profile:end -->").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "# Progress\n\nintro\n\n<!-- profile:start -->\nTWO\n<!-- profile:end -->\n\n## Log\n| a |\n");
        fs::remove_file(&file).ok();
    }
}
