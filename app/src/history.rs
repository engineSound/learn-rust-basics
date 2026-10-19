//! Learning history: an append-only JSON Lines file (format: docs/SHARED-FORMAT.md).

use crate::json::{self, Json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CODE: usize = 64 * 1024;

/// Requests run on separate threads; keep each line's write whole.
static APPEND_LOCK: Mutex<()> = Mutex::new(());

pub fn append_event(file: &Path, track: &str, lesson_id: Option<&Json>, kind: &str, duration_ms: Option<f64>, payload: Json) -> std::io::Result<()> {
    let event = Json::obj(vec![
        ("ts", Json::Str(now_iso())),
        ("track", Json::str(track)),
        ("lesson_id", lesson_id.cloned().unwrap_or(Json::Null)),
        ("concept", Json::Null),
        ("kind", Json::str(kind)),
        ("duration_ms", duration_ms.map(Json::Num).unwrap_or(Json::Null)),
        ("payload", payload),
    ]);
    let _guard = APPEND_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(file)?;
    f.write_all(format!("{}\n", event.to_string()).as_bytes())
}

/// All events, oldest first. Lines that aren't valid JSON are skipped.
pub fn read_events(file: &Path) -> Vec<Json> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines().filter(|l| !l.trim().is_empty()).filter_map(|l| json::parse(l).ok()).collect()
}

/// The payload recorded for a test run, built from the runner's result.
pub fn run_payload(result: &Json, code: &str) -> Json {
    let num = |k: &str| result.get(k).cloned().unwrap_or(Json::Num(0.0));
    let failing = result
        .get("tests")
        .map(|t| t.as_array())
        .unwrap_or(&[])
        .iter()
        .filter(|t| t.get("ok").and_then(Json::as_bool) == Some(false))
        .filter_map(|t| t.get("name").cloned())
        .collect();
    let errors = result
        .get("compileErrors")
        .map(|t| t.as_array())
        .unwrap_or(&[])
        .iter()
        .filter_map(|e| match e.get("code") {
            Some(Json::Str(c)) => Some(Json::str(c.clone())),
            _ => e.get("title").cloned(),
        })
        .collect();
    let mut cut = code.len().min(MAX_CODE);
    while !code.is_char_boundary(cut) {
        cut -= 1;
    }
    Json::obj(vec![
        ("passed", num("passed")),
        ("failed", num("failed")),
        ("total", num("total")),
        ("failing", Json::Arr(failing)),
        ("errors", Json::Arr(errors)),
        ("timed_out", result.get("timedOut").cloned().unwrap_or(Json::Bool(false))),
        ("code", Json::str(&code[..cut])),
    ])
}

/// True when a run result counts as solving the lesson.
pub fn is_pass(result: &Json) -> bool {
    let n = |k: &str| result.get(k).and_then(Json::as_f64).unwrap_or(0.0);
    n("total") > 0.0
        && n("failed") == 0.0
        && result.get("compileErrors").map(|e| e.as_array().is_empty()).unwrap_or(true)
        && result.get("timedOut").and_then(Json::as_bool) != Some(true)
}

/// Per-lesson summary shown in the UI.
pub fn lesson_stats(events: &[Json], lesson_id: Option<&Json>) -> Json {
    let runs: Vec<&Json> = events
        .iter()
        .filter(|e| lesson_id.is_some() && e.get("lesson_id") == lesson_id)
        .filter(|e| matches!(e.get("kind").and_then(Json::as_str), Some("pass" | "fail")))
        .collect();
    let first_pass = runs.iter().position(|e| e.get("kind").and_then(Json::as_str) == Some("pass"));
    let last = runs.last();
    let payload_num = |e: &Json, k: &str| e.get("payload").and_then(|p| p.get(k)).and_then(Json::as_f64);
    let best = runs.iter().filter_map(|e| payload_num(e, "passed")).fold(0.0, f64::max);
    // A run that doesn't compile reports 0 tests, so take the count from any run that did.
    let total = runs.iter().filter_map(|e| payload_num(e, "total")).fold(0.0, f64::max);
    Json::obj(vec![
        ("runs", Json::Num(runs.len() as f64)),
        ("first_pass_run", first_pass.map(|i| Json::Num(i as f64 + 1.0)).unwrap_or(Json::Null)),
        ("last_result", last.and_then(|e| e.get("kind").cloned()).unwrap_or(Json::Null)),
        ("last_ts", last.and_then(|e| e.get("ts").cloned()).unwrap_or(Json::Null)),
        ("best_passed", Json::Num(best)),
        ("total", if total > 0.0 { Json::Num(total) } else { Json::Null }),
    ])
}

/// Current UTC time as ISO 8601 with milliseconds, e.g. 2026-10-12T09:30:00.000Z.
pub fn now_iso() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    iso_from_millis(d.as_millis() as i64)
}

pub fn iso_from_millis(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        ms.rem_euclid(1000)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_timestamps() {
        assert_eq!(iso_from_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_from_millis(951_782_400_123), "2000-02-29T00:00:00.123Z");
        assert_eq!(iso_from_millis(1_791_801_000_000), "2026-10-12T10:30:00.000Z");
    }

    #[test]
    fn append_read_back_skip_torn_line_and_stats() {
        let file = std::env::temp_dir().join(format!("learn-hist-{}", std::process::id())).join("events.jsonl");
        let id = Json::str("x");
        let other = Json::str("other");
        let fail = json::parse(r#"{"passed":2,"failed":1,"total":3,"tests":[{"name":"a","ok":true},{"name":"c","ok":false}],"compileErrors":[],"timedOut":false}"#).unwrap();
        let pass = json::parse(r#"{"passed":3,"failed":0,"total":3,"tests":[],"compileErrors":[],"timedOut":false}"#).unwrap();
        let broken = json::parse(r#"{"passed":0,"failed":0,"total":0,"tests":[],"compileErrors":[{"code":"E0308","title":"mismatched types"}],"timedOut":false}"#).unwrap();
        assert!(!is_pass(&fail) && is_pass(&pass) && !is_pass(&broken));

        append_event(&file, "rust", Some(&id), "lesson_open", None, Json::Null).unwrap();
        append_event(&file, "rust", Some(&id), "fail", Some(900.0), run_payload(&broken, "fn")).unwrap();
        append_event(&file, "rust", Some(&id), "fail", Some(800.0), run_payload(&fail, "fn a")).unwrap();
        append_event(&file, "rust", Some(&other), "pass", None, run_payload(&pass, "")).unwrap();
        append_event(&file, "rust", Some(&id), "pass", Some(700.0), run_payload(&pass, "fn b")).unwrap();
        fs::OpenOptions::new().append(true).open(&file).unwrap().write_all(b"{\"ts\":\"broken").unwrap();

        let events = read_events(&file);
        assert_eq!(events.len(), 5);
        let keys: Vec<&str> = match &events[0] {
            Json::Obj(f) => f.iter().map(|(k, _)| k.as_str()).collect(),
            _ => panic!(),
        };
        assert_eq!(keys, ["ts", "track", "lesson_id", "concept", "kind", "duration_ms", "payload"]);
        assert_eq!(events[1].get("payload").and_then(|p| p.get("errors")), Some(&Json::Arr(vec![Json::str("E0308")])));
        assert_eq!(events[2].get("payload").and_then(|p| p.get("failing")), Some(&Json::Arr(vec![Json::str("c")])));

        let st = lesson_stats(&events, Some(&id));
        assert_eq!(st.get("runs"), Some(&Json::Num(3.0)));
        assert_eq!(st.get("first_pass_run"), Some(&Json::Num(3.0)));
        assert_eq!(st.get("last_result"), Some(&Json::str("pass")));
        assert_eq!(st.get("best_passed"), Some(&Json::Num(3.0)));
        assert_eq!(st.get("total"), Some(&Json::Num(3.0)));
        assert_eq!(lesson_stats(&[], Some(&id)).get("runs"), Some(&Json::Num(0.0)));
        fs::remove_dir_all(file.parent().unwrap()).ok();
    }
}
