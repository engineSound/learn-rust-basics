# Design

## What it is
A small web app I run on my Mac and later on AWS. It shows the next lesson,
lets me write Rust in the browser, compiles and tests it, and remembers
everything so a tutor (Claude) can adapt to me.

## Parts

```
browser ──► std-only HTTP server ──► runner (cargo test in a lesson copy, 120 s timeout)
                 │
                 ├──► data/events.jsonl  (event log, see SHARED-FORMAT.md)
                 ├──► profile → PROGRESS.md (generated block)
                 └──► tutor: Claude API (profile + lesson + my code + compiler output)
```

- **No dependencies.** The server (`app/src/`) uses only the standard
  library: a small HTTP server, a JSON reader and writer, the lesson loader,
  the runner and the history log. No crates to download, fast to build. (Axum and Tokio come later as
  lessons, not as plumbing.)
- **Web UI** (`app/public/`, shared with `learn-nodejs`): plain HTML, CSS and
  JS. A textarea editor with line numbers, a small Markdown renderer for
  lessons, and a results panel. Works offline.
- **Runner** (`app/src/runner.rs`): copies the lesson to
  `data/runs/<slug>/`, writes my code into the `edit:` file, and runs
  `cargo test` with a shared `data/target/`, so rebuilds take about a second.
  It parses compiler errors (keeping codes like `E0382`, dropping warnings)
  and libtest results into JSON. One run at a time.
- **Profile** (`app/src/profile.rs`): derives concept mastery, the review
  schedule, recurring mistakes and the next step from the events (rules in
  `docs/SHARED-FORMAT.md`), and rewrites the generated block in `PROGRESS.md`
  after every run.
- **Server** (`app/src/main.rs`): the API in `docs/SHARED-FORMAT.md`.
  Listens on 127.0.0.1 only unless `HOST` is set.
- **History** (`app/history.rs`): every lesson open and every run is
  appended to `data/events.jsonl`; per-lesson stats are derived from it on
  each request. The file lives only on the machine running the app (it's
  gitignored). The learner profile is the summary that gets committed.
  Compiler error codes are stored with each `fail` event, because repeated
  codes are the clearest sign of what to review.
- **Tutor** (`app/tutor.rs`, optional): hints and newly generated,
  verified drills from Claude, using the shared prompts in `docs/tutor/`.
  Rules in `docs/SHARED-FORMAT.md`, section 6. API key in `.env`
  (`ANTHROPIC_API_KEY`), never committed.

## Running it
- **Mac (now):** `cargo run`, open http://localhost:3003.
- **AWS (later):** the same app in a Docker container (with the Rust
  toolchain) on an existing EC2 instance, behind HTTPS and a login, because it
  compiles and runs code.

## Safety
The runner executes code I wrote. Only I use it, but it still runs with a
timeout, in a scratch directory, and in a container on AWS.
