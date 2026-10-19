# Shared format

`learn-nodejs`, `learn-rust`, `learn-nodejs-basics` and `learn-rust-basics`
are independent apps. They follow the same **lesson format** and **history
format** so they can later merge into one multi-language tutor without losing
any learning history. Keep this file identical in all four repos; change it in
all of them or none.

## 1. Lesson format

Each exercise is a folder named `lessons/NN-slug/` with:

| File | Purpose |
|------|---------|
| `lesson.md` | YAML front matter (below) + the sections below: learn, check, do |
| exercise file(s) | Starter code with `TODO`s for the learner to fill in |
| test file(s) | Tests that decide pass/fail; the app only runs these |

Front matter:

```yaml
id: node-03-battery-low        # <track>-<NN>-<slug>, globally unique
track: nodejs                  # nodejs | rust | (future languages)
title: Decide when a battery is low
topic: Conditions & booleans   # short label for the outline
minutes: 8                     # target time for this exercise
concepts: [conditions, equality, booleans]
prereqs: [node-02-convert-temperature]   # lesson ids
contrast: java                 # language the explanation compares against
edit: exercise.js              # the file the learner edits (Rust: src/lib.rs)
kind: core                     # core (default) | drill | stretch
for: node-03-battery-low       # drills and stretches only (not on a core lesson): the core lesson they practise
generated: true                # only on drills written by the tutor (section 6)
targets: One sentence on what the drill aims at   # generated drills only
```

**Kinds.** A *core* lesson moves the learner forward along the roadmap. Each
core lesson should have:
- one or more **drills** (`kind: drill`, ~10 min, a single concept or two,
  *new* code): used for reviews and as extra practice after a struggle;
- optionally a **stretch** (`kind: stretch`, harder, combines concepts):
  offered when the learner found the core lesson easy.

Name them after their core lesson so they sort next to it: `01a-drill-…`,
`01b-stretch-…`, then `02-…`. Section 5 says when each is offered.

**Lesson body: learn first, then do.** The app shows every exercise as up to
three numbered steps, *The idea* → *Quick check* → *Your turn*, built from
these `##` sections of `lesson.md` (in this order):

| Section | Shown as | Content |
|---------|----------|---------|
| `## Learn the idea` (`## Recall` for drills and stretches) | step 1 | 2–4 short paragraphs, a Java contrast, a ```visual (core lessons), and at least one ```run block |
| `## Quick check` | step 2 | one ```predict block (optional for drills) |
| `## Your turn` | step 3 | what to write, in one or two sentences |
| `## Example` | inside step 3 | calls and results: `name(args)  →  result` |
| `## Hints` | under a failed run, one at a time | a numbered list of 3, from a nudge to nearly the answer (required for core lessons) |
| `## Review` | after a pass | what to do when it comes back for review |

Front matter also takes `topic:` (a short label for the outline, e.g.
`Conditions & booleans`). One exercise teaches **one practical skill** in
about 8 minutes: one function (two at most), set in the running
device-and-service story, with test names written as plain sentences.

Two special code blocks:

````
```run
console.log("an example the learner can edit and run");
```

```predict
console.log(0 == "");
---
options: true | false
answer: true
why: `==` converts the empty string to 0 first.
```
````

**Visualizations.** A ```visual block (JSON) becomes an animated, step-through
scene (`app/public/visual.js`): scenario buttons grouped under *How it works*
and *What if it goes wrong*, the code with the current line highlighted, boxes
in labelled regions (stack/heap, call stack/queue, before/after…), arrows, a
*Step* explanation and a *Rule*, plus Back / Play / Next and ← → keys.

```
{ "title": "See it in action", "intro": "…",
  "regions": [ { "id": "stack", "label": "Stack · your variables" },
               { "id": "out", "label": "Console", "kind": "output" } ],
  "scenarios": [ {
    "name": "Move", "group": "How it works" | "What if it goes wrong",
    "code": "let a = …\nlet b = a;",
    "regions": [ … ],                     // optional, overrides the top level
    "check": { "output": "hi" }           // or { "error": "E0382" }; optional "prelude", "code"
    "steps": [ {
      "line": 2,                          // highlighted line (or a list), optional
      "say": "What happens, in plain words.",
      "rule": "The rule behind it.",      // optional
      "set": [ { "id": "a", "in": "stack", "title": "a", "value": "…", "tag": "owner",
                 "sub": "String", "state": "ok" } ],  // add, or update fields by id
      "drop": [ "b" ],                    // remove boxes
      "arrows": [ { "from": "a", "to": "hi", "label": "&", "state": "accent", "style": "solid" } ],
      "error": "E0382: borrow of moved value"         // shown in red for this step
    } ] } ] }
```

Box states: `normal`, `ok`, `active`, `info`, `bad`, `moved`, `faded`. A box
whose `in` changes **slides** to its new region; new boxes fade in; dropped
boxes fade out; arrows (replaced per step) draw themselves. Arrow states: `ok`,
`accent`, `muted`, `bad`.

Both apps' tests validate every visual (regions, arrows and lines refer to
things that exist) and run each scenario's `check` for real, so a visual can
never claim output or a compiler error that isn't true.

**Anatomy.** A ```anatomy block (JSON) takes one piece of code apart token by
token, for the bedrock lessons ("what does `fn` mean?"). The code is shown with
each part clickable; ← → or Back / Next walk through the parts, and the panel
shows what the part is, what it means, and the Java equivalent.

```
{ "title": "Anatomy of a Rust program", "intro": "…",
  "code": "fn main() {\n    println!(\"Hi\");\n}",
  "parts": [ { "text": "fn", "kind": "keyword",
               "say": "Starts a function definition.",
               "java": "No keyword: a method is recognised by its return type." },
             … ] }
```

Parts are found in the code in order; text between them (spaces, punctuation
you don't explain) stays plain. Kinds: `keyword`, `name`, `type`, `macro`,
`literal`, `symbol`, `operator` (each has its own colour). Both apps' tests
check that every part is found, in order, and has a `say`.

`run` blocks are editable and run with `POST /api/snippet` (Node: an ES
module; Rust: the body of `fn main` unless it defines one). In `predict`
blocks the learner picks an option before seeing the answer; the answer
must be exactly what the code prints (both apps' checks run every block).

`concepts` are the unit the tutor tracks. Name them in lowercase kebab-case
and reuse names across tracks when the idea is the same (e.g. `closures`,
`error-handling`, `async`). That reuse is how a future merged app notices
"you learned this in Node; here's the Rust version."

## 2. App API and front end

Both apps serve the same JSON API, and `app/public/` (the browser UI) is
**byte-for-byte identical** in both repos. Change it in both or neither.

| Method and path | Body | Returns |
|-----------------|------|---------|
| `GET /api/info` | | `{ track, title, accent, tutor: { enabled, reason } }` |
| `GET /api/lessons` | | `[{ slug, id, title, minutes, concepts, kind, for, stats }]` |
| `GET /api/lessons/:slug` | | `{ slug, meta, body, file, code, saved, stats }` |
| `PUT /api/lessons/:slug/code` | code as plain text | `{ saved: true }` |
| `DELETE /api/lessons/:slug/code` | | `{ code, stats, next }` (back to the starter) |
| `POST /api/lessons/:slug/run` | code as plain text | run result (below) plus `stats`, `next` |
| `POST /api/lessons/:slug/feedback` | `easy`, `right` or `hard` | `{ stats, next }` |
| `GET /api/profile` | | `{ profile, markdown, next }` (section 4) |
| `GET /api/history` | | events, newest first, without `lesson_open` and without code |
| `POST /api/snippet` | JSON `{ code }` | `{ output, ok, errors, timedOut }`: runs a ```run / ```predict block |
| `POST /api/lessons/:slug/log` | JSON `{ kind: "hint" \| "predict", payload }` | `{ ok }`: records a hint shown or a quick-check answer |
| `POST /api/lessons/:slug/hint` | JSON `{ code, question }` | `{ hint }` (Markdown; section 6) |
| `POST /api/lessons/:slug/visualize` | JSON `{ code }` | `{ visual, verified: { call, prints }, attempts }`: a checked step-through of the learner's failing code (section 6) |
| `POST /api/lessons/:slug/drill` | | `{ slug, id, title, targets, attempts, stats, next }`: a new verified drill (section 6) |

Tutor errors come back as `{ error }` with status 401 (key rejected), 429
(rate-limited), 502 (API error, refusal, or no drill passed its checks) or
503 (tutor not set up).

`next` is the adaptive next step (section 5) with the `slug` of its lesson
folder added, or `slug: null`.

Run result: `{ passed, failed, total, tests: [{ name, ok, message, expected?, actual? }],
compileErrors: [{ code, title, message }], timedOut, durationMs, output }`.
`compileErrors[].code` is a compiler error code such as `E0308`, or `null`.
`expected` / `actual` are display strings taken from a failed equality
assertion (Node's TAP output; Rust's `assert_eq!` left/right), shown to the
learner as "Expected … Got …".

The learner's code is saved to `data/work/<slug>/<edit file>` (gitignored).

## 3. History format

One append-only event log in [JSON Lines](https://jsonlines.org):
`data/events.jsonl`, one event per line, oldest first. It is plain text (you
can read it in any editor) and needs no database or dependencies. The app
skips any line that isn't valid JSON, such as a half-written last line after
a crash.

```json
{"ts":"2026-10-12T09:30:00.000Z","track":"rust","lesson_id":"rust-01-first-function","concept":null,"kind":"fail","duration_ms":352,"payload":{"passed":0,"failed":0,"total":0,"failing":[],"errors":["E0308"],"timed_out":false,"code":"..."}}
```

| Field | Meaning |
|-------|---------|
| `ts` | ISO 8601, UTC, with milliseconds |
| `track` | `nodejs`, `rust`, … |
| `lesson_id` | from lesson front matter, or `null` |
| `concept` | a single concept, or `null` |
| `kind` | see below |
| `duration_ms` | how long the run took, or `null` |
| `payload` | kind-specific object, or `null` |

Kinds recorded today:

| `kind` | When | `payload` |
|--------|------|-----------|
| `lesson_open` | a lesson is opened | `null` |
| `pass` | a run where every test passes | run payload |
| `fail` | any other run (failures, compile errors, timeout) | run payload |
| `reset` | code reset to the starter | `null` |
| `feedback` | the learner rates a solved exercise | `{ difficulty: "easy" \| "right" \| "hard" }` |

Run payload: `{ passed, failed, total, failing: [test names],
errors: [compiler error codes or titles], timed_out, code }`. `code` is
what was run, capped at 64 KB.

| `hint` | a hint was shown | `{ source: "lesson" \| "tutor" \| "visual", level?: number, question: string \| null }` |
| `predict` | a quick-check answer | `{ question: number, correct: boolean }` |
| `generated` | the tutor wrote a new drill (`lesson_id` is the new drill) | `{ for, targets, attempts }` |

Reviews aren't a separate kind: a passing run on a lesson that's due counts as a review. Sessions aren't recorded as events; a session is
derived as a run of events with no gap longer than 30 minutes.

Everything else (per-lesson stats, mastery, the review schedule, the learner
profile) is **derived** from events and can be rebuilt at any time. Merging
the two apps later means concatenating the two files and sorting by `ts`.

The API returns per-lesson `stats` derived from events:
`{ runs, first_pass_run, last_result, last_ts, best_passed, total }`, where
`total` is the largest test count seen (a run that doesn't compile reports 0
tests).

## 4. Learner profile

Derived from the events after every run and reset, and written into
`PROGRESS.md` between `<!-- profile:start -->` and `<!-- profile:end -->`
(everything outside that block is left alone). `PROGRESS.md` is committed,
so the tutor and any Claude session can read it; the raw events stay local.
`GET /api/profile` returns `{ profile, markdown }`.

Both apps must produce **exactly** the output in `docs/fixtures/`: every case
in `profile-cases.json` (one per adaptive path), and `profile-expected.md` for
the first case. Each app's tests check this. Change the rules in both apps and
regenerate the fixtures together.

**Review schedule (spaced repetition, per core lesson).** Intervals are 1,
3, 7, 16 and 35 days, as with language-learning flashcards.

- The first passing run solves the lesson: review step 0, due 1 day later.
- Once the lesson is solved, runs on its drills and stretches count as runs
  on the lesson for reviewing it. Drills and stretches have no schedule of
  their own.
- Runs before the due time don't count as reviews.
- Once due, a passing run moves up one step (capped at the last) and sets the
  next due date from that run. A failing run marks the review as lapsed; the
  next pass then restarts at step 0 (due 1 day later).

**Mastery (0–1, per concept).** Average over the lessons that list the
concept and have at least one run:

| Exercise state (any kind) | Score |
|--------------|-------|
| started, not solved | 0.2 |
| solved on run 1 / runs 2–3 / run 4 or later | 0.7 / 0.6 / 0.5 |
| plus each review step reached (core lessons) | +0.1 |
| review currently lapsed | −0.2 |

Clamped to 0–1 and rounded to 2 decimals; `null` if never practiced.

**Recurring mistakes.** Across failing runs, count each compiler error code
(or title) and each failing test name; keep those seen at least twice, most
frequent first (ties alphabetical), top 5.

**Sessions.** Events with gaps of at most 30 minutes form a session; total
minutes is the sum of session spans. The streak counts consecutive UTC days
with any event, ending today or yesterday.

**Pace.** The median of "solved on run" over solved core lessons, and how
many times each rating was given.

The lesson `stats` in the API also carry `review_step`, `due` and `due_now`
from the profile. The UI marks due lessons with ↻.

## 5. Adaptive next step

The app always recommends one next step, chosen from the history in this
order. The UI opens it on start and shows it after every run and rating;
`PROGRESS.md` records it.

1. **Review.** If core lessons are due, take the one due longest. Practise it
   with one of its drills (never-run first, then the one run longest ago),
   so the review is fresh code rather than a replay. With no drill, review
   the lesson itself (Reset, then solve from memory).
2. **React to the last solved core lesson**, unless the learner has already
   started a later core lesson:
   - it was **hard** (rated "too hard", or solved on run 4 or later without
     a "too easy" rating) → its first unsolved drill, as extra practice;
   - it was **easy** (rated "too easy") → its first unsolved stretch.
   A first-try solve without a rating just moves on: with under 3 hours a
   week, extra exercises are offered only when the learner signals for them.
3. **Current lesson.** The first unsolved core lesson. After 3 or more runs
   without solving it, offer its first unsolved drill as a warm-up instead.
4. **Done.** All core lessons solved: say when the next review is due.

`next` is `{ kind, lesson_id, title, reason }`, where `kind` is `review`,
`drill`, `stretch`, `lesson` or `done` (then `lesson_id` and `title` are
`null`). `reason` is a short human-readable explanation shown in the UI.

The rules choose among exercises that exist. The tutor (section 6) adds new
ones aimed at the learner's mistakes; once saved, a generated drill is an
ordinary drill to these rules (reviews, extra practice, warm-ups).

## 6. Tutor (Claude)

Optional: on when `ANTHROPIC_API_KEY` is set (in `.env`, never committed).
Everything else works without it. Model `claude-opus-5-5`, with
`fallbacks: "default"` (beta `server-side-fallback-2026-07-01`) so a declined
request is retried on Anthropic's recommended fallback model. Node uses the
official SDK (`@anthropic-ai/sdk`); Rust has no official SDK and calls the
Messages API over raw HTTP through the system `curl`, passing the key in a
private temporary config file, never on the command line.
Values in `.env` win over the shell's (an old exported `ANTHROPIC_API_KEY`
must not shadow it), and only the API key is sent. A rejected key (401) is
explained without revealing it: where it came from (`.env` or the shell), its
first 12 and last 4 characters, the API's message, and the usual mix-ups (a
Claude subscription token instead of an API key, a non-default
`ANTHROPIC_BASE_URL`).

The prompts are shared files in `docs/tutor/` (identical in both repos):
`system.md` (persona; includes the learner's own "## Learner" section of
`PROGRESS.md` and the generated profile), `hint.md`, `visual.md`,
`visual-schema.json`, `drill.md`, `conventions-<track>.md` and
`drill-schema.json`.

**Hint** (effort `medium`). The app runs the tests on the learner's current
code first, then asks for the smallest hint that unblocks them, under 120
words, never the solution; compiler error codes are explained first.

**Show me what my code does** (effort `medium`, structured output with
`visual-schema.json`). After a failing run, the app runs the tests, then asks
for a step-through visual (section 1 format) of the learner's own code on the
first failing test, ending where it goes wrong, without the fix. Every field
in the schema is required, so "" and line 0 mean "not set"; the app turns the
answer into a normal visual with one scenario in the group *Your code*.
Before the learner sees it, the app checks it:

1. the visualizer can play it (the same checks as lesson visuals: regions,
   boxes, arrows and lines all refer to things that exist);
2. its claim is true on the real code: `check.call` is run after the
   learner's code (Node: `console.log(<call>)`; Rust: `println!("{:?}", <call>)`)
   and must print `check.prints`; if the code doesn't load or compile,
   `call` is "" and the real error must contain `check.prints`.

A visual that fails a check is sent back once with the reason (for a wrong
claim, with what really printed); a second failure is reported to the
learner. A shown visual is logged as a `hint` with `source: "visual"`.

**New drill** (effort `high`, structured output with `drill-schema.json`).
Aimed at the exercise's core lesson, using what the profile says: runs and
rating on that lesson, the mastery of its concepts, recurring mistakes and the
latest failing runs. Existing exercises for the lesson are included so the
drill is new code. Before anything is saved, the app checks the drill:

1. the starter code loads/compiles and runs at least 2 tests;
2. at least one test fails against the starter (the tests check something);
3. the reference solution passes every test.

A drill that fails a check is sent back once with the reason; a second
failure is reported to the learner and nothing is saved. A drill that passes
is saved as `lessons/<NN>g-<slug>/` (`kind: drill`, `for:` the core lesson,
`generated: true`, concepts limited to the lesson's), and its solution as
`data/solutions/<slug>/` (local only; never shown to the learner).

## 7. Session shape (under 3 h/week, short daily sessions)

A ~20 minute session:

1. 3 min: one or two review items that are due
2. 15 min: the current lesson's exercise, run → feedback → fix
3. 2 min: one-line reflection ("what tripped me up"), saved as an event
