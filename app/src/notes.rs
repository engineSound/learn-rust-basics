//! The learner's own notes, one Markdown file per lesson: notes/<slug>.md (gitignored).
//! Format (docs/SHARED-FORMAT.md, section 8; shared with the other learn-* repos):
//!
//! ```text
//! # <lesson title>
//!
//! <!-- note {"anchor":"token:println!","at":"2026-10-20T09:05:00.000Z"} -->
//! ## `println!`
//!
//! free text…
//! ```

use crate::json::{self, Json};
use std::fs;
use std::path::Path;

pub const MAX_NOTE: usize = 20_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub anchor: String,
    pub label: String,
    pub text: String,
    pub at: String,
}

impl Note {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("anchor", Json::str(self.anchor.clone())),
            ("label", Json::str(self.label.clone())),
            ("text", Json::str(self.text.clone())),
            ("at", Json::str(self.at.clone())),
        ])
    }
}

/// Markdown file → (title, notes). Unreadable markers are skipped.
pub fn parse(text: &str) -> (String, Vec<Note>) {
    let text = text.replace("\r\n", "\n");
    let title = text.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or("").trim().to_string();
    let mut notes = Vec::new();
    let mut cur: Option<(Note, Vec<&str>)> = None;
    let flush = |cur: &mut Option<(Note, Vec<&str>)>, notes: &mut Vec<Note>| {
        if let Some((mut n, lines)) = cur.take() {
            n.text = lines.join("\n").trim().to_string();
            notes.push(n);
        }
    };
    for line in text.split('\n') {
        let marker = line.strip_prefix("<!-- note ").and_then(|r| r.strip_suffix(" -->")).filter(|m| m.starts_with('{'));
        if let Some(meta) = marker {
            flush(&mut cur, &mut notes);
            let meta = json::parse(meta).unwrap_or(Json::Null);
            let field = |k: &str| meta.get(k).and_then(Json::as_str).unwrap_or("").to_string();
            let anchor = field("anchor");
            if !anchor.is_empty() {
                cur = Some((Note { anchor, label: String::new(), text: String::new(), at: field("at") }, Vec::new()));
            }
        } else if let Some((n, lines)) = cur.as_mut() {
            match line.strip_prefix("## ") {
                Some(label) if n.label.is_empty() && lines.iter().all(|l| l.is_empty()) => n.label = label.trim().to_string(),
                _ => lines.push(line),
            }
        }
    }
    flush(&mut cur, &mut notes);
    (title, notes)
}

/// (title, notes) → Markdown, the exact inverse of `parse` for well-formed notes.
pub fn format(title: &str, notes: &[Note]) -> String {
    let mut parts = vec![format!("# {title}")];
    for n in notes {
        let meta = Json::obj(vec![("anchor", Json::str(n.anchor.clone())), ("at", Json::str(n.at.clone()))]);
        parts.push(format!("<!-- note {} -->\n## {}\n\n{}", meta.to_string(), n.label, n.text));
    }
    parts.join("\n\n") + "\n"
}

pub fn read(dir: &Path, slug: &str) -> (String, Vec<Note>) {
    fs::read_to_string(dir.join(format!("{slug}.md"))).map(|t| parse(&t)).unwrap_or_default()
}

/// Create, replace or (with empty text) remove one note; returns the lesson's notes.
pub fn save(dir: &Path, slug: &str, title: &str, anchor: &str, label: &str, text: &str, at: &str) -> Result<Vec<Note>, String> {
    if anchor.is_empty() || anchor.len() > 300 || anchor.contains("-->") || anchor.contains('\n') {
        return Err("A note needs a short anchor".into());
    }
    let body = text.trim();
    if body.len() > MAX_NOTE {
        return Err("That note is too long".into());
    }
    let (_, mut notes) = read(dir, slug);
    notes.retain(|n| n.anchor != anchor || !body.is_empty());
    let label = if label.trim().is_empty() { anchor } else { label };
    let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if !body.is_empty() {
        match notes.iter_mut().find(|n| n.anchor == anchor) {
            Some(n) => {
                n.label = label;
                n.text = body.to_string();
                n.at = at.to_string();
            }
            None => notes.push(Note { anchor: anchor.into(), label, text: body.into(), at: at.into() }),
        }
    }
    let file = dir.join(format!("{slug}.md"));
    if notes.is_empty() {
        fs::remove_file(&file).ok();
    } else {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        fs::write(&file, format(title, &notes)).map_err(|e| e.to_string())?;
    }
    Ok(notes)
}

/// Every lesson's notes: [{ slug, title, notes }].
pub fn list(dir: &Path) -> Json {
    let mut slugs: Vec<String> = fs::read_dir(dir)
        .map(|it| it.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".md")).map(String::from)).collect())
        .unwrap_or_default();
    slugs.sort();
    Json::Arr(
        slugs
            .into_iter()
            .filter_map(|slug| {
                let (title, notes) = read(dir, &slug);
                (!notes.is_empty()).then(|| {
                    Json::obj(vec![("slug", Json::str(slug)), ("title", Json::str(title)), ("notes", Json::Arr(notes.iter().map(Note::to_json).collect()))])
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_example_parses_and_writes_back_byte_for_byte() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/fixtures");
        let text = fs::read_to_string(fixtures.join("notes-example.md")).unwrap();
        let want = json::parse(&fs::read_to_string(fixtures.join("notes-example.json")).unwrap()).unwrap();
        let (title, notes) = parse(&text);
        assert_eq!(Some(&Json::str(title.clone())), want.get("title"));
        let got = Json::Arr(notes.iter().map(Note::to_json).collect());
        assert_eq!(got.to_string(), want.get("notes").unwrap().to_string());
        assert_eq!(format(&title, &notes), text);
    }

    #[test]
    fn save_replaces_in_place_and_empty_text_deletes() {
        let dir = std::env::temp_dir().join(format!("learn-notes-test-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        save(&dir, "01-x", "Lesson X", "idea", "The idea", "first", "T1").unwrap();
        save(&dir, "01-x", "Lesson X", "token:println!", "`println!`", "a macro\n\nline two", "T2").unwrap();
        let notes = save(&dir, "01-x", "Lesson X", "idea", "The idea", "  changed  ", "T3").unwrap();
        assert_eq!(notes.iter().map(|n| (n.anchor.as_str(), n.text.as_str(), n.at.as_str())).collect::<Vec<_>>(), vec![("idea", "changed", "T3"), ("token:println!", "a macro\n\nline two", "T2")]);
        assert_eq!(read(&dir, "01-x").1, notes);
        assert!(list(&dir).to_string().contains("\"slug\":\"01-x\",\"title\":\"Lesson X\""));
        save(&dir, "01-x", "Lesson X", "idea", "", "", "T4").unwrap();
        save(&dir, "01-x", "Lesson X", "token:println!", "", " ", "T5").unwrap();
        assert!(!dir.join("01-x.md").exists(), "no notes left: no file");
        assert!(save(&dir, "01-x", "X", "bad-->anchor", "", "x", "T6").is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
