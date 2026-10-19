//! The tutor: hints, step-through visuals of the learner's own code and newly generated drills from Claude, adapted to the learner's profile.
//! Prompts live in docs/tutor/ (shared with learn-nodejs). Rules: docs/SHARED-FORMAT.md, section 6.
//!
//! Rust has no official Anthropic SDK, so this calls the Messages API over raw HTTP through
//! the system `curl` (preinstalled on macOS), keeping the app free of crate dependencies.

use crate::json::{self, Json};
use crate::lessons::Lesson;
use crate::runner;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub const MODEL: &str = "claude-opus-5-5";
const MAX_TOKENS: f64 = 16000.0;
const ATTEMPTS: usize = 2;

#[derive(Debug)]
pub struct TutorError {
    pub status: u16,
    pub message: String,
}

fn err(status: u16, message: impl Into<String>) -> TutorError {
    TutorError { status, message: message.into() }
}

type Transport = Box<dyn Fn(&Json) -> Result<Json, TutorError> + Send>;

/// Tests replace the HTTP call with a function that returns canned responses.
static FAKE: Mutex<Option<Transport>> = Mutex::new(None);

#[cfg(test)]
pub fn set_transport_for_tests(t: Option<Transport>) {
    *FAKE.lock().unwrap_or_else(|e| e.into_inner()) = t;
}

fn api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY").ok().filter(|k| !k.trim().is_empty())
}

/// `{ enabled, reason }` for /api/info.
pub fn status() -> Json {
    let fake = FAKE.lock().map(|f| f.is_some()).unwrap_or(false);
    let reason = if fake || api_key().is_some() { None } else { Some("put ANTHROPIC_API_KEY in .env, then restart.") };
    Json::obj(vec![("enabled", Json::Bool(reason.is_none())), ("reason", reason.map(Json::str).unwrap_or(Json::Null))])
}

/// Names set from .env (for explaining a rejected key).
static FROM_ENV_FILE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Load KEY=VALUE lines from .env into the environment. Its values win over the shell's,
/// so an old exported key can't shadow it.
pub fn load_env_file(path: &Path) {
    let Ok(text) = fs::read_to_string(path) else { return };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let (k, v) = (k.trim().trim_start_matches("export ").trim(), v.trim().trim_matches('"').trim_matches('\'').trim());
            if !k.is_empty() && !v.is_empty() {
                std::env::set_var(k, v);
                FROM_ENV_FILE.lock().unwrap_or_else(|e| e.into_inner()).push(k.to_string());
            }
        }
    }
}

/// Why a key was rejected, without revealing it: where it came from, its prefix and last 4
/// characters (as console.anthropic.com shows them), and the usual mix-ups. Same wording in learn-nodejs.
pub fn key_problem(api_message: &str) -> String {
    let Some(key) = api_key() else { return "No API key is set. Put ANTHROPIC_API_KEY in .env, then restart.".into() };
    let from_file = |name: &str| FROM_ENV_FILE.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|k| k == name);
    let where_ = if from_file("ANTHROPIC_API_KEY") { ".env" } else { "your shell environment (not .env)" };
    let chars: Vec<char> = key.chars().collect();
    let head: String = chars.iter().take(12).collect();
    let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    let quoted = if api_message.is_empty() { String::new() } else { format!(" (\"{api_message}\")") };
    let mut lines = vec![
        format!("The API key was rejected{quoted}."),
        format!("The app used the key from {where_}: {head}…{tail} ({} characters).", chars.len()),
    ];
    if !from_file("ANTHROPIC_API_KEY") {
        lines.push("Put your key in .env (it wins over the shell), or remove the old `export ANTHROPIC_API_KEY` from your shell profile.".into());
    }
    if key.starts_with("sk-ant-oat") {
        lines.push("This is a Claude subscription login token, not an API key. Create an API key at console.anthropic.com → API keys.".into());
    } else if !key.starts_with("sk-ant-api") {
        lines.push("That doesn't look like an Anthropic API key (they start with sk-ant-api). Check for a stray character or a different value.".into());
    }
    if let Ok(base) = std::env::var("ANTHROPIC_BASE_URL") {
        if base.trim_end_matches('/') != "https://api.anthropic.com" {
            let src = if from_file("ANTHROPIC_BASE_URL") { " in .env" } else { " in your shell" };
            lines.push(format!("Requests go to {base} (ANTHROPIC_BASE_URL{src}), not api.anthropic.com: remove it."));
        }
    }
    if lines.len() == 2 {
        lines.push("If those characters match a key at console.anthropic.com → API keys, check that it is active; otherwise paste the key into .env again, then restart.".into());
    }
    lines.join(" ")
}

/// POST /v1/messages with curl. The key goes in a private temporary config file, never on the command line.
fn post_messages(body: &Json) -> Result<Json, TutorError> {
    let key = api_key().ok_or_else(|| err(503, "The tutor needs ANTHROPIC_API_KEY in .env."))?;
    let base = std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com".into());
    let config = std::env::temp_dir().join(format!("learn-tutor-{}-{:?}.curl", std::process::id(), std::thread::current().id()));
    let quoted = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let lines = [
        format!("url = {}", quoted(&format!("{}/v1/messages", base.trim_end_matches('/')))),
        format!("header = {}", quoted(&format!("x-api-key: {key}"))),
        "header = \"anthropic-version: 2023-06-01\"".to_string(),
        "header = \"anthropic-beta: server-side-fallback-2026-07-01\"".to_string(),
        "header = \"content-type: application/json\"".to_string(),
        "data-binary = \"@-\"".to_string(),
        "silent".to_string(),
        "show-error".to_string(),
        "max-time = 600".to_string(),
        "write-out = \"\\n%{http_code}\"".to_string(),
    ];
    {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
        let mut f = opts.open(&config).map_err(|e| err(500, format!("can't write curl config: {e}")))?;
        f.write_all((lines.join("\n") + "\n").as_bytes()).map_err(|e| err(500, e.to_string()))?;
    }
    let result = (|| {
        let mut child = Command::new("curl")
            .arg("--config")
            .arg(&config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| err(503, format!("The tutor needs curl: {e}")))?;
        child.stdin.take().expect("stdin").write_all(body.to_string().as_bytes()).map_err(|e| err(502, e.to_string()))?;
        child.wait_with_output().map_err(|e| err(502, e.to_string()))
    })();
    fs::remove_file(&config).ok();
    let output = result?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (text, code) = stdout.rsplit_once('\n').unwrap_or(("", &stdout));
    let code: u16 = code.trim().parse().unwrap_or(0);
    if code == 0 {
        return Err(err(502, format!("Couldn't reach the Claude API: {}", String::from_utf8_lossy(&output.stderr).trim())));
    }
    let parsed = json::parse(text).map_err(|_| err(502, format!("The Claude API returned HTTP {code} with an unreadable body.")))?;
    if code != 200 {
        let message = parsed.get("error").and_then(|e| e.get("message")).and_then(Json::as_str).unwrap_or("unknown error").to_string();
        return Err(match code {
            401 => err(401, key_problem(&message)),
            429 => err(429, "The tutor is rate-limited right now. Try again in a minute."),
            _ => err(502, format!("The Claude API returned an error ({code}): {message}")),
        });
    }
    Ok(parsed)
}

/// One request to Claude; returns the text of the reply.
fn ask(system: &str, messages: &[Json], effort: &str, schema: Option<&Json>) -> Result<String, TutorError> {
    let mut output_config = vec![("effort", Json::str(effort))];
    if let Some(s) = schema {
        output_config.push(("format", Json::obj(vec![("type", Json::str("json_schema")), ("schema", s.clone())])));
    }
    let body = Json::obj(vec![
        ("model", Json::str(MODEL)),
        ("max_tokens", Json::Num(MAX_TOKENS)),
        // A declined request is re-run on Anthropic's recommended fallback model (beta header in post_messages).
        ("fallbacks", Json::str("default")),
        ("output_config", Json::obj(output_config)),
        ("system", Json::str(system)),
        ("messages", Json::Arr(messages.to_vec())),
    ]);
    let response = {
        let fake = FAKE.lock().unwrap_or_else(|e| e.into_inner());
        match fake.as_ref() {
            Some(f) => f(&body),
            None => {
                drop(fake);
                post_messages(&body)
            }
        }
    }?;
    match response.get("stop_reason").and_then(Json::as_str) {
        Some("refusal") => return Err(err(502, "The tutor declined this request.")),
        Some("max_tokens") => return Err(err(502, "The tutor ran out of room before finishing. Try again.")),
        _ => {}
    }
    Ok(response
        .get("content")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .filter(|b| b.get("type").and_then(Json::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Json::as_str))
        .collect())
}

fn user_message(text: String) -> Json {
    Json::obj(vec![("role", Json::str("user")), ("content", Json::Str(text))])
}

// ---------- prompts ----------

pub struct Context {
    pub docs_dir: PathBuf,
    pub track_name: String,
    pub learner: String,
    pub profile_markdown: String,
    pub lessons_dir: PathBuf,
    pub data_dir: PathBuf,
    pub track: String,
    pub id_prefix: String,
}

fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}

fn read_prompt(ctx: &Context, name: &str) -> Result<String, TutorError> {
    fs::read_to_string(ctx.docs_dir.join("tutor").join(name)).map_err(|e| err(500, format!("can't read docs/tutor/{name}: {e}")))
}

/// The "## Learner" section of PROGRESS.md: the learner's own description.
pub fn learner_section(progress: &str) -> String {
    let mut lines = progress.lines().skip_while(|l| l.trim() != "## Learner");
    if lines.next().is_none() {
        return "(not written yet)".into();
    }
    let body: Vec<&str> = lines.take_while(|l| !l.starts_with("## ") && !l.starts_with("<!--")).collect();
    let text = body.join("\n").trim().to_string();
    if text.is_empty() { "(not written yet)".into() } else { text }
}

fn system_prompt(ctx: &Context) -> Result<String, TutorError> {
    let profile: Vec<&str> = ctx.profile_markdown.lines().filter(|l| !l.starts_with("<!--")).collect();
    Ok(render(&read_prompt(ctx, "system.md")?, &[("track_name", &ctx.track_name), ("learner", &ctx.learner), ("profile", &profile.join("\n"))]))
}

fn num(j: &Json, key: &str) -> f64 {
    j.get(key).and_then(Json::as_f64).unwrap_or(0.0)
}

/// Plain-text summary of a run result for the prompt.
pub fn describe_result(r: Option<&Json>) -> String {
    let Some(r) = r else { return "Not run yet.".into() };
    if r.get("timedOut").and_then(Json::as_bool) == Some(true) {
        return "The run timed out.".into();
    }
    let errors = r.get("compileErrors").map(Json::as_array).unwrap_or(&[]);
    if !errors.is_empty() {
        let parts: Vec<String> = errors
            .iter()
            .map(|e| {
                let code = e.get("code").and_then(Json::as_str).map(|c| format!("{c} ")).unwrap_or_default();
                format!("{code}{}\n{}", e.get("title").and_then(Json::as_str).unwrap_or(""), e.get("message").and_then(Json::as_str).unwrap_or(""))
            })
            .collect();
        return format!("It does not compile yet:\n\n{}", parts.join("\n\n"));
    }
    let mut lines = vec![format!("{} of {} tests pass.", num(r, "passed"), num(r, "total"))];
    for t in r.get("tests").map(Json::as_array).unwrap_or(&[]).iter().filter(|t| t.get("ok").and_then(Json::as_bool) == Some(false)) {
        let msg = t.get("message").and_then(Json::as_str).unwrap_or("");
        let short: Vec<&str> = msg.lines().take(6).collect();
        let name = t.get("name").and_then(Json::as_str).unwrap_or("");
        lines.push(if short.is_empty() { format!("- FAILED {name}") } else { format!("- FAILED {name}: {}", short.join(" / ")) });
    }
    lines.join("\n")
}

// ---------- hints ----------

pub fn hint(ctx: &Context, title: &str, lesson: &str, file: &str, code: &str, result: Option<&Json>, question: &str) -> Result<String, TutorError> {
    let question = if question.trim().is_empty() { "(No question: they pressed \"Hint\".)" } else { question.trim() };
    let user = render(
        &read_prompt(ctx, "hint.md")?,
        &[("title", title), ("lesson", lesson), ("file", file), ("code", code), ("result", &describe_result(result)), ("question", question)],
    );
    Ok(ask(&system_prompt(ctx)?, &[user_message(user)], "medium", None)?.trim().to_string())
}

// ---------- "Show me what my code does": a visual of the learner's own failing run ----------

/// The lesson text without its ```visual blocks (they are long), and the first one as a style example.
pub fn split_visuals(body: &str) -> (String, Option<String>) {
    let mut text = String::new();
    let mut first = None;
    let mut rest = body;
    while let Some(start) = rest.find("```visual\n") {
        text.push_str(&rest[..start]);
        let after = &rest[start + "```visual\n".len()..];
        let end = after.find("\n```").unwrap_or(after.len());
        if first.is_none() {
            first = Some(after[..end].to_string());
        }
        rest = after.get(end + "\n```".len()..).unwrap_or("");
    }
    text.push_str(rest);
    (text, first)
}

/// The first scenario of a lesson visual, without its check, as a compact style example.
fn example_of(visual: &str) -> String {
    let Ok(spec) = json::parse(visual) else { return "(none)".into() };
    let first: Vec<Json> = spec.get("scenarios").map(Json::as_array).unwrap_or(&[]).iter().take(1).map(|sc| without(sc, &["check"])).collect();
    Json::obj(vec![
        ("title", spec.get("title").cloned().unwrap_or(Json::Null)),
        ("regions", spec.get("regions").cloned().unwrap_or(Json::Arr(vec![]))),
        ("scenarios", Json::Arr(first)),
    ])
    .to_string()
}

fn without(j: &Json, keys: &[&str]) -> Json {
    match j {
        Json::Obj(fields) => Json::Obj(fields.iter().filter(|(k, _)| !keys.contains(&k.as_str())).cloned().collect()),
        other => other.clone(),
    }
}

/// An object without its "" fields ("" means "not set" in visual-schema.json).
fn keep(j: &Json) -> Json {
    match j {
        Json::Obj(fields) => Json::Obj(fields.iter().filter(|(_, v)| v.as_str() != Some("")).cloned().collect()),
        other => other.clone(),
    }
}

/// Schema answer → the visual spec visual.js plays ("" and line 0 mean "not set").
pub fn to_visual(reply: &Json) -> Json {
    let arr = |j: Option<&Json>| j.map(Json::as_array).unwrap_or(&[]).to_vec();
    let sc = reply.get("scenario").cloned().unwrap_or(Json::Obj(vec![]));
    let steps: Vec<Json> = arr(sc.get("steps"))
        .iter()
        .map(|s| {
            let line = s.get("line").and_then(Json::as_f64).filter(|n| *n > 0.0).map(Json::Num).unwrap_or(Json::str(""));
            keep(&Json::obj(vec![
                ("line", line),
                ("say", s.get("say").cloned().unwrap_or(Json::str(""))),
                ("rule", s.get("rule").cloned().unwrap_or(Json::str(""))),
                ("error", s.get("error").cloned().unwrap_or(Json::str(""))),
                ("set", Json::Arr(arr(s.get("set")).iter().map(keep).collect())),
                ("drop", Json::Arr(arr(s.get("drop")))),
                ("arrows", Json::Arr(arr(s.get("arrows")).iter().map(keep).collect())),
            ]))
        })
        .collect();
    let name = sc.get("name").and_then(Json::as_str).filter(|n| !n.is_empty()).unwrap_or("Your code");
    Json::obj(vec![
        ("title", reply.get("title").cloned().unwrap_or(Json::str(""))),
        ("intro", reply.get("intro").cloned().unwrap_or(Json::str(""))),
        ("regions", Json::Arr(arr(reply.get("regions")).iter().map(keep).collect())),
        (
            "scenarios",
            Json::Arr(vec![Json::obj(vec![
                ("name", Json::str(name)),
                ("group", Json::str("Your code")),
                ("code", sc.get("code").cloned().unwrap_or(Json::str(""))),
                ("steps", Json::Arr(steps)),
            ])]),
        ),
    ])
}

fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Run the claimed call on the learner's real code: None when the claim holds, else what really happened.
fn check_claim(ctx: &Context, code: &str, call: &str, prints: &str) -> Result<Option<String>, TutorError> {
    let prints = flat(prints);
    if prints.is_empty() {
        return Ok(Some("check.prints is empty.".into()));
    }
    let main = if call.is_empty() { "fn main() {}".to_string() } else { format!("fn main() {{\n    println!(\"{{:?}}\", {call});\n}}") };
    let r = runner::run_snippet(&format!("{}\n\n{main}\n", code.trim_end()), &ctx.data_dir).map_err(|e| err(500, e.to_string()))?;
    let ok = r.get("ok").and_then(Json::as_bool) == Some(true);
    let out = flat(r.get("output").and_then(Json::as_str).unwrap_or(""));
    if r.get("timedOut").and_then(Json::as_bool) == Some(true) {
        return Ok(Some(format!("Running `{call}` on their code timed out.")));
    }
    if ok && call.is_empty() {
        return Ok(Some("Their code compiles, so check.call must make the call the failing test makes.".into()));
    }
    let holds = if ok { out == prints || out.ends_with(&format!(" {prints}")) } else { prints.len() >= 3 && r.to_string().contains(&prints) };
    if holds {
        return Ok(None);
    }
    let what = if call.is_empty() { "their code".to_string() } else { format!("`println!(\"{{:?}}\", {call})`") };
    let really: String = if out.is_empty() { describe_errors(&r) } else { out.chars().take(1500).collect() };
    Ok(Some(format!("Running {what} really gave:\n\n{really}\n\nbut check.prints says \"{prints}\". Trace their code again: the visual must show what really happens.")))
}

fn describe_errors(r: &Json) -> String {
    let errors: Vec<String> = r
        .get("errors")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|e| format!("{} {}", e.get("code").and_then(Json::as_str).unwrap_or(""), e.get("title").and_then(Json::as_str).unwrap_or("")).trim().to_string())
        .collect();
    if errors.is_empty() { "(no output)".into() } else { errors.join("\n") }
}

pub struct Trace {
    pub visual: Json,
    pub call: String,
    pub prints: String,
    pub attempts: usize,
}

/// Ask Claude for a step-through visual of the learner's code on the first failing test, then
/// check it: its structure (as visual.js will play it) and its claim (run on the real code).
pub fn visualize_run(ctx: &Context, title: &str, lesson: &str, file: &str, code: &str, result: Option<&Json>) -> Result<Trace, TutorError> {
    let (text, example) = split_visuals(lesson);
    let example = example.map(|e| example_of(&e)).unwrap_or_else(|| "(this lesson has none)".into());
    let user = render(
        &read_prompt(ctx, "visual.md")?,
        &[
            ("title", title),
            ("lesson", text.trim()),
            ("file", file),
            ("code", code),
            ("result", &describe_result(result)),
            ("example", &example),
            ("call_rule", "a Rust expression that calls their code the way the failing test does (the app adds `fn main() { println!(\"{:?}\", <call>); }` after their code)"),
            ("compile_word", "compile"),
        ],
    );
    let schema = json::parse(&read_prompt(ctx, "visual-schema.json")?).map_err(|e| err(500, format!("bad visual-schema.json: {e}")))?;
    let system = system_prompt(ctx)?;
    let mut messages = vec![user_message(user)];
    let mut problem = String::new();
    for attempt in 1..=ATTEMPTS {
        let reply = ask(&system, &messages, "medium", Some(&schema))?;
        match json::parse(&reply) {
            Err(_) => problem = "The reply was not valid JSON.".into(),
            Ok(parsed) => {
                let visual = to_visual(&parsed);
                let shape = crate::lessons::visual_problems(&visual);
                let check = parsed.get("check");
                let field = |k: &str| check.and_then(|c| c.get(k)).and_then(Json::as_str).unwrap_or("").trim().to_string();
                let (call, prints) = (field("call"), field("prints"));
                let found = if shape.is_empty() {
                    check_claim(ctx, code, &call, &prints)?
                } else {
                    Some(format!("The visualizer cannot play it:\n- {}", shape.iter().take(8).cloned().collect::<Vec<_>>().join("\n- ")))
                };
                match found {
                    None => return Ok(Trace { visual, call, prints, attempts: attempt }),
                    Some(p) => problem = p,
                }
            }
        }
        messages.push(Json::obj(vec![("role", Json::str("assistant")), ("content", Json::Str(reply))]));
        messages.push(user_message(format!("The app checked this visual and it failed:\n\n{problem}\n\nReturn a corrected visual in the same format.")));
    }
    let first_line = problem.lines().next().unwrap_or("").to_string();
    Err(err(502, format!("The tutor couldn't trace your code reliably ({first_line}). Try again, or ask a question instead.")))
}

// ---------- generated drills ----------

pub struct Parent {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub concepts: Vec<String>,
    pub body: String,
}

pub struct Made {
    pub slug: String,
    pub id: String,
    pub title: String,
    pub targets: String,
    pub attempts: usize,
}

struct Drill {
    title: String,
    slug: String,
    concepts: Vec<String>,
    minutes: f64,
    targets: String,
    explanation: String,
    starter: String,
    tests: String,
    solution: String,
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").replace(" #", " ")
}

fn slugify(s: &str) -> String {
    let lower = s.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).take(5).collect();
    if words.is_empty() { "drill".into() } else { words.join("-") }
}

fn to_drill(j: &Json) -> Result<Drill, String> {
    let text = |k: &str| match j.get(k).and_then(Json::as_str) {
        Some(s) if !s.trim().is_empty() => Ok(s.to_string()),
        _ => Err(format!("field \"{k}\" is missing or empty")),
    };
    let concepts = match j.get("concepts") {
        Some(Json::Arr(items)) => items.iter().filter_map(Json::as_str).map(String::from).collect(),
        _ => return Err("field \"concepts\" must be a list".into()),
    };
    Ok(Drill {
        title: text("title")?,
        slug: text("slug")?,
        concepts,
        minutes: j.get("minutes").and_then(Json::as_f64).unwrap_or(10.0),
        targets: text("targets")?,
        explanation: text("explanation")?,
        starter: text("starter")?,
        tests: text("tests")?,
        solution: text("solution")?,
    })
}

fn cargo_toml(name: &str) -> String {
    format!("[package]\nname = \"lesson-{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n")
}

fn with_tests(code: &str, tests: &str) -> String {
    format!("{}\n\n{}\n", code.trim_end(), tests.trim())
}

/// Run the drill's tests against its starter (must compile and fail) and its solution (must pass).
fn verify(ctx: &Context, d: &Drill) -> Result<Option<String>, TutorError> {
    let dir = std::env::temp_dir().join(format!("learn-gen-{}-{:?}", std::process::id(), std::thread::current().id()));
    fs::remove_dir_all(&dir).ok();
    let io = |e: std::io::Error| err(500, e.to_string());
    fs::create_dir_all(dir.join("src")).map_err(io)?;
    fs::write(dir.join("Cargo.toml"), cargo_toml("gen-check")).map_err(io)?;
    let starter_code = with_tests(&d.starter, &d.tests);
    fs::write(dir.join("src/lib.rs"), &starter_code).map_err(io)?;
    let lesson = Lesson { slug: "gen-check".into(), dir: dir.clone(), meta: vec![], body: String::new(), file: "src/lib.rs".into(), starter: starter_code.clone() };
    let check = || -> std::io::Result<Option<String>> {
        let starter = runner::run_lesson(&lesson, &starter_code, &ctx.data_dir)?;
        let (total, passed) = (num(&starter, "total"), num(&starter, "passed"));
        if !starter.get("compileErrors").map(Json::as_array).unwrap_or(&[]).is_empty() || starter.get("timedOut").and_then(Json::as_bool) == Some(true) {
            return Ok(Some(format!("The starter code does not compile: {}", describe_result(Some(&starter)))));
        }
        if total < 2.0 {
            return Ok(Some(format!("Only {total} test(s) ran; write at least 2.")));
        }
        if passed == total {
            return Ok(Some("Every test already passes against the starter code, so the tests check nothing.".into()));
        }
        let solved = runner::run_lesson(&lesson, &with_tests(&d.solution, &d.tests), &ctx.data_dir)?;
        let compiled = solved.get("compileErrors").map(Json::as_array).unwrap_or(&[]).is_empty();
        if !compiled || num(&solved, "failed") > 0.0 || num(&solved, "total") != total || solved.get("timedOut").and_then(Json::as_bool) == Some(true) {
            return Ok(Some(format!("The solution does not pass the tests: {}", describe_result(Some(&solved)))));
        }
        Ok(None)
    };
    let outcome = check().map_err(io);
    fs::remove_dir_all(&dir).ok();
    outcome
}

/// Ask Claude for a drill aimed at the learner's difficulties, verify it, and save it as
/// lessons/<NN>g-<slug>/ (kind: drill, for: the core lesson).
pub fn generate_drill(ctx: &Context, parent: &Parent, existing: &str, target: &str) -> Result<Made, TutorError> {
    let concepts = parent.concepts.join(", ");
    let user = render(
        &read_prompt(ctx, "drill.md")?,
        &[
            ("target", target),
            ("parent_title", &parent.title),
            ("parent_id", &parent.id),
            ("parent_concepts", &concepts),
            ("parent_lesson", &parent.body),
            ("existing", if existing.is_empty() { "(none)" } else { existing }),
            ("conventions", &read_prompt(ctx, &format!("conventions-{}.md", ctx.track))?),
        ],
    );
    let schema = json::parse(&read_prompt(ctx, "drill-schema.json")?).map_err(|e| err(500, format!("bad drill-schema.json: {e}")))?;
    let system = system_prompt(ctx)?;
    let mut messages = vec![user_message(user)];
    let mut problem = String::new();
    for attempt in 1..=ATTEMPTS {
        let text = ask(&system, &messages, "high", Some(&schema))?;
        let checked = match json::parse(&text) {
            Err(_) => Err("The reply was not valid JSON.".to_string()),
            Ok(j) => to_drill(&j),
        };
        match checked {
            Ok(d) => match verify(ctx, &d)? {
                None => return save(ctx, parent, &d, attempt),
                Some(p) => problem = p,
            },
            Err(p) => problem = p,
        }
        messages.push(Json::obj(vec![("role", Json::str("assistant")), ("content", Json::Str(text))]));
        messages.push(user_message(format!("The app checked this drill and it failed:\n\n{problem}\n\nReturn a corrected drill in the same format.")));
    }
    Err(err(502, format!("The tutor couldn't produce a drill that passed its checks ({problem}). Try again.")))
}

fn save(ctx: &Context, parent: &Parent, d: &Drill, attempts: usize) -> Result<Made, TutorError> {
    let nn: String = parent.slug.chars().take_while(|c| c.is_ascii_digit()).collect();
    let base = format!("{}g-{}", if nn.is_empty() { "00".to_string() } else { nn }, slugify(&d.slug));
    let mut slug = base.clone();
    let mut i = 2;
    while ctx.lessons_dir.join(&slug).exists() {
        slug = format!("{base}-{i}");
        i += 1;
    }
    let mut concepts: Vec<String> = d.concepts.iter().map(|c| slugify(c)).filter(|c| parent.concepts.contains(c)).collect();
    if concepts.is_empty() {
        concepts = parent.concepts.clone();
    }
    let id = format!("{}-{slug}", ctx.id_prefix);
    let minutes = if d.minutes >= 1.0 && d.minutes.fract() == 0.0 { d.minutes } else { 10.0 };
    let front = [
        "---".to_string(),
        format!("id: {id}"),
        format!("track: {}", ctx.track),
        format!("title: {}", one_line(&d.title)),
        "kind: drill".into(),
        format!("for: {}", parent.id),
        format!("minutes: {minutes}"),
        format!("concepts: [{}]", concepts.join(", ")),
        format!("prereqs: [{}]", parent.id),
        "contrast: java".into(),
        "edit: src/lib.rs".into(),
        "generated: true".into(),
        format!("targets: {}", one_line(&d.targets)),
        "---".into(),
        String::new(),
    ]
    .join("\n");
    let dir = ctx.lessons_dir.join(&slug);
    let io = |e: std::io::Error| err(500, e.to_string());
    fs::create_dir_all(dir.join("src")).map_err(io)?;
    fs::write(dir.join("lesson.md"), format!("{front}{}\n", d.explanation.trim())).map_err(io)?;
    fs::write(dir.join("Cargo.toml"), cargo_toml(&slug)).map_err(io)?;
    fs::write(dir.join("src/lib.rs"), with_tests(&d.starter, &d.tests)).map_err(io)?;
    // The reference solution stays local (data/ is gitignored): the learner never sees it.
    let sol = ctx.data_dir.join("solutions").join(&slug);
    fs::create_dir_all(&sol).map_err(io)?;
    fs::write(sol.join("lib.rs"), with_tests(&d.solution, &d.tests)).map_err(io)?;
    Ok(Made { slug, id, title: one_line(&d.title), targets: one_line(&d.targets), attempts })
}

/// What the drill should aim at, from the profile and the latest failures on this lesson.
pub fn drill_target(profile: &Json, parent: &Parent, events: &[Json], lesson_ids: &[String]) -> String {
    let mut lines = Vec::new();
    if let Some(l) = profile.get("lessons").map(Json::as_array).unwrap_or(&[]).iter().find(|l| l.get("id").and_then(Json::as_str) == Some(parent.id.as_str())) {
        let solved = match l.get("first_pass_run").and_then(Json::as_f64) {
            Some(n) => format!("solved on run {n}"),
            None => "not solved yet".into(),
        };
        let rated = l.get("feedback").and_then(Json::as_str).map(|f| format!(", rated \"{f}\"")).unwrap_or_default();
        lines.push(format!("- Their history on \"{}\": {} runs, {solved}{rated}.", parent.title, num(l, "runs")));
    }
    let mut weak: Vec<(String, f64)> = profile
        .get("concepts")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .filter_map(|c| Some((c.get("name")?.as_str()?.to_string(), c.get("mastery")?.as_f64()?)))
        .filter(|(n, _)| parent.concepts.contains(n))
        .collect();
    weak.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if !weak.is_empty() {
        let list: Vec<String> = weak.iter().map(|(n, m)| format!("{n} {m:.2}")).collect();
        lines.push(format!("- Mastery of this lesson's concepts (0–1): {}.", list.join(", ")));
    }
    let recurring: Vec<String> = profile
        .get("recurring")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|r| format!("{} ×{}", r.get("what").and_then(Json::as_str).unwrap_or(""), num(r, "count")))
        .collect();
    if !recurring.is_empty() {
        lines.push(format!("- Recurring mistakes across all exercises: {}.", recurring.join(", ")));
    }
    let fails: Vec<&Json> = events
        .iter()
        .filter(|e| e.get("kind").and_then(Json::as_str) == Some("fail"))
        .filter(|e| e.get("lesson_id").and_then(Json::as_str).map_or(false, |id| lesson_ids.iter().any(|x| x == id)))
        .collect();
    for f in fails.iter().rev().take(3).rev() {
        let Some(p) = f.get("payload") else { continue };
        let what: Vec<&str> = p
            .get("errors")
            .map(Json::as_array)
            .unwrap_or(&[])
            .iter()
            .chain(p.get("failing").map(Json::as_array).unwrap_or(&[]))
            .filter_map(Json::as_str)
            .take(4)
            .collect();
        if !what.is_empty() {
            lines.push(format!("- A recent failing run on this lesson: {}.", what.join("; ")));
        }
    }
    if lines.is_empty() {
        lines.push("- No history on this lesson yet: write a gentle drill covering its main concept.".into());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex as StdMutex};

    /// Tutor tests share the global transport and environment; run them one at a time.
    static SERIAL: StdMutex<()> = StdMutex::new(());

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn context(dir: &Path) -> Context {
        Context {
            docs_dir: root().join("docs"),
            track_name: "Rust".into(),
            learner: "Professional Java developer.".into(),
            profile_markdown: "<!-- profile:start -->\n## Learner profile\nNext: something\n<!-- profile:end -->".into(),
            lessons_dir: dir.join("lessons"),
            data_dir: dir.join("data"),
            track: "rust".into(),
            id_prefix: "rust".into(),
        }
    }

    fn parent() -> Parent {
        let l = crate::lessons::load(&root().join("lessons"), "02-let-mut").unwrap().unwrap();
        Parent {
            id: l.get("id").and_then(Json::as_str).unwrap().into(),
            slug: l.slug.clone(),
            title: l.get("title").and_then(Json::as_str).unwrap().into(),
            concepts: l.get("concepts").unwrap().as_array().iter().filter_map(Json::as_str).map(String::from).collect(),
            body: l.body.clone(),
        }
    }

    fn reply(text: &str) -> Json {
        json::parse(&format!(r#"{{"stop_reason":"end_turn","content":[{{"type":"thinking","thinking":""}},{{"type":"text","text":{}}}]}}"#, Json::str(text).to_string())).unwrap()
    }

    /// A fake transport: records request bodies and returns the replies in order.
    fn fake(replies: Vec<Json>) -> Arc<StdMutex<Vec<Json>>> {
        let seen = Arc::new(StdMutex::new(Vec::new()));
        let queue = StdMutex::new(replies);
        let log = seen.clone();
        set_transport_for_tests(Some(Box::new(move |body: &Json| {
            log.lock().unwrap().push(body.clone());
            Ok(queue.lock().unwrap().remove(0))
        })));
        seen
    }

    fn drill(solution: &str, starter: &str) -> Json {
        Json::obj(vec![
            ("title", Json::str("Totals without surprises")),
            ("slug", Json::str("Totals Without Surprises")),
            ("concepts", Json::Arr(vec![Json::str("mut"), Json::str("shadowing"), Json::str("not-here")])),
            ("minutes", Json::Num(8.0)),
            ("targets", Json::str("Sums of empty slices: #1 mistake so far.")),
            ("explanation", Json::str("# Drill: totals\n\n- sum of nothing is 0\n\n## Exercise\nTwo functions.")),
            ("starter", Json::str(starter)),
            ("tests", Json::str("#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn total_adds() {\n        assert_eq!(total(&[1, 2]), 3);\n    }\n\n    #[test]\n    fn total_of_nothing_is_zero() {\n        assert_eq!(total(&[]), 0);\n    }\n\n    #[test]\n    fn count_counts() {\n        assert_eq!(count(&[5, 6, 7]), 3);\n    }\n}\n")),
            ("solution", Json::str(solution)),
        ])
    }
    const STARTER: &str = "/// Sum: total(&[1, 2]) == 3.\npub fn total(xs: &[i64]) -> i64 {\n    todo!()\n}\n\n/// Count: count(&[5, 6]) == 2.\npub fn count(xs: &[i64]) -> usize {\n    todo!()\n}\n";
    const SOLUTION: &str = "pub fn total(xs: &[i64]) -> i64 {\n    xs.iter().sum()\n}\n\npub fn count(xs: &[i64]) -> usize {\n    xs.len()\n}\n";

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("learn-tutor-test-{name}-{}", std::process::id()));
        fs::remove_dir_all(&d).ok();
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn learner_section_reads_only_the_learners_own_words() {
        let text = "# Progress\n\n## Learner\n- Java pro\n- short sessions\n\n<!-- profile:start -->\nX\n<!-- profile:end -->\n\n## Log\n";
        assert_eq!(learner_section(text), "- Java pro\n- short sessions");
        assert_eq!(learner_section("# nothing"), "(not written yet)");
    }

    #[test]
    fn hint_sends_profile_code_and_result_with_fallbacks() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let seen = fake(vec![reply("Look at the `;` on line 7: E0308.")]);
        let result = json::parse(r#"{"passed":0,"failed":0,"total":0,"tests":[],"compileErrors":[{"code":"E0308","title":"mismatched types","message":"expected f64"}],"timedOut":false}"#).unwrap();
        let text = hint(&context(&std::env::temp_dir()), "T", "Lesson text", "src/lib.rs", "MY CODE", Some(&result), "").unwrap();
        set_transport_for_tests(None);
        assert_eq!(text, "Look at the `;` on line 7: E0308.");
        let req = seen.lock().unwrap()[0].clone();
        assert_eq!(req.get("model"), Some(&Json::str(MODEL)));
        assert_eq!(req.get("fallbacks"), Some(&Json::str("default")));
        assert_eq!(req.get("output_config"), Some(&Json::obj(vec![("effort", Json::str("medium"))])));
        let system = req.get("system").and_then(Json::as_str).unwrap();
        assert!(system.contains("Professional Java developer.") && system.contains("Next: something") && !system.contains("profile:start"));
        let user = req.get("messages").unwrap().as_array()[0].get("content").and_then(Json::as_str).unwrap().to_string();
        assert!(user.contains("MY CODE") && user.contains("E0308 mismatched types"));
    }

    #[test]
    fn refusal_becomes_a_clear_error() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        fake(vec![json::parse(r#"{"stop_reason":"refusal","content":[]}"#).unwrap()]);
        let r = hint(&context(&std::env::temp_dir()), "T", "", "f", "", None, "q");
        set_transport_for_tests(None);
        assert_eq!(r.unwrap_err().message, "The tutor declined this request.");
    }

    #[test]
    fn generated_drill_is_verified_saved_and_loadable() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp("save");
        let seen = fake(vec![reply(&drill(SOLUTION, STARTER).to_string()), reply(&drill(SOLUTION, STARTER).to_string())]);
        let made = generate_drill(&context(&dir), &parent(), "(none)", "- E0308 ×3").unwrap();
        let again = generate_drill(&context(&dir), &parent(), "", "").unwrap();
        set_transport_for_tests(None);
        assert_eq!(made.slug, "02g-totals-without-surprises");
        assert_eq!(made.id, "rust-02g-totals-without-surprises");
        assert_eq!(made.attempts, 1);
        assert_eq!(again.slug, "02g-totals-without-surprises-2");
        let req = seen.lock().unwrap()[0].clone();
        let oc = req.get("output_config").unwrap();
        assert_eq!(oc.get("effort"), Some(&Json::str("high")));
        assert_eq!(oc.get("format").and_then(|f| f.get("type")), Some(&Json::str("json_schema")));
        let user = req.get("messages").unwrap().as_array()[0].get("content").and_then(Json::as_str).unwrap().to_string();
        assert!(user.contains("E0308 ×3") && user.contains("#[cfg(test)]"));

        let saved = crate::lessons::load(&dir.join("lessons"), &made.slug).unwrap().unwrap();
        assert_eq!(saved.get("kind"), Some(&Json::str("drill")));
        assert_eq!(saved.get("for"), Some(&Json::str("rust-02-let-mut")));
        assert_eq!(saved.get("concepts"), Some(&Json::Arr(vec![Json::str("mut"), Json::str("shadowing")])));
        assert_eq!(saved.get("targets"), Some(&Json::str("Sums of empty slices: 1 mistake so far.")));
        assert!(saved.starter.contains("todo!()") && saved.starter.contains("fn total_adds"));
        assert!(fs::read_to_string(dir.join("lessons").join(&made.slug).join("Cargo.toml")).unwrap().contains("lesson-02g-totals-without-surprises"));
        assert!(dir.join("data/solutions").join(&made.slug).join("lib.rs").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failed_checks_are_sent_back_once_then_it_gives_up() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp("retry");
        let wrong = SOLUTION.replace("xs.iter().sum()", "0");
        let seen = fake(vec![reply(&drill(&wrong, STARTER).to_string()), reply(&drill(SOLUTION, STARTER).to_string())]);
        let made = generate_drill(&context(&dir), &parent(), "", "").unwrap();
        assert_eq!(made.attempts, 2);
        let retry = seen.lock().unwrap()[1].get("messages").unwrap().as_array().to_vec();
        assert_eq!(retry.len(), 3);
        let note = retry[2].get("content").and_then(Json::as_str).unwrap();
        assert!(note.contains("solution does not pass") && note.contains("FAILED total adds"), "{note}");

        // Tests that already pass against the starter check nothing; garbage isn't a drill.
        fake(vec![reply(&drill(SOLUTION, SOLUTION).to_string()), reply("not json")]);
        let r = generate_drill(&context(&dir), &parent(), "", "");
        set_transport_for_tests(None);
        assert!(r.err().unwrap().message.contains("couldn't produce a drill"));
        fs::remove_dir_all(&dir).ok();
    }

    /// The real HTTP path (curl), against a local server standing in for the API.
    #[test]
    fn curl_transport_sends_key_headers_and_body() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                head.push(line.trim().to_string());
            }
            let len: usize = head.iter().find_map(|h| h.to_lowercase().strip_prefix("content-length: ").map(|v| v.parse().unwrap())).unwrap();
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            let reply = r#"{"stop_reason":"end_turn","content":[{"type":"text","text":"hello from the fake API"}]}"#;
            let mut out = stream;
            write!(out, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).unwrap();
            (head, String::from_utf8(body).unwrap())
        });
        std::env::set_var("ANTHROPIC_BASE_URL", format!("http://127.0.0.1:{port}"));
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");
        let text = hint(&context(&std::env::temp_dir()), "T", "", "f", "code", None, "why?");
        std::env::remove_var("ANTHROPIC_BASE_URL");
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert_eq!(text.unwrap(), "hello from the fake API");
        let (head, body) = server.join().unwrap();
        assert!(head[0].starts_with("POST /v1/messages "), "{head:?}");
        for h in ["x-api-key: test-key-123", "anthropic-version: 2023-06-01", "anthropic-beta: server-side-fallback-2026-07-01", "content-type: application/json"] {
            assert!(head.iter().any(|l| l.eq_ignore_ascii_case(h)), "missing {h} in {head:?}");
        }
        let sent = json::parse(&body).unwrap();
        assert_eq!(sent.get("model"), Some(&Json::str(MODEL)));
        assert!(sent.get("messages").unwrap().as_array()[0].get("content").and_then(Json::as_str).unwrap().contains("why?"));
        assert!(fs::read_dir(std::env::temp_dir()).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".curl")), "curl config with the key was left behind");
    }

    /// A reply in visual-schema.json's shape: every field present, "" and 0 meaning "not set".
    fn trace_reply(call: &str, prints: &str, arrow_to: &str, line: f64) -> String {
        let s = |v: &str| Json::str(v);
        let bx = |id: &str, inn: &str, title: &str, value: &str, state: &str| {
            Json::obj(vec![("id", s(id)), ("in", s(inn)), ("title", s(title)), ("value", s(value)), ("tag", s("")), ("sub", s("")), ("state", s(state))])
        };
        let step = |line: f64, say: &str, rule: &str, error: &str, set: Vec<Json>, arrows: Vec<Json>| {
            Json::obj(vec![("line", Json::Num(line)), ("say", s(say)), ("rule", s(rule)), ("error", s(error)), ("set", Json::Arr(set)), ("drop", Json::Arr(vec![])), ("arrows", Json::Arr(arrows))])
        };
        let arrow = Json::obj(vec![("from", s("n")), ("to", s(arrow_to)), ("label", s("format!")), ("state", s("bad")), ("style", s(""))]);
        Json::obj(vec![
            ("title", s("What your greeting() does with \"Sam\"")),
            ("intro", s("Watch the text being built.")),
            (
                "regions",
                Json::Arr(vec![
                    Json::obj(vec![("id", s("fn")), ("label", s("Inside greeting(name)")), ("kind", s(""))]),
                    Json::obj(vec![("id", s("out")), ("label", s("Console")), ("kind", s("output"))]),
                ]),
            ),
            (
                "scenario",
                Json::obj(vec![
                    ("name", s("Your code with \"Sam\"")),
                    ("code", s("pub fn greeting(name: &str) -> String {\n    format!(\"Hello {name}\")\n}\nprintln!(\"{}\", greeting(\"Sam\"));")),
                    (
                        "steps",
                        Json::Arr(vec![
                            step(4.0, "The test calls greeting(\"Sam\").", "", "", vec![bx("n", "fn", "name", "\"Sam\"", "active")], vec![]),
                            step(line, "format! builds \"Hello Sam\": no comma, no \"!\".", "", "", vec![bx("r", "fn", "result", "\"Hello Sam\"", "bad")], vec![arrow]),
                            step(0.0, "The test wanted \"Hello, Sam!\".", "Put the comma and ! inside the format string.", "Expected \"Hello, Sam!\", got \"Hello Sam\"", vec![bx("o", "out", "", "Hello Sam", "")], vec![]),
                        ]),
                    ),
                ]),
            ),
            ("check", Json::obj(vec![("call", s(call)), ("prints", s(prints))])),
        ])
        .to_string()
    }

    /// A small lesson made on the fly: the basics lessons don't have a greeting exercise.
    fn greeting_lesson(dir: &Path) -> Lesson {
        let ldir = dir.join("greeting");
        fs::create_dir_all(ldir.join("src")).unwrap();
        fs::write(ldir.join("Cargo.toml"), cargo_toml("greeting")).unwrap();
        let starter = "pub fn greeting(name: &str) -> String {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn says_hello_to_sam() {\n        assert_eq!(greeting(\"Sam\"), \"Hello, Sam!\");\n    }\n}\n";
        fs::write(ldir.join("src/lib.rs"), starter).unwrap();
        let body = "# Greeting\n\nReturn a greeting.\n\n```visual\n{\"title\": \"x\"}\n```\n".to_string();
        Lesson { slug: "greeting".into(), dir: ldir, meta: vec![], body, file: "src/lib.rs".into(), starter: starter.into() }
    }

    const ALMOST: &str = "pub fn greeting(name: &str) -> String {\n    format!(\"Hello {name}\")\n}\n";

    #[test]
    fn visualize_checks_the_trace_on_the_real_code_and_sends_a_wrong_claim_back_once() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp("trace");
        let lesson = greeting_lesson(&dir);
        let code = lesson.starter.replacen("    todo!()", "    format!(\"Hello {name}\")", 1);
        let result = runner::run_lesson(&lesson, &code, &dir.join("data")).unwrap();
        let seen = fake(vec![reply(&trace_reply("greeting(\"Sam\")", "\"Hello, Sam!\"", "r", 2.0)), reply(&trace_reply("greeting(\"Sam\")", "\"Hello Sam\"", "r", 2.0))]);
        let made = visualize_run(&context(&dir), "Your first Rust function", &lesson.body, &lesson.file, &code, Some(&result));
        set_transport_for_tests(None);
        let made = made.unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!(made.attempts, 2);
        assert_eq!((made.call.as_str(), made.prints.as_str()), ("greeting(\"Sam\")", "\"Hello Sam\""));
        let sc = &made.visual.get("scenarios").unwrap().as_array()[0];
        assert_eq!(sc.get("group"), Some(&Json::str("Your code")));
        let steps = sc.get("steps").unwrap().as_array();
        assert_eq!(steps[2].get("line"), None, "line 0 means no line");
        assert_eq!(steps[0].get("rule"), None, "empty strings are dropped");
        assert_eq!(steps[0].get("set").unwrap().as_array()[0].to_string(), r#"{"id":"n","in":"fn","title":"name","value":"\"Sam\"","state":"active"}"#);
        assert_eq!(made.visual.get("regions").unwrap().as_array()[0].get("kind"), None);

        let requests = seen.lock().unwrap().clone();
        let oc = requests[0].get("output_config").unwrap();
        assert_eq!(oc.get("effort"), Some(&Json::str("medium")));
        assert!(oc.get("format").and_then(|f| f.get("schema")).and_then(|s| s.get("properties")).and_then(|p| p.get("check")).is_some());
        let prompt = requests[0].get("messages").unwrap().as_array()[0].get("content").and_then(Json::as_str).unwrap().to_string();
        assert!(prompt.contains("format!(\"Hello {name}\")") && prompt.contains("FAILED"), "sends their code and the failing tests");
        assert!(!prompt.contains("```visual"), "the lesson text is sent without its visual blocks");
        let msgs = requests[1].get("messages").unwrap().as_array();
        let sent_back = msgs[msgs.len() - 1].get("content").and_then(Json::as_str).unwrap();
        assert!(sent_back.contains("really gave") && sent_back.contains("Hello Sam"), "{sent_back}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn visualize_never_returns_a_visual_the_player_cannot_show() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp("trace-bad");
        let seen = fake(vec![reply(&trace_reply("greeting(\"Sam\")", "\"Hello Sam\"", "nowhere", 2.0)), reply(&trace_reply("greeting(\"Sam\")", "\"Hello Sam\"", "r", 9.0))]);
        let r = visualize_run(&context(&dir), "T", "# L", "src/lib.rs", ALMOST, None);
        set_transport_for_tests(None);
        assert!(r.err().unwrap().message.contains("couldn't trace"));
        let requests = seen.lock().unwrap().clone();
        let msgs = requests[1].get("messages").unwrap().as_array();
        assert!(msgs[msgs.len() - 1].get("content").and_then(Json::as_str).unwrap().contains("arrow n → nowhere points at a missing box"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn visualize_traces_code_that_does_not_compile_to_its_real_error() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp("trace-e0308");
        fake(vec![reply(&trace_reply("", "E0308", "r", 2.0))]);
        let code = ALMOST.replace("\")\n", "\");\n");
        let r = visualize_run(&context(&dir), "T", "# L", "src/lib.rs", &code, None);
        set_transport_for_tests(None);
        let made = r.unwrap_or_else(|e| panic!("{}", e.message));
        assert_eq!((made.call.as_str(), made.prints.as_str()), ("", "E0308"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn split_visuals_keeps_the_text_and_the_first_visual() {
        let (text, first) = split_visuals("# A\n\n```visual\n{\"x\":1}\n```\n\nmore\n```visual\n{\"y\":2}\n```\nend");
        assert_eq!(text, "# A\n\n\n\nmore\n\nend");
        assert_eq!(first.as_deref(), Some("{\"x\":1}"));
    }

    #[test]
    fn env_file_wins_and_a_rejected_key_is_explained_without_revealing_it() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let saved = (std::env::var("ANTHROPIC_API_KEY").ok(), std::env::var("ANTHROPIC_BASE_URL").ok());
        std::env::remove_var("ANTHROPIC_BASE_URL");
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-oat01-old-shell-token-xyz");
        let shell = key_problem("invalid x-api-key");
        assert!(shell.contains("your shell environment (not .env)") && shell.contains("subscription login token"), "{shell}");

        let dir = temp("env");
        let real = format!("sk-ant-api03-{}Wxyz", "A".repeat(80));
        fs::write(dir.join(".env"), format!("# my key\nANTHROPIC_API_KEY=\"{real}\"\n")).unwrap();
        load_env_file(&dir.join(".env"));
        assert_eq!(std::env::var("ANTHROPIC_API_KEY").unwrap(), real);
        let msg = key_problem("invalid x-api-key");
        assert!(msg.contains("rejected (\"invalid x-api-key\"). The app used the key from .env: sk-ant-api03…Wxyz (97 characters)."), "{msg}");
        assert!(!msg.contains(&"A".repeat(10)), "never shows the key");
        assert!(msg.contains("console.anthropic.com"));

        std::env::set_var("ANTHROPIC_BASE_URL", "http://localhost:9999");
        assert!(key_problem("").contains("Requests go to http://localhost:9999 (ANTHROPIC_BASE_URL in your shell)"));
        for (k, v) in [("ANTHROPIC_API_KEY", saved.0), ("ANTHROPIC_BASE_URL", saved.1)] {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        fs::remove_dir_all(&dir).ok();
    }
}
