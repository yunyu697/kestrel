//! Inline calculator: arithmetic, functions, and unit conversion.

#[derive(Debug, Clone)]
pub struct CalcResult {
    pub display: String,
    pub value: f64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
}

fn tokenize(input: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let save = i;
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                if i < chars.len() && chars[i].is_ascii_digit() {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                } else {
                    i = save;
                }
            }
            let s: String = chars[start..i].iter().filter(|c| **c != '_').collect();
            out.push(Tok::Num(s.parse().ok()?));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect::<String>().to_lowercase()));
            continue;
        }
        match c {
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            ',' => out.push(Tok::Comma),
            '+' | '-' | '*' | '/' | '%' | '^' => out.push(Tok::Op(c)),
            '×' => out.push(Tok::Op('*')),
            '÷' => out.push(Tok::Op('/')),
            '（' => out.push(Tok::LParen),
            '）' => out.push(Tok::RParen),
            '＋' => out.push(Tok::Op('+')),
            '－' => out.push(Tok::Op('-')),
            '＊' => out.push(Tok::Op('*')),
            '／' => out.push(Tok::Op('/')),
            _ => return None,
        }
        i += 1;
    }
    Some(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat_op(&mut self, ops: &[char]) -> Option<char> {
        if let Some(Tok::Op(c)) = self.peek() {
            if ops.contains(c) {
                let c = *c;
                self.pos += 1;
                return Some(c);
            }
        }
        None
    }

    fn expr(&mut self) -> Option<f64> {
        let mut lhs = self.term()?;
        loop {
            match self.eat_op(&['+', '-']) {
                Some('+') => lhs += self.term()?,
                Some('-') => lhs -= self.term()?,
                _ => break,
            }
        }
        Some(lhs)
    }

    fn term(&mut self) -> Option<f64> {
        let mut lhs = self.power()?;
        loop {
            match self.eat_op(&['*', '/', '%']) {
                Some('*') => lhs *= self.power()?,
                Some('/') => {
                    let rhs = self.power()?;
                    if rhs == 0.0 {
                        return None;
                    }
                    lhs /= rhs;
                }
                Some('%') => {
                    let rhs = self.power()?;
                    if rhs == 0.0 {
                        return None;
                    }
                    lhs %= rhs;
                }
                _ => break,
            }
        }
        Some(lhs)
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.eat_op(&['^']).is_some() {
            let exp = self.power()?; // right associative
            return Some(base.powf(exp));
        }
        Some(base)
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                Some(-self.unary()?)
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Option<f64> {
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Some(n)
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let v = self.expr()?;
                if matches!(self.peek(), Some(Tok::RParen)) {
                    self.pos += 1;
                    Some(v)
                } else {
                    None
                }
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                match name.as_str() {
                    "pi" | "π" => return Some(std::f64::consts::PI),
                    "e" => return Some(std::f64::consts::E),
                    "tau" => return Some(std::f64::consts::TAU),
                    _ => {}
                }
                if matches!(self.peek(), Some(Tok::LParen)) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        loop {
                            args.push(self.expr()?);
                            if matches!(self.peek(), Some(Tok::Comma)) {
                                self.pos += 1;
                                continue;
                            }
                            break;
                        }
                    }
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        return None;
                    }
                    self.pos += 1;
                    return apply_fn(&name, &args);
                }
                None
            }
            _ => None,
        }
    }
}

fn apply_fn(name: &str, a: &[f64]) -> Option<f64> {
    let one = |f: fn(f64) -> f64| a.first().map(|v| f(*v));
    match name {
        "sqrt" => one(f64::sqrt),
        "cbrt" => one(f64::cbrt),
        "abs" => one(f64::abs),
        "sin" => one(f64::sin),
        "cos" => one(f64::cos),
        "tan" => one(f64::tan),
        "asin" => one(f64::asin),
        "acos" => one(f64::acos),
        "atan" => one(f64::atan),
        "ln" => one(f64::ln),
        "log" | "log10" => one(f64::log10),
        "log2" => one(f64::log2),
        "exp" => one(f64::exp),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "round" => one(f64::round),
        "sign" => one(f64::signum),
        "deg" => one(f64::to_degrees),
        "rad" => one(f64::to_radians),
        "min" => a.iter().cloned().reduce(f64::min),
        "max" => a.iter().cloned().reduce(f64::max),
        "pow" => match (a.first(), a.get(1)) {
            (Some(b), Some(e)) => Some(b.powf(*e)),
            _ => None,
        },
        "logx" => match (a.first(), a.get(1)) {
            (Some(x), Some(b)) => Some(x.log(*b)),
            _ => None,
        },
        "hypot" => match (a.first(), a.get(1)) {
            (Some(x), Some(y)) => Some(x.hypot(*y)),
            _ => None,
        },
        _ => None,
    }
}

pub fn format_number(v: f64) -> String {
    if !v.is_finite() {
        return if v.is_nan() { "NaN".into() } else { "∞".into() };
    }
    let abs = v.abs();
    if abs != 0.0 && (abs >= 1e15 || abs < 1e-6) {
        return format!("{v:.6e}");
    }
    let rounded = (v * 1e10).round() / 1e10;
    let mut s = format!("{rounded}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    // thousands separators for the integer part
    let neg = s.starts_with('-');
    let body = if neg { &s[1..] } else { &s[..] };
    let (int_part, frac) = match body.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (body, None),
    };
    let mut grouped = String::new();
    let digits: Vec<char> = int_part.chars().collect();
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(*c);
    }
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push_str(&grouped);
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    out
}

// ---------------- units ----------------

#[derive(Clone, Copy, PartialEq)]
enum Dim {
    Length,
    Mass,
    Temp,
    Data,
    Time,
    Area,
    Volume,
    Speed,
    Energy,
}

fn unit_table(u: &str) -> Option<(Dim, f64, f64)> {
    // (dimension, factor to base unit, offset for temperature scales)
    let t = match u {
        // length (base: meter)
        "mm" | "毫米" => (Dim::Length, 0.001, 0.0),
        "cm" | "厘米" => (Dim::Length, 0.01, 0.0),
        "m" | "米" => (Dim::Length, 1.0, 0.0),
        "km" | "千米" | "公里" => (Dim::Length, 1000.0, 0.0),
        "in" | "inch" | "英寸" => (Dim::Length, 0.0254, 0.0),
        "ft" | "foot" | "feet" | "英尺" => (Dim::Length, 0.3048, 0.0),
        "yd" | "yard" | "码" => (Dim::Length, 0.9144, 0.0),
        "mi" | "mile" | "英里" => (Dim::Length, 1609.344, 0.0),
        "nmi" | "海里" => (Dim::Length, 1852.0, 0.0),
        // mass (base: kilogram)
        "mg" | "毫克" => (Dim::Mass, 1e-6, 0.0),
        "g" | "克" => (Dim::Mass, 0.001, 0.0),
        "kg" | "千克" | "公斤" => (Dim::Mass, 1.0, 0.0),
        "t" | "吨" => (Dim::Mass, 1000.0, 0.0),
        "oz" | "盎司" => (Dim::Mass, 0.028_349_523, 0.0),
        "lb" | "lbs" | "磅" => (Dim::Mass, 0.453_592_37, 0.0),
        // temperature (base: celsius)
        "c" | "℃" | "摄氏度" => (Dim::Temp, 1.0, 0.0),
        "f" | "℉" | "华氏度" => (Dim::Temp, 5.0 / 9.0, -32.0 * 5.0 / 9.0),
        "k" | "开尔文" => (Dim::Temp, 1.0, -273.15),
        // data (base: byte)
        "b" | "byte" | "字节" => (Dim::Data, 1.0, 0.0),
        "kb" => (Dim::Data, 1024.0, 0.0),
        "mb" => (Dim::Data, 1024.0 * 1024.0, 0.0),
        "gb" => (Dim::Data, 1024.0f64.powi(3), 0.0),
        "tb" => (Dim::Data, 1024.0f64.powi(4), 0.0),
        "kib" => (Dim::Data, 1024.0, 0.0),
        "mib" => (Dim::Data, 1024.0 * 1024.0, 0.0),
        "gib" => (Dim::Data, 1024.0f64.powi(3), 0.0),
        // time (base: second)
        "ms" | "毫秒" => (Dim::Time, 0.001, 0.0),
        "s" | "sec" | "秒" => (Dim::Time, 1.0, 0.0),
        "min" | "分钟" => (Dim::Time, 60.0, 0.0),
        "h" | "hr" | "hour" | "小时" => (Dim::Time, 3600.0, 0.0),
        "d" | "day" | "天" => (Dim::Time, 86400.0, 0.0),
        "w" | "week" | "周" => (Dim::Time, 604800.0, 0.0),
        // area (base: m²)
        "m2" | "平方米" => (Dim::Area, 1.0, 0.0),
        "km2" | "平方公里" => (Dim::Area, 1e6, 0.0),
        "ha" | "公顷" => (Dim::Area, 10000.0, 0.0),
        "acre" | "英亩" => (Dim::Area, 4046.856_422_4, 0.0),
        "ft2" | "平方英尺" => (Dim::Area, 0.092_903_04, 0.0),
        // volume (base: liter)
        "ml" | "毫升" => (Dim::Volume, 0.001, 0.0),
        "l" | "升" => (Dim::Volume, 1.0, 0.0),
        "gal" | "加仑" => (Dim::Volume, 3.785_411_784, 0.0),
        "cup" | "杯" => (Dim::Volume, 0.236_588_236, 0.0),
        // speed (base: m/s)
        "kmh" | "kph" | "km/h" => (Dim::Speed, 1.0 / 3.6, 0.0),
        "mph" => (Dim::Speed, 0.447_04, 0.0),
        "mps" | "m/s" => (Dim::Speed, 1.0, 0.0),
        "kn" | "节" => (Dim::Speed, 0.514_444, 0.0),
        // energy (base: joule)
        "j" | "焦" => (Dim::Energy, 1.0, 0.0),
        "cal" | "卡" => (Dim::Energy, 4.184, 0.0),
        "kcal" | "千卡" | "大卡" => (Dim::Energy, 4184.0, 0.0),
        "kwh" | "度" => (Dim::Energy, 3.6e6, 0.0),
        _ => return None,
    };
    Some(t)
}

fn unit_name(u: &str) -> String {
    match u {
        "mm" => "毫米".into(),
        "cm" => "厘米".into(),
        "m" => "米".into(),
        "km" => "千米".into(),
        "in" => "英寸".into(),
        "ft" => "英尺".into(),
        "mi" => "英里".into(),
        "kg" => "千克".into(),
        "g" => "克".into(),
        "lb" => "磅".into(),
        "c" => "摄氏度".into(),
        "f" => "华氏度".into(),
        "k" => "开尔文".into(),
        "kb" | "kib" => "KB".into(),
        "mb" | "mib" => "MB".into(),
        "gb" | "gib" => "GB".into(),
        "tb" => "TB".into(),
        "s" => "秒".into(),
        "min" => "分钟".into(),
        "h" => "小时".into(),
        "d" => "天".into(),
        "kmh" => "km/h".into(),
        "mph" => "mph".into(),
        "l" => "升".into(),
        "ml" => "毫升".into(),
        "kcal" => "千卡".into(),
        other => other.to_string(),
    }
}

fn convert_units(input: &str) -> Option<CalcResult> {
    let lower = input.to_lowercase();
    let (num_part, rest) = {
        let idx = lower
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == ' '))?;
        (&lower[..idx], lower[idx..].trim())
    };
    let value: f64 = num_part.trim().parse().ok()?;

    // split "unit to unit"
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let from = parts[0].trim_end_matches('的');
    let sep = parts[1];
    if !matches!(sep, "to" | "in" | "as" | "->" | ">" | "转" | "等于" | "=") {
        return None;
    }
    let to = parts[2];
    let (fd, ff, fo) = unit_table(from)?;
    let (td, tf, toff) = unit_table(to)?;
    if fd != td {
        return None;
    }
    let base = value * ff + fo;
    let out = (base - toff) / tf;
    Some(CalcResult {
        display: format!("{} {}", format_number(out), unit_name(to)),
        value: out,
        detail: format!(
            "{} {} = {} {}",
            format_number(value),
            unit_name(from),
            format_number(out),
            unit_name(to)
        ),
    })
}

/// Evaluate any supported input. Returns `None` when the text is not a calculation.
pub fn evaluate(input: &str) -> Option<CalcResult> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return None;
    }
    if let Some(r) = convert_units(trimmed) {
        return Some(r);
    }

    // "15% of 200"
    let lower = trimmed.to_lowercase();
    if let Some(idx) = lower.find("% of ") {
        let p: f64 = lower[..idx].trim().parse().ok()?;
        let base: f64 = lower[idx + 5..].trim().parse().ok()?;
        let v = p / 100.0 * base;
        return Some(CalcResult {
            display: format_number(v),
            value: v,
            detail: format!("{p}% × {base}"),
        });
    }

    // Only attempt when it actually looks like math.
    let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
    let has_op = trimmed
        .chars()
        .any(|c| matches!(c, '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')'));
    let has_fn = trimmed.contains('(');
    if !has_digit || !(has_op || has_fn) {
        return None;
    }

    let toks = tokenize(trimmed)?;
    let mut p = Parser { toks, pos: 0 };
    let v = p.expr()?;
    if p.pos != p.toks.len() {
        return None;
    }
    if !v.is_finite() {
        return None;
    }
    Some(CalcResult {
        display: format_number(v),
        value: v,
        detail: trimmed.to_string(),
    })
}
