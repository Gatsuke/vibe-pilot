//! Minimal JSON: value, parser, serializer. Pure std, no dependencies.
//!
//! Covers the full JSON grammar (escapes, unicode surrogate pairs, exponents)
//! with a depth guard. Used for `vp report --json` and the MCP stdio server.

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
    pub fn parse(src: &str) -> Result<Json, String> {
        let mut p = Parser {
            c: src.chars().collect(),
            i: 0,
            depth: 0,
        };
        p.ws();
        let v = p.value()?;
        p.ws();
        if p.i != p.c.len() {
            return Err(format!("trailing characters at offset {}", p.i));
        }
        Ok(v)
    }

    pub fn dump(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.is_finite() {
                    if n.fract() == 0.0 && n.abs() < 9.0e15 {
                        out.push_str(&format!("{}", *n as i64));
                    } else {
                        out.push_str(&format!("{}", n));
                    }
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => write_str(s, out),
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
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

    // ---- accessors ----

    pub fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Obj(pairs) = self {
            pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Json::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        if let Json::Num(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Json::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_arr(&self) -> Option<&Vec<Json>> {
        if let Json::Arr(a) = self {
            Some(a)
        } else {
            None
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    // ---- constructors ----

    pub fn s(v: &str) -> Json {
        Json::Str(v.to_string())
    }

    pub fn n(v: f64) -> Json {
        Json::Num(v)
    }

    pub fn b(v: bool) -> Json {
        Json::Bool(v)
    }

    pub fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn arr(v: Vec<Json>) -> Json {
        Json::Arr(v)
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
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser {
    c: Vec<char>,
    i: usize,
    depth: u32,
}

impl Parser {
    fn ws(&mut self) {
        while self.i < self.c.len() && self.c[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    fn value(&mut self) -> Result<Json, String> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("nesting too deep".into());
        }
        let r = match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('t') => self.lit("true", Json::Bool(true)),
            Some('f') => self.lit("false", Json::Bool(false)),
            Some('n') => self.lit("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(format!("unexpected character '{}'", c)),
            None => Err("unexpected end of input".into()),
        };
        self.depth -= 1;
        r
    }

    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        for ch in word.chars() {
            if self.peek() != Some(ch) {
                return Err(format!("expected literal '{}'", word));
            }
            self.i += 1;
        }
        Ok(v)
    }

    fn object(&mut self) -> Result<Json, String> {
        self.i += 1; // consume '{'
        let mut pairs: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.ws();
            if self.peek() != Some('"') {
                return Err("expected object key".into());
            }
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(':') {
                return Err("expected ':' after key".into());
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            pairs.push((k, v));
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some('}') => {
                    self.i += 1;
                    return Ok(Json::Obj(pairs));
                }
                _ => return Err("expected ',' or '}'".into()),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.i += 1; // consume '['
        let mut items: Vec<Json> = Vec::new();
        self.ws();
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some(']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err("expected ',' or ']'".into()),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // consume opening quote
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            self.i += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.peek().ok_or("dangling escape")?;
                    self.i += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                // expect a low surrogate
                                if self.peek() != Some('\\') {
                                    return Err("lone high surrogate".into());
                                }
                                self.i += 1;
                                if self.peek() != Some('u') {
                                    return Err("bad surrogate pair".into());
                                }
                                self.i += 1;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err("bad low surrogate".into());
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(cp).ok_or("invalid codepoint")?);
                        }
                        _ => return Err(format!("bad escape '\\{}'", e)),
                    }
                }
                c if (c as u32) < 0x20 => {
                    return Err("unescaped control character in string".into())
                }
                c => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self.peek().ok_or("truncated \\u escape")?;
            let d = c.to_digit(16).ok_or_else(|| format!("bad hex '{}'", c))?;
            v = v.wrapping_mul(16).wrapping_add(d);
            self.i += 1;
        }
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some('-') {
            self.i += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        if self.peek() == Some('.') {
            self.i += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.i += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.i += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let s: String = self.c[start..self.i].iter().collect();
        s.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("invalid number '{}'", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_basic() {
        let src = r#"{"a":1,"b":[true,false,null],"c":"x\ny","d":{"e":-2.5e2}}"#;
        let v = Json::parse(src).unwrap();
        assert_eq!(v.get("a").unwrap().as_num(), Some(1.0));
        assert_eq!(v.get("c").unwrap().as_str(), Some("x\ny"));
        let dump = v.dump();
        let v2 = Json::parse(&dump).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn unicode_escapes() {
        let v = Json::parse(r#""A é 😀""#).unwrap();
        assert_eq!(v.as_str(), Some("A é 😀"));
        let v2 = Json::parse("\"\\u0041 \\u00e9 \\ud83d\\ude00\"").unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Json::parse("{").is_err());
        assert!(Json::parse("[1,]").is_err());
        assert!(Json::parse("nul").is_err());
        assert!(Json::parse("\"\\u12\"").is_err());
        assert!(Json::parse("{}x").is_err());
    }

    #[test]
    fn escapes_on_dump() {
        let v = Json::s("tab\there \"q\" \\slash");
        assert_eq!(v.dump(), "\"tab\\there \\\"q\\\" \\\\slash\"");
    }
}
