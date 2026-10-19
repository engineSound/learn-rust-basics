//! Runs a lesson's tests against the learner's code with `cargo test`.
//!
//! The lesson is copied to data/runs/<slug>/ (kept between runs) and built with a
//! shared target directory, so after the first build a run takes a second or two.

use crate::json::Json;
use crate::lessons::Lesson;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_OUTPUT: usize = 200_000;
const SNIPPET_TIMEOUT: Duration = Duration::from_secs(60);

/// One run at a time: runs share the scratch folder and the target directory.
static RUN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, PartialEq)]
pub struct TestResult {
    pub name: String,
    pub ok: bool,
    pub message: String,
    /// From `assert_eq!(actual, expected)`: the `right` value.
    pub expected: Option<String>,
    /// From `assert_eq!(actual, expected)`: the `left` value.
    pub actual: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Diagnostic {
    pub code: Option<String>,
    pub title: String,
    pub message: String,
}

pub fn run_lesson(lesson: &Lesson, code: &str, data_dir: &Path) -> std::io::Result<Json> {
    let _guard = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let scratch = data_dir.join("runs").join(&lesson.slug);
    if scratch.exists() {
        fs::remove_dir_all(scratch.join("src")).ok();
    }
    copy_dir(&lesson.dir, &scratch)?;
    // Make the copy its own workspace so cargo doesn't look for the repo's workspace.
    let manifest = scratch.join("Cargo.toml");
    let mut toml = fs::read_to_string(&manifest)?;
    if !toml.contains("[workspace]") {
        toml.push_str("\n[workspace]\n");
        fs::write(&manifest, toml)?;
    }
    fs::write(scratch.join(&lesson.file), code)?;

    let started = Instant::now();
    let (stdout, stderr, timed_out, _) = cargo(&["test", "--color", "never"], &scratch, data_dir, TIMEOUT)?;
    let duration_ms = started.elapsed().as_millis() as f64;

    let tests = parse_tests(&stdout);
    let diagnostics = parse_errors(&stderr);
    let passed = tests.iter().filter(|t| t.ok).count();
    let total = tests.len();
    let output = format!("{}\n{}", stderr.trim(), stdout.trim()).trim().to_string();

    Ok(Json::obj(vec![
        ("passed", Json::Num(passed as f64)),
        ("failed", Json::Num((total - passed) as f64)),
        ("total", Json::Num(total as f64)),
        (
            "tests",
            Json::Arr(
                tests
                    .into_iter()
                    .map(|t| {
                        let mut fields = vec![("name", Json::Str(t.name)), ("ok", Json::Bool(t.ok)), ("message", Json::Str(t.message))];
                        if t.expected.is_some() || t.actual.is_some() {
                            fields.push(("expected", Json::Str(t.expected.unwrap_or_default())));
                            fields.push(("actual", Json::Str(t.actual.unwrap_or_default())));
                        }
                        Json::obj(fields)
                    })
                    .collect(),
            ),
        ),
        (
            "compileErrors",
            Json::Arr(
                diagnostics
                    .into_iter()
                    .map(|d| {
                        Json::obj(vec![
                            ("code", d.code.map(Json::Str).unwrap_or(Json::Null)),
                            ("title", Json::Str(d.title)),
                            ("message", Json::Str(d.message)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("timedOut", Json::Bool(timed_out)),
        ("durationMs", Json::Num(duration_ms)),
        ("output", Json::Str(output)),
    ]))
}

/// Run cargo in `dir` with a shared target directory and a timeout.
/// Returns (stdout, stderr, timed_out, succeeded).
fn cargo(args: &[&str], dir: &Path, data_dir: &Path, timeout: Duration) -> std::io::Result<(String, String, bool, bool)> {
    let started = Instant::now();
    let mut child = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(args)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", data_dir.join("target"))
        .env("CARGO_TERM_COLOR", "never")
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let out = read_async(child.stdout.take());
    let err = read_async(child.stderr.take());
    let mut timed_out = false;
    let mut succeeded = false;
    loop {
        if let Some(status) = child.try_wait()? {
            succeeded = status.success();
            break;
        }
        if started.elapsed() > timeout {
            timed_out = true;
            child.kill().ok();
            child.wait().ok();
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    Ok((out.join().unwrap_or_default(), err.join().unwrap_or_default(), timed_out, succeeded))
}

/// Run a small example from a lesson (```run blocks). Code without `fn main` is wrapped in one.
pub fn run_snippet(code: &str, data_dir: &Path) -> std::io::Result<Json> {
    let _guard = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = data_dir.join("snippet");
    fs::create_dir_all(dir.join("src"))?;
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"snippet\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n")?;
    let main = if code.contains("fn main") { code.to_string() } else { format!("fn main() {{\n{code}\n}}\n") };
    fs::write(dir.join("src/main.rs"), main)?;
    let (stdout, stderr, timed_out, ok) = cargo(&["run", "--quiet", "--color", "never"], &dir, data_dir, SNIPPET_TIMEOUT)?;
    let errors = parse_errors(&stderr);
    let mut output = stdout;
    if !ok && errors.is_empty() {
        // A runtime panic: show it after the program's own output.
        let panic = stderr.find("thread 'main'").map(|i| &stderr[i..]).unwrap_or(&stderr);
        output.push_str(panic.lines().filter(|l| !l.starts_with("note: ")).collect::<Vec<_>>().join("\n").trim());
    }
    Ok(Json::obj(vec![
        ("output", Json::Str(output)),
        ("ok", Json::Bool(ok && !timed_out)),
        (
            "errors",
            Json::Arr(
                errors
                    .into_iter()
                    .map(|d| Json::obj(vec![("code", d.code.map(Json::Str).unwrap_or(Json::Null)), ("title", Json::Str(d.title)), ("message", Json::Str(d.message))]))
                    .collect(),
            ),
        ),
        ("timedOut", Json::Bool(timed_out)),
    ]))
}

fn read_async<R: Read + Send + 'static>(pipe: Option<R>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = pipe {
            let mut chunk = [0u8; 8192];
            while let Ok(n) = p.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                if buf.len() < MAX_OUTPUT {
                    buf.extend_from_slice(&chunk[..n]);
                }
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let dest: PathBuf = to.join(&name);
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// Parse libtest's output: `test tests::name ... ok|FAILED` plus the failure sections.
pub fn parse_tests(stdout: &str) -> Vec<TestResult> {
    let mut tests: Vec<TestResult> = stdout
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("test ")?;
            let (name, status) = rest.rsplit_once(" ... ")?;
            let ok = match status.trim() {
                "ok" => true,
                "FAILED" => false,
                _ => return None,
            };
            Some(TestResult { name: name.to_string(), ok, message: String::new(), expected: None, actual: None })
        })
        .collect();

    // Sections look like: ---- tests::name stdout ----\n<panic message>\n
    let mut current: Option<String> = None;
    let mut buf: Vec<&str> = Vec::new();
    let flush = |name: &Option<String>, buf: &mut Vec<&str>, tests: &mut Vec<TestResult>| {
        if let Some(name) = name {
            if let Some(t) = tests.iter_mut().find(|t| &t.name == name) {
                t.message = clean_panic(buf);
                let value = |prefix: &str| buf.iter().find_map(|l| l.trim().strip_prefix(prefix).map(|v| v.trim().to_string()));
                t.actual = value("left:");
                t.expected = value("right:");
            }
        }
        buf.clear();
    };
    for line in stdout.lines() {
        if let Some(name) = line.strip_prefix("---- ").and_then(|s| s.strip_suffix(" stdout ----")) {
            flush(&current, &mut buf, &mut tests);
            current = Some(name.to_string());
        } else if line == "failures:" || line.starts_with("test result:") {
            flush(&current, &mut buf, &mut tests);
            current = None;
        } else if current.is_some() {
            buf.push(line);
        }
    }
    flush(&current, &mut buf, &mut tests);

    // "tests::says_hello" (unit test) or "says_hello" (tests/*.rs) → "says hello".
    for t in &mut tests {
        let short = t.name.strip_prefix("tests::").unwrap_or(&t.name);
        if !short.contains("::") {
            t.name = short.replace('_', " ");
        }
    }
    tests
}

fn clean_panic(lines: &[&str]) -> String {
    lines
        .iter()
        .filter(|l| !l.starts_with("note: "))
        .map(|l| {
            // "thread 'tests::x' (123) panicked at src/lib.rs:7:5:" -> "panicked at src/lib.rs:7:5:"
            match (l.starts_with("thread '"), l.find("panicked at ")) {
                (true, Some(i)) => &l[i..],
                _ => l,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Pull compiler errors (not warnings) out of cargo's stderr.
pub fn parse_errors(stderr: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut push = |block: &mut Vec<&str>| {
        if let Some(first) = block.first() {
            if let Some(d) = to_diagnostic(first, block) {
                out.push(d);
            }
        }
        block.clear();
    };
    for line in stderr.lines() {
        let starts_new = line.starts_with("error") || line.starts_with("warning") || line.starts_with("   Compiling") || line.starts_with("    Finished") || line.starts_with("     Running");
        if starts_new {
            push(&mut block);
        }
        block.push(line);
    }
    push(&mut block);
    out
}

fn to_diagnostic(first: &str, block: &[&str]) -> Option<Diagnostic> {
    let (code, title) = if let Some(rest) = first.strip_prefix("error[") {
        let (code, title) = rest.split_once("]: ")?;
        (Some(code.to_string()), title.to_string())
    } else if let Some(title) = first.strip_prefix("error: ") {
        // Cargo's own summary lines aren't useful to the learner.
        if title.starts_with("could not compile") || title.starts_with("test failed") || title.starts_with("aborting") {
            return None;
        }
        (None, title.to_string())
    } else {
        return None;
    };
    Some(Diagnostic { code, title, message: block.join("\n").trim_end().to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STDOUT: &str = "
running 2 tests
test tests::adds_numbers ... ok
test tests::subtracts ... FAILED

failures:

---- tests::subtracts stdout ----

thread 'tests::subtracts' (891) panicked at src/lib.rs:13:5:
not yet implemented
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    tests::subtracts

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";

    #[test]
    fn parses_results_and_panic_messages() {
        assert_eq!(
            parse_tests(STDOUT),
            vec![
                TestResult { name: "adds numbers".into(), ok: true, message: String::new(), expected: None, actual: None },
                TestResult { name: "subtracts".into(), ok: false, message: "panicked at src/lib.rs:13:5:\nnot yet implemented".into(), expected: None, actual: None },
            ]
        );
    }

    #[test]
    fn assert_eq_failures_give_expected_and_got() {
        let out = "test tests::f ... FAILED\n\nfailures:\n\n---- tests::f stdout ----\n\nthread 'tests::f' (7) panicked at src/lib.rs:3:5:\nassertion `left == right` failed\n  left: 32.0\n right: 33.0\n\nfailures:\n";
        let t = &parse_tests(out)[0];
        assert_eq!((t.actual.as_deref(), t.expected.as_deref()), (Some("32.0"), Some("33.0")));
    }

    #[test]
    fn snippets_run_wrapped_in_main_and_report_errors_and_panics() {
        let data = std::env::temp_dir().join(format!("learn-snippet-test-{}", std::process::id()));
        let field = |j: &Json, k: &str| j.get(k).cloned().unwrap();
        let ok = run_snippet("let x = 2;\nprintln!(\"x = {}\", x * 21);", &data).unwrap();
        assert_eq!(field(&ok, "ok"), Json::Bool(true));
        assert_eq!(field(&ok, "output"), Json::str("x = 42\n"));
        let broken = run_snippet("let x: i32 = \"text\";", &data).unwrap();
        assert_eq!(field(&broken, "ok"), Json::Bool(false));
        assert!(broken.to_string().contains("E0308"));
        let panics = run_snippet("let v: Vec<i32> = vec![];\nprintln!(\"{}\", v[3]);", &data).unwrap();
        assert_eq!(field(&panics, "ok"), Json::Bool(false));
        assert!(field(&panics, "output").as_str().unwrap().contains("index out of bounds"));
        fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn keeps_errors_with_codes_and_drops_warnings_and_summaries() {
        let stderr = "   Compiling lesson v0.1.0
warning: unused variable: `c`
 --> src/lib.rs:6:30

error[E0308]: mismatched types
 --> src/lib.rs:7:5
  |
7 |     c * 2.0;
  |             - help: remove this semicolon

error: could not compile `lesson` (lib test) due to 1 previous error
";
        let errs = parse_errors(stderr);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code.as_deref(), Some("E0308"));
        assert_eq!(errs[0].title, "mismatched types");
        assert!(errs[0].message.contains("remove this semicolon"));
    }

    /// End to end: copies lesson 05, runs real `cargo test`, and reads the results.
    #[test]
    fn runs_lesson_05_starter_broken_and_solved() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let lesson = crate::lessons::load(&root.join("lessons"), "05-if-else").unwrap().unwrap();
        let data = std::env::temp_dir().join(format!("learn-rust-test-{}", std::process::id()));
        let field = |j: &Json, key: &str| match j {
            Json::Obj(f) => f.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap(),
            _ => panic!("not an object"),
        };
        let len = |j: Json| match j {
            Json::Arr(a) => a.len(),
            _ => panic!("not an array"),
        };

        let starter = run_lesson(&lesson, &lesson.starter, &data).unwrap();
        assert_eq!(field(&starter, "passed"), Json::Num(0.0));
        assert_eq!(field(&starter, "total"), Json::Num(4.0));

        let broken = lesson.starter.replacen("    todo!()", "    String::from(\"ok\");", 1);
        let r = run_lesson(&lesson, &broken, &data).unwrap();
        assert_eq!(len(field(&r, "compileErrors")), 1, "{}", r.to_string());
        assert!(r.to_string().contains("E0308"));

        let half = lesson.starter.replacen("    todo!()", "    if charging { String::from(\"charging\") } else if level < 20 { String::from(\"low\") } else { String::from(\"ok\") }", 1);
        let r = run_lesson(&lesson, &half, &data).unwrap();
        assert_eq!(field(&r, "passed"), Json::Num(3.0), "{}", r.to_string());

        let solved = half.replacen("    todo!()", "    level >= 0 && level <= 100", 1);
        let r = run_lesson(&lesson, &solved, &data).unwrap();
        assert_eq!(field(&r, "passed"), Json::Num(4.0), "{}", r.to_string());
        assert_eq!(field(&r, "failed"), Json::Num(0.0));
        fs::remove_dir_all(&data).ok();
    }
}
