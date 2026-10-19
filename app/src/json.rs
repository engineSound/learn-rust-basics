//! A tiny JSON writer, enough for the API responses.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// Field of an object, or None.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> &[Json] {
        match self {
            Json::Arr(items) => items,
            _ => &[],
        }
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) if n.is_finite() => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{n}"));
                }
            }
            Json::Num(_) => out.push_str("null"),
            Json::Str(s) => write_str(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse a JSON document.
pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser { chars: text.chars().collect(), pos: 0 };
    let value = p.value()?;
    p.skip_ws();
    if p.pos != p.chars.len() {
        return Err(format!("unexpected trailing text at {}", p.pos));
    }
    Ok(value)
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Result<char, String> {
        let c = self.peek().ok_or("unexpected end of input")?;
        self.pos += 1;
        Ok(c)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    fn expect_word(&mut self, word: &str, value: Json) -> Result<Json, String> {
        for expected in word.chars() {
            if self.next()? != expected {
                return Err(format!("expected {word}"));
            }
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek().ok_or("unexpected end of input")? {
            'n' => self.expect_word("null", Json::Null),
            't' => self.expect_word("true", Json::Bool(true)),
            'f' => self.expect_word("false", Json::Bool(false)),
            '"' => self.string().map(Json::Str),
            '[' => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == Some(']') {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.skip_ws();
                    match self.next()? {
                        ',' => continue,
                        ']' => return Ok(Json::Arr(items)),
                        c => return Err(format!("expected , or ] but found {c:?}")),
                    }
                }
            }
            '{' => {
                self.pos += 1;
                let mut fields = Vec::new();
                self.skip_ws();
                if self.peek() == Some('}') {
                    self.pos += 1;
                    return Ok(Json::Obj(fields));
                }
                loop {
                    self.skip_ws();
                    let key = self.string()?;
                    self.skip_ws();
                    if self.next()? != ':' {
                        return Err("expected :".into());
                    }
                    fields.push((key, self.value()?));
                    self.skip_ws();
                    match self.next()? {
                        ',' => continue,
                        '}' => return Ok(Json::Obj(fields)),
                        c => return Err(format!("expected , or }} but found {c:?}")),
                    }
                }
            }
            _ => self.number(),
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while matches!(self.peek(), Some('-' | '+' | '.' | 'e' | 'E' | '0'..='9')) {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>().map(Json::Num).map_err(|_| format!("bad number {text:?}"))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.next()? != '"' {
            return Err("expected a string".into());
        }
        let mut out = String::new();
        loop {
            match self.next()? {
                '"' => return Ok(out),
                '\\' => match self.next()? {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let mut code = self.hex4()?;
                        if (0xD800..0xDC00).contains(&code) && self.peek() == Some('\\') {
                            self.pos += 1;
                            if self.next()? != 'u' {
                                return Err("bad surrogate pair".into());
                            }
                            let low = self.hex4()?;
                            code = 0x10000 + ((code - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
                        }
                        out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                    }
                    c => return Err(format!("bad escape \\{c}")),
                },
                c => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut code = 0;
        for _ in 0..4 {
            code = code * 16 + self.next()?.to_digit(16).ok_or("bad \\u escape")?;
        }
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_nested_values_and_escapes() {
        let j = Json::obj(vec![
            ("a", Json::Num(3.0)),
            ("b", Json::Arr(vec![Json::Bool(true), Json::Null])),
            ("c", Json::str("say \"hi\"\n")),
            ("d", Json::Num(0.5)),
        ]);
        assert_eq!(j.to_string(), r#"{"a":3,"b":[true,null],"c":"say \"hi\"\n","d":0.5}"#);
    }

    #[test]
    fn parse_round_trips_what_we_write() {
        let original = Json::obj(vec![
            ("ts", Json::str("2026-10-12T10:00:00.000Z")),
            ("n", Json::Num(-2.5)),
            ("list", Json::Arr(vec![Json::Null, Json::Bool(false), Json::str("tab\there \"q\" é")])),
            ("empty", Json::Obj(vec![])),
        ]);
        assert_eq!(parse(&original.to_string()), Ok(original));
    }

    #[test]
    fn parse_handles_unicode_escapes_and_rejects_garbage() {
        assert_eq!(parse(r#"" \u00e9 \ud83d\ude00 ""#), Ok(Json::str(" é 😀 ")));
        assert_eq!(parse(" [1, 2e3] ").unwrap().as_array().len(), 2);
        assert!(parse("{\"ts\":\"broken").is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("1 2").is_err());
    }
}
