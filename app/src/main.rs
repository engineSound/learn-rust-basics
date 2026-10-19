//! learn-rust web app. Standard library only: no dependencies to download.
//! Start with `cargo run`, then open http://localhost:3003.

mod history;
mod json;
mod lessons;
mod profile;
mod tutor;
mod runner;

use json::Json;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

const MAX_BODY: usize = 256 * 1024;

struct Paths {
    lessons: PathBuf,
    data: PathBuf,
    events: PathBuf,
    progress: PathBuf,
    public: PathBuf,
}

const TRACK: &str = "rust";

struct Request {
    method: String,
    path: String,
    body: String,
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn json(status: u16, value: Json) -> Response {
        Response { status, content_type: "application/json; charset=utf-8", body: value.to_string().into_bytes() }
    }

    fn error(status: u16, message: &str) -> Response {
        Response::json(status, Json::obj(vec![("error", Json::str(message))]))
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repo root").to_path_buf();
    // ANTHROPIC_API_KEY for the tutor lives in .env (gitignored); see .env.example.
    tutor::load_env_file(&root.join(".env"));
    let paths: &'static Paths = Box::leak(Box::new(Paths {
        lessons: root.join("lessons"),
        data: root.join("data"),
        events: std::env::var_os("LEARN_EVENTS").map(PathBuf::from).unwrap_or_else(|| root.join("data").join("events.jsonl")),
        progress: std::env::var_os("LEARN_PROGRESS").map(PathBuf::from).unwrap_or_else(|| root.join("PROGRESS.md")),
        public: root.join("app").join("public"),
    }));
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3003".into());
    let listener = TcpListener::bind(format!("{host}:{port}")).unwrap_or_else(|e| panic!("can't listen on {host}:{port}: {e}"));
    println!("Learn Rust running at http://{}:{port}", if host == "0.0.0.0" { "localhost" } else { &host });

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            if let Err(e) = serve(stream, paths) {
                eprintln!("request failed: {e}");
            }
        });
    }
}

fn serve(mut stream: TcpStream, paths: &Paths) -> std::io::Result<()> {
    let response = match read_request(&stream) {
        Ok(req) => handle(&req, paths),
        Err(msg) => Response::error(400, &msg),
    };
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        401 => "Unauthorized",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn read_request(stream: &TcpStream) -> Result<Request, String> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    let mut parts = line.split_whitespace();
    let method = parts.next().ok_or("empty request")?.to_string();
    let target = parts.next().ok_or("no path")?;
    let path = target.split('?').next().unwrap_or("/").to_string();

    let mut length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).map_err(|e| e.to_string())?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().map_err(|_| "bad Content-Length")?;
            }
        }
    }
    if length > MAX_BODY {
        return Err("Code is too large".into());
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    let body = String::from_utf8(body).map_err(|_| "body is not UTF-8")?;
    Ok(Request { method, path, body })
}

fn work_path(paths: &Paths, lesson: &lessons::Lesson) -> PathBuf {
    paths.data.join("work").join(&lesson.slug).join(&lesson.file)
}

fn save_code(paths: &Paths, lesson: &lessons::Lesson, code: &str) -> std::io::Result<()> {
    let p = work_path(paths, lesson);
    fs::create_dir_all(p.parent().expect("work dir"))?;
    fs::write(p, code)
}

fn profile_of(paths: &Paths, events: &[Json]) -> Json {
    let refs: Vec<profile::LessonRef> = lessons::list(&paths.lessons)
        .iter()
        .map(|l| profile::LessonRef {
            id: l.get("id").and_then(Json::as_str).unwrap_or(&l.slug).to_string(),
            title: l.get("title").and_then(Json::as_str).unwrap_or(&l.slug).to_string(),
            concepts: l.get("concepts").map(Json::as_array).unwrap_or(&[]).iter().filter_map(Json::as_str).map(String::from).collect(),
            kind: l.get("kind").and_then(Json::as_str).unwrap_or("core").to_string(),
            parent: l.get("for").and_then(Json::as_str).map(String::from),
        })
        .collect();
    profile::build_profile(TRACK, &refs, events, &history::now_iso())
}

/// The profile's next step plus the lesson folder (slug) the UI needs to open it.
fn next_with_slug(paths: &Paths, profile: &Json) -> Json {
    let mut next = profile.get("next").cloned().unwrap_or(Json::Null);
    let id = next.get("lesson_id").cloned();
    let slug = lessons::list(&paths.lessons).into_iter().find(|l| l.get("id") == id.as_ref()).map(|l| Json::Str(l.slug)).unwrap_or(Json::Null);
    if let Json::Obj(fields) = &mut next {
        fields.push(("slug".to_string(), slug));
    }
    next
}

/// History stats for one lesson plus its review schedule from the profile.
fn stats_for(events: &[Json], profile: &Json, lesson_id: Option<&Json>) -> Json {
    let mut stats = history::lesson_stats(events, lesson_id);
    let entry = profile.get("lessons").map(Json::as_array).unwrap_or(&[]).iter().find(|l| l.get("id") == lesson_id);
    if let Json::Obj(fields) = &mut stats {
        for key in ["review_step", "due"] {
            fields.push((key.to_string(), entry.and_then(|e| e.get(key)).cloned().unwrap_or(Json::Null)));
        }
        fields.push(("due_now".to_string(), entry.and_then(|e| e.get("due_now")).cloned().unwrap_or(Json::Bool(false))));
    }
    stats
}

/// After a run, reset or rating: refresh the generated profile block in PROGRESS.md.
/// Returns the lesson's `stats` and the adaptive `next` step.
fn refresh_progress(paths: &Paths, lesson_id: Option<&Json>) -> Vec<(String, Json)> {
    let events = history::read_events(&paths.events);
    let profile = profile_of(paths, &events);
    if let Err(e) = profile::write_progress(&paths.progress, &profile::profile_markdown(&profile)) {
        eprintln!("couldn't update PROGRESS.md: {e}");
    }
    vec![("stats".to_string(), stats_for(&events, &profile, lesson_id)), ("next".to_string(), next_with_slug(paths, &profile))]
}

/// Generate, verify and save a new drill for this exercise's core lesson.
fn make_drill(paths: &Paths, lesson: &lessons::Lesson) -> Response {
    let all = lessons::list(&paths.lessons);
    let kind = lesson.get("kind").and_then(Json::as_str).unwrap_or("core");
    let parent_id = if kind == "core" { lesson.get("id") } else { lesson.get("for") }.and_then(Json::as_str).unwrap_or("").to_string();
    let Some(parent) = all.iter().find(|l| l.get("id").and_then(Json::as_str) == Some(parent_id.as_str())) else {
        return Response::error(400, "This exercise has no core lesson to practise.");
    };
    let children: Vec<&lessons::Lesson> = all.iter().filter(|l| l.get("for").and_then(Json::as_str) == Some(parent_id.as_str())).collect();
    let existing: Vec<String> = std::iter::once(parent)
        .chain(children.iter().copied())
        .map(|l| format!("### {} ({})\n\n```\n{}\n```", l.get("title").and_then(Json::as_str).unwrap_or(&l.slug), l.get("kind").and_then(Json::as_str).unwrap_or("core"), l.starter))
        .collect();
    let concepts: Vec<String> = parent.get("concepts").map(Json::as_array).unwrap_or(&[]).iter().filter_map(Json::as_str).map(String::from).collect();
    let target_parent = tutor::Parent {
        id: parent_id.clone(),
        slug: parent.slug.clone(),
        title: parent.get("title").and_then(Json::as_str).unwrap_or(&parent.slug).to_string(),
        concepts,
        body: parent.body.clone(),
    };
    let events = history::read_events(&paths.events);
    let profile = profile_of(paths, &events);
    let mut ids = vec![parent_id.clone()];
    ids.extend(children.iter().filter_map(|c| c.get("id").and_then(Json::as_str)).map(String::from));
    let target = tutor::drill_target(&profile, &target_parent, &events, &ids);
    match tutor::generate_drill(&tutor_context(paths, &profile), &target_parent, &existing.join("\n\n"), &target) {
        Err(e) => tutor_error(e),
        Ok(made) => {
            let id = Json::str(made.id.clone());
            let payload = Json::obj(vec![("for", Json::str(parent_id)), ("targets", Json::str(made.targets.clone())), ("attempts", Json::Num(made.attempts as f64))]);
            if let Err(e) = history::append_event(&paths.events, TRACK, Some(&id), "generated", None, payload) {
                eprintln!("couldn't record history: {e}");
            }
            let mut fields = vec![
                ("slug".to_string(), Json::Str(made.slug)),
                ("id".to_string(), id.clone()),
                ("title".to_string(), Json::Str(made.title)),
                ("targets".to_string(), Json::Str(made.targets)),
                ("attempts".to_string(), Json::Num(made.attempts as f64)),
            ];
            fields.extend(refresh_progress(paths, Some(&id)));
            Response::json(200, Json::Obj(fields))
        }
    }
}

/// What the tutor needs to know about the learner (system prompt).
fn tutor_context(paths: &Paths, profile: &Json) -> tutor::Context {
    let progress = fs::read_to_string(&paths.progress).unwrap_or_default();
    tutor::Context {
        docs_dir: paths.lessons.parent().expect("repo root").join("docs"),
        track_name: "Rust".into(),
        learner: tutor::learner_section(&progress),
        profile_markdown: profile::profile_markdown(profile),
        lessons_dir: paths.lessons.clone(),
        data_dir: paths.data.clone(),
        track: TRACK.into(),
        id_prefix: "rust".into(),
    }
}

fn tutor_error(e: tutor::TutorError) -> Response {
    Response::error(e.status, &e.message)
}

fn handle(req: &Request, paths: &Paths) -> Response {
    let method = req.method.as_str();
    let path = req.path.as_str();

    if method == "GET" && path == "/api/info" {
        return Response::json(
            200,
            Json::obj(vec![("track", Json::str("rust")), ("title", Json::str("Rust Basics")), ("accent", Json::str("#8a3ffc")), ("tutor", tutor::status())]),
        );
    }
    if method == "GET" && path == "/api/lessons" {
        let events = history::read_events(&paths.events);
        let profile = profile_of(paths, &events);
        let list = lessons::list(&paths.lessons)
            .into_iter()
            .map(|l| {
                Json::obj(vec![
                    ("slug", Json::str(l.slug.clone())),
                    ("id", l.get("id").cloned().unwrap_or(Json::Null)),
                    ("title", l.get("title").cloned().unwrap_or_else(|| Json::str(l.slug.clone()))),
                    ("minutes", l.get("minutes").cloned().unwrap_or(Json::Null)),
                    ("concepts", l.get("concepts").cloned().unwrap_or(Json::Arr(vec![]))),
                    ("kind", l.get("kind").cloned().unwrap_or_else(|| Json::str("core"))),
                    ("for", l.get("for").cloned().unwrap_or(Json::Null)),
                    ("topic", l.get("topic").cloned().unwrap_or(Json::Null)),
                    ("stats", stats_for(&events, &profile, l.get("id"))),
                ])
            })
            .collect();
        return Response::json(200, Json::Arr(list));
    }
    if method == "POST" && path == "/api/snippet" {
        let body = json::parse(if req.body.trim().is_empty() { "{}" } else { &req.body }).unwrap_or(Json::Obj(vec![]));
        let code = body.get("code").and_then(Json::as_str).unwrap_or("").to_string();
        return runner::run_snippet(&code, &paths.data).map(|r| Response::json(200, r)).unwrap_or_else(|e| Response::error(500, &e.to_string()));
    }
    if method == "GET" && path == "/api/history" {
        let events = history::read_events(&paths.events);
        let list: Vec<Json> = events
            .iter()
            .rev()
            .filter(|e| e.get("kind").and_then(Json::as_str) != Some("lesson_open"))
            .take(300)
            .map(|e| match e {
                Json::Obj(fields) => Json::Obj(
                    fields
                        .iter()
                        .map(|(k, v)| match (k.as_str(), v) {
                            ("payload", Json::Obj(p)) => (k.clone(), Json::Obj(p.iter().filter(|(pk, _)| pk != "code").cloned().collect())),
                            _ => (k.clone(), v.clone()),
                        })
                        .collect(),
                ),
                other => other.clone(),
            })
            .collect();
        return Response::json(200, Json::Arr(list));
    }
    if method == "GET" && path == "/api/profile" {
        let profile = profile_of(paths, &history::read_events(&paths.events));
        let markdown = profile::profile_markdown(&profile);
        let next = next_with_slug(paths, &profile);
        return Response::json(200, Json::obj(vec![("profile", profile), ("markdown", Json::Str(markdown)), ("next", next)]));
    }

    if let Some(rest) = path.strip_prefix("/api/lessons/") {
        let (slug, action) = match rest.split_once('/') {
            Some((s, a)) => (s, a),
            None => (rest, ""),
        };
        let lesson = match lessons::load(&paths.lessons, slug) {
            Ok(Some(l)) => l,
            Ok(None) => return Response::error(404, "No such lesson"),
            Err(e) => return Response::error(500, &e),
        };
        let id = lesson.get("id").cloned();
        let log = |kind: &str, duration: Option<f64>, payload: Json| {
            if let Err(e) = history::append_event(&paths.events, TRACK, id.as_ref(), kind, duration, payload) {
                eprintln!("couldn't record history: {e}");
            }
        };
        let stats = || {
            let events = history::read_events(&paths.events);
            stats_for(&events, &profile_of(paths, &events), id.as_ref())
        };
        let result: std::io::Result<Response> = match (method, action) {
            ("GET", "") => {
                let saved = fs::read_to_string(work_path(paths, &lesson)).ok();
                log("lesson_open", None, Json::Null);
                Ok(Response::json(
                    200,
                    Json::obj(vec![
                        ("slug", Json::str(lesson.slug.clone())),
                        ("meta", Json::Obj(lesson.meta.clone())),
                        ("body", Json::str(lesson.body.clone())),
                        ("file", Json::str(lesson.file.clone())),
                        ("saved", Json::Bool(saved.is_some())),
                        ("code", Json::Str(saved.unwrap_or_else(|| lesson.starter.clone()))),
                        ("stats", stats()),
                    ]),
                ))
            }
            ("PUT", "code") => save_code(paths, &lesson, &req.body).map(|_| Response::json(200, Json::obj(vec![("saved", Json::Bool(true))]))),
            ("DELETE", "code") => {
                fs::remove_file(work_path(paths, &lesson)).ok();
                log("reset", None, Json::Null);
                let mut fields = vec![("code".to_string(), Json::str(lesson.starter.clone()))];
                fields.extend(refresh_progress(paths, id.as_ref()));
                Ok(Response::json(200, Json::Obj(fields)))
            }
            ("POST", "run") => save_code(paths, &lesson, &req.body)
                .and_then(|_| runner::run_lesson(&lesson, &req.body, &paths.data))
                .map(|result| {
                    let kind = if history::is_pass(&result) { "pass" } else { "fail" };
                    let duration = result.get("durationMs").and_then(Json::as_f64);
                    log(kind, duration, history::run_payload(&result, &req.body));
                    let mut result = result;
                    if let Json::Obj(fields) = &mut result {
                        fields.extend(refresh_progress(paths, id.as_ref()));
                    }
                    Response::json(200, result)
                }),
            ("POST", "feedback") => {
                let difficulty = req.body.trim();
                if !["easy", "right", "hard"].contains(&difficulty) {
                    return Response::error(400, "Rate it easy, right or hard");
                }
                log("feedback", None, Json::obj(vec![("difficulty", Json::str(difficulty))]));
                Ok(Response::json(200, Json::Obj(refresh_progress(paths, id.as_ref()))))
            }
            ("POST", "hint") => {
                let body = json::parse(if req.body.trim().is_empty() { "{}" } else { &req.body }).unwrap_or(Json::Obj(vec![]));
                let code = body.get("code").and_then(Json::as_str).unwrap_or("").to_string();
                let question = body.get("question").and_then(Json::as_str).unwrap_or("").to_string();
                save_code(paths, &lesson, &code).and_then(|_| runner::run_lesson(&lesson, &code, &paths.data)).map(|result| {
                    let events = history::read_events(&paths.events);
                    let ctx = tutor_context(paths, &profile_of(paths, &events));
                    let title = lesson.get("title").and_then(Json::as_str).unwrap_or(&lesson.slug).to_string();
                    match tutor::hint(&ctx, &title, &lesson.body, &lesson.file, &code, Some(&result), &question) {
                        Ok(text) => {
                            let q = if question.trim().is_empty() { Json::Null } else { Json::str(question.trim()) };
                            log("hint", None, Json::obj(vec![("source", Json::str("tutor")), ("question", q)]));
                            Response::json(200, Json::obj(vec![("hint", Json::Str(text))]))
                        }
                        Err(e) => tutor_error(e),
                    }
                })
            }
            ("POST", "visualize") => {
                let body = json::parse(if req.body.trim().is_empty() { "{}" } else { &req.body }).unwrap_or(Json::Obj(vec![]));
                let code = body.get("code").and_then(Json::as_str).unwrap_or("").to_string();
                save_code(paths, &lesson, &code).and_then(|_| runner::run_lesson(&lesson, &code, &paths.data)).map(|result| {
                    if history::is_pass(&result) {
                        return Response::error(400, "Every check passes now: nothing to trace.");
                    }
                    if result.get("timedOut").and_then(Json::as_bool) == Some(true) {
                        return Response::error(400, "Your code runs forever, so there is nothing to trace yet. Look for a loop that never ends.");
                    }
                    let events = history::read_events(&paths.events);
                    let ctx = tutor_context(paths, &profile_of(paths, &events));
                    let title = lesson.get("title").and_then(Json::as_str).unwrap_or(&lesson.slug).to_string();
                    match tutor::visualize_run(&ctx, &title, &lesson.body, &lesson.file, &code, Some(&result)) {
                        Ok(trace) => {
                            log("hint", None, Json::obj(vec![("source", Json::str("visual")), ("question", Json::Null)]));
                            Response::json(
                                200,
                                Json::obj(vec![
                                    ("visual", trace.visual),
                                    ("verified", Json::obj(vec![("call", Json::Str(trace.call)), ("prints", Json::Str(trace.prints))])),
                                    ("attempts", Json::Num(trace.attempts as f64)),
                                ]),
                            )
                        }
                        Err(e) => tutor_error(e),
                    }
                })
            }
            ("POST", "drill") => Ok(make_drill(paths, &lesson)),
            ("POST", "log") => {
                let body = json::parse(if req.body.trim().is_empty() { "{}" } else { &req.body }).unwrap_or(Json::Obj(vec![]));
                let p = body.get("payload").cloned().unwrap_or(Json::Obj(vec![]));
                let num = |k: &str| p.get(k).and_then(Json::as_f64).map(Json::Num).unwrap_or(Json::Null);
                match body.get("kind").and_then(Json::as_str) {
                    Some("hint") => {
                        log("hint", None, Json::obj(vec![("source", Json::str("lesson")), ("level", num("level")), ("question", Json::Null)]));
                        Ok(Response::json(200, Json::obj(vec![("ok", Json::Bool(true))])))
                    }
                    Some("predict") => {
                        let correct = p.get("correct").and_then(Json::as_bool) == Some(true);
                        log("predict", None, Json::obj(vec![("question", num("question")), ("correct", Json::Bool(correct))]));
                        Ok(Response::json(200, Json::obj(vec![("ok", Json::Bool(true))])))
                    }
                    _ => Ok(Response::error(400, "Only hint and predict can be logged")),
                }
            }
            _ => Ok(Response::error(405, "Method not allowed")),
        };
        return result.unwrap_or_else(|e| Response::error(500, &e.to_string()));
    }

    if method == "GET" {
        let file = if path == "/" { "index.html" } else { &path[1..] };
        let safe = !file.is_empty() && file.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') && !file.starts_with('.');
        if safe {
            if let Ok(body) = fs::read(paths.public.join(file)) {
                let content_type = match file.rsplit('.').next() {
                    Some("html") => "text/html; charset=utf-8",
                    Some("css") => "text/css; charset=utf-8",
                    Some("js") => "text/javascript; charset=utf-8",
                    _ => "application/octet-stream",
                };
                return Response { status: 200, content_type, body };
            }
        }
    }
    Response::error(404, "Not found")
}
