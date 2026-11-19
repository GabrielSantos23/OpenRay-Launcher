use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn function(name: &str) -> Option<fn(f64) -> f64> {
    Some(match name {
        "sqrt" => f64::sqrt,
        "cbrt" => f64::cbrt,
        "abs" => f64::abs,
        "round" => js_round,
        "floor" => f64::floor,
        "ceil" => f64::ceil,
        "exp" => f64::exp,
        "ln" => f64::ln,
        "log" => f64::log10,
        "log2" => f64::log2,
        "sin" => f64::sin,
        "cos" => f64::cos,
        "tan" => f64::tan,
        "asin" => f64::asin,
        "acos" => f64::acos,
        "atan" => f64::atan,
        _ => return None,
    })
}

fn constant(name: &str) -> Option<f64> {
    match name {
        "pi" | "π" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        "tau" => Some(std::f64::consts::TAU),
        _ => None,
    }
}

#[derive(Clone, PartialEq, Debug)]
enum Token {
    Num(f64),
    Op(char),
    Id(String),
}

static SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?|\.\d+)").unwrap());
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-zπ][a-z0-9]*").unwrap());

fn tokenize(input: &str) -> Option<Vec<Token>> {
    let source = input.replace('×', "*").replace('÷', "/").replace("**", "^");
    let mut tokens = Vec::new();
    let mut rest = source.as_str();
    while let Some(c) = rest.chars().next() {
        if let Some(m) = SPACE.find(rest) {
            rest = &rest[m.end()..];
        } else if let Some(m) = NUMBER.find(rest) {
            tokens.push(Token::Num(m.as_str().replace(',', "").parse().ok()?));
            rest = &rest[m.end()..];
        } else if let Some(m) = WORD.find(rest) {
            tokens.push(Token::Id(m.as_str().to_lowercase()));
            rest = &rest[m.end()..];
        } else if "+-*/^%()!".contains(c) {
            tokens.push(Token::Op(c));
            rest = &rest[c.len_utf8()..];
        } else {
            return None;
        }
    }
    Some(tokens)
}

fn factorial(n: f64) -> f64 {
    if n.fract() != 0.0 || !(0.0..=170.0).contains(&n) {
        return f64::NAN;
    }
    (2..=n as u32).fold(1.0, |acc, k| acc * k as f64)
}

#[derive(Clone, Copy)]
struct Value {
    value: f64,
    percent: bool,
}

impl Value {
    fn of(value: f64) -> Self {
        Self { value, percent: false }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn is_op(&self, op: char) -> bool {
        self.tokens.get(self.pos) == Some(&Token::Op(op))
    }

    fn primary(&mut self) -> Option<Value> {
        let token = self.tokens.get(self.pos)?.clone();
        self.pos += 1;
        match token {
            Token::Num(n) => Some(Value::of(n)),
            Token::Op('(') => {
                let inner = self.expression()?;
                if !self.is_op(')') {
                    return None;
                }
                self.pos += 1;
                Some(Value::of(inner))
            }
            Token::Id(name) => match constant(&name) {
                Some(value) => Some(Value::of(value)),
                None => {
                    let f = function(&name)?;
                    Some(Value::of(f(self.unary()?.value)))
                }
            },
            Token::Op(_) => None,
        }
    }

    fn postfix(&mut self) -> Option<Value> {
        let mut result = self.primary()?;
        loop {
            if self.is_op('!') {
                self.pos += 1;
                result = Value::of(factorial(result.value));
            } else if self.is_op('%') {
                self.pos += 1;
                result = Value { value: result.value / 100.0, percent: true };
            } else if matches!(self.tokens.get(self.pos), Some(Token::Id(_))) || self.is_op('(') {
                result = Value::of(result.value * self.postfix()?.value);
            } else {
                return Some(result);
            }
        }
    }

    fn power(&mut self) -> Option<Value> {
        let base = self.postfix()?;
        if !self.is_op('^') {
            return Some(base);
        }
        self.pos += 1;
        Some(Value::of(base.value.powf(self.unary()?.value)))
    }

    fn unary(&mut self) -> Option<Value> {
        if self.is_op('-') {
            self.pos += 1;
            let value = self.unary()?;
            return Some(Value { value: -value.value, percent: value.percent });
        }
        if self.is_op('+') {
            self.pos += 1;
            return self.unary();
        }
        self.power()
    }

    fn term(&mut self) -> Option<f64> {
        let mut result = self.unary()?.value;
        while self.is_op('*') || self.is_op('/') {
            let multiply = self.is_op('*');
            self.pos += 1;
            let right = self.unary()?.value;
            result = if multiply { result * right } else { result / right };
        }
        Some(result)
    }

    fn expression(&mut self) -> Option<f64> {
        let mut result = self.term()?;
        while self.is_op('+') || self.is_op('-') {
            let add = self.is_op('+');
            self.pos += 1;
            let start = self.pos;
            let right = self.unary()?;
            let plain_percent = right.percent && (self.is_op('+') || self.is_op('-') || self.is_op(')') || self.pos >= self.tokens.len());
            let amount = if plain_percent {
                result * right.value
            } else {
                self.pos = start;
                self.term()?
            };
            result = if add { result + amount } else { result - amount };
        }
        Some(result)
    }
}

pub fn evaluate(input: &str) -> Option<f64> {
    let tokens = tokenize(input).unwrap_or_default();
    if tokens.is_empty() {
        return None;
    }
    let mut parser = Parser { tokens, pos: 0 };
    let result = parser.expression()?;
    (parser.pos == parser.tokens.len() && result.is_finite()).then_some(result)
}

pub fn is_expression(input: &str) -> bool {
    let Some(tokens) = tokenize(input) else { return false };
    if tokens.len() < 2 {
        return matches!(tokens.as_slice(), [Token::Id(name)] if constant(name).is_some() && input.trim().chars().count() > 1);
    }
    tokens.iter().any(|t| match t {
        Token::Op(_) => true,
        Token::Id(name) => function(name).is_some() || constant(name).is_some(),
        Token::Num(_) => false,
    })
}

pub fn to_precision(value: f64, digits: usize) -> f64 {
    format!("{:.*e}", digits - 1, value).parse().unwrap_or(value)
}

pub fn group(value: f64, max_fraction: usize) -> String {
    let fixed = format!("{:.*}", max_fraction, value.abs());
    let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let frac = frac.trim_end_matches('0');
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let negative = value < 0.0 && (int != "0" || !frac.is_empty());
    format!("{}{}{}{}", if negative { "-" } else { "" }, grouped, if frac.is_empty() { "" } else { "." }, frac)
}

pub fn format_number(value: f64) -> String {
    group(to_precision(value, 12), 10)
}

pub fn plain_number(value: f64) -> String {
    format!("{}", to_precision(value, 12))
}

const ONES: [&str; 20] = ["Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen", "Seventeen", "Eighteen", "Nineteen"];
const TENS: [&str; 10] = ["", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety"];
const SCALES: [&str; 5] = ["", "Thousand", "Million", "Billion", "Trillion"];

fn below_thousand(n: u64) -> String {
    let (hundreds, rest) = (n / 100, n % 100);
    let mut parts = Vec::new();
    if hundreds > 0 {
        parts.push(format!("{} Hundred", ONES[hundreds as usize]));
    }
    if rest >= 20 {
        parts.push(format!("{}{}", TENS[(rest / 10) as usize], if rest % 10 > 0 { format!(" {}", ONES[(rest % 10) as usize]) } else { String::new() }));
    } else if rest > 0 || parts.is_empty() {
        parts.push(ONES[rest as usize].into());
    }
    parts.join(" ")
}

pub fn number_to_words(value: f64) -> Option<String> {
    if value.fract() != 0.0 || value.abs() >= 1e15 {
        return None;
    }
    if value == 0.0 {
        return Some("Zero".into());
    }
    let mut parts = Vec::new();
    let mut n = value.abs() as u64;
    let mut scale = 0;
    while n > 0 {
        let chunk = n % 1000;
        if chunk > 0 {
            parts.insert(0, format!("{}{}", below_thousand(chunk), if SCALES[scale].is_empty() { String::new() } else { format!(" {}", SCALES[scale]) }));
        }
        n /= 1000;
        scale += 1;
    }
    Some(format!("{}{}", if value < 0.0 { "Minus " } else { "" }, parts.join(" ")))
}

const PT_ONES: [&str; 20] = ["zero", "um", "dois", "três", "quatro", "cinco", "seis", "sete", "oito", "nove", "dez", "onze", "doze", "treze", "quatorze", "quinze", "dezesseis", "dezessete", "dezoito", "dezenove"];
const PT_TENS: [&str; 10] = ["", "", "vinte", "trinta", "quarenta", "cinquenta", "sessenta", "setenta", "oitenta", "noventa"];
const PT_HUNDREDS: [&str; 10] = ["", "cento", "duzentos", "trezentos", "quatrocentos", "quinhentos", "seiscentos", "setecentos", "oitocentos", "novecentos"];

fn pt_below_thousand(n: u64) -> String {
    let (hundreds, rest) = (n / 100, n % 100);
    let mut parts = Vec::new();
    if hundreds > 0 {
        parts.push(if hundreds == 1 && rest == 0 { "cem".to_owned() } else { PT_HUNDREDS[hundreds as usize].to_owned() });
    }
    if rest >= 20 {
        parts.push(format!("{}{}", PT_TENS[(rest / 10) as usize], if rest % 10 > 0 { format!(" e {}", PT_ONES[(rest % 10) as usize]) } else { String::new() }));
    } else if rest > 0 || parts.is_empty() {
        parts.push(PT_ONES[rest as usize].into());
    }
    parts.join(" e ")
}

fn pt_scale(n: u64, scale: usize) -> String {
    let words = pt_below_thousand(n);
    let plural = n > 1;
    match scale {
        0 => words,
        1 if n == 1 => "mil".into(),
        1 => format!("{words} mil"),
        _ => format!("{} {}", if n == 1 { "um".to_owned() } else { words }, match (scale, plural) {
            (2, false) => "milhão",
            (2, true) => "milhões",
            (3, false) => "bilhão",
            (3, true) => "bilhões",
            (_, false) => "trilhão",
            _ => "trilhões",
        }),
    }
}

pub fn number_to_words_pt(value: f64) -> Option<String> {
    if value.fract() != 0.0 || value.abs() >= 1e15 {
        return None;
    }
    if value == 0.0 {
        return Some("Zero".into());
    }
    let mut chunks = Vec::new();
    let mut n = value.abs() as u64;
    let mut scale = 0;
    while n > 0 {
        if n % 1000 > 0 {
            chunks.insert(0, (n % 1000, scale));
        }
        n /= 1000;
        scale += 1;
    }
    let words: Vec<String> = chunks
        .iter()
        .enumerate()
        .map(|(i, (n, scale))| {
            let word = pt_scale(*n, *scale);
            let rest = &chunks[i + 1..];
            let needs_e = rest.len() == 1 && (rest[0].0 < 100 || rest[0].0 % 100 == 0);
            if needs_e { format!("{word} e") } else { word }
        })
        .collect();
    Some(capitalize(&format!("{}{}", if value < 0.0 { "menos " } else { "" }, words.join(" "))))
}

const PT_NUMBERS: [(&str, f64); 49] = [
    ("zero", 0.0), ("um", 1.0), ("uma", 1.0), ("dois", 2.0), ("duas", 2.0), ("tres", 3.0), ("quatro", 4.0), ("cinco", 5.0), ("seis", 6.0), ("sete", 7.0), ("oito", 8.0), ("nove", 9.0),
    ("dez", 10.0), ("onze", 11.0), ("doze", 12.0), ("treze", 13.0), ("quatorze", 14.0), ("catorze", 14.0), ("quinze", 15.0), ("dezesseis", 16.0), ("dezessete", 17.0), ("dezoito", 18.0), ("dezenove", 19.0),
    ("vinte", 20.0), ("trinta", 30.0), ("quarenta", 40.0), ("cinquenta", 50.0), ("sessenta", 60.0), ("setenta", 70.0), ("oitenta", 80.0), ("noventa", 90.0),
    ("cem", 100.0), ("cento", 100.0), ("duzentos", 200.0), ("duzentas", 200.0), ("trezentos", 300.0), ("trezentas", 300.0), ("quatrocentos", 400.0), ("quatrocentas", 400.0), ("quinhentos", 500.0), ("quinhentas", 500.0),
    ("seiscentos", 600.0), ("seiscentas", 600.0), ("setecentos", 700.0), ("setecentas", 700.0), ("oitocentos", 800.0), ("oitocentas", 800.0), ("novecentos", 900.0), ("novecentas", 900.0),
];
const PT_SCALES: [(&str, f64); 5] = [("mil", 1e3), ("milhao", 1e6), ("milhoes", 1e6), ("bilhao", 1e9), ("bilhoes", 1e9)];
const PT_OPERATORS: [(&str, &str); 7] = [("multiplicado por", "*"), ("dividido por", "/"), ("elevado a", "^"), ("por cento", "%"), ("mais", "+"), ("menos", "-"), ("vezes", "*")];

fn word_value(table: &[(&str, f64)], word: &str) -> Option<f64> {
    table.iter().find(|(w, _)| *w == word).map(|(_, v)| *v)
}

pub fn spell_numbers(text: &str) -> Option<String> {
    let words: Vec<String> = unaccent(&text.to_lowercase()).split_whitespace().map(str::to_owned).collect();
    let is_word = |w: &str| word_value(&PT_NUMBERS, w).or_else(|| word_value(&PT_SCALES, w)).is_some();
    let (mut out, mut changed, mut i) = (Vec::new(), false, 0);
    while i < words.len() {
        let operator = PT_OPERATORS.iter().find(|(phrase, _)| {
            let n = phrase.split(' ').count();
            words.get(i..i + n).is_some_and(|w| w.join(" ") == *phrase)
        });
        if let Some((phrase, op)) = operator {
            out.push(op.to_string());
            i += phrase.split(' ').count();
            changed = true;
            continue;
        }
        let (mut total, mut current, mut j, mut spelled) = (0.0, 0.0, i, false);
        while let Some(w) = words.get(j) {
            if let Some(v) = word_value(&PT_NUMBERS, w) {
                current += v;
                spelled = true;
            } else if let Some(scale) = word_value(&PT_SCALES, w) {
                total += if current == 0.0 { 1.0 } else { current } * scale;
                current = 0.0;
                spelled = true;
            } else if w == "e" && spelled && words.get(j + 1).is_some_and(|n| is_word(n)) {
            } else if j == i && words.get(j + 1).is_some_and(|n| word_value(&PT_SCALES, n).is_some()) && w.replace(',', ".").parse::<f64>().is_ok() {
                current = w.replace(',', ".").parse().unwrap_or(0.0);
            } else {
                break;
            }
            j += 1;
        }
        if spelled {
            out.push(format!("{}", total + current));
            changed = true;
            i = j;
        } else {
            out.push(words[i].clone());
            i += 1;
        }
    }
    changed.then(|| out.join(" "))
}

pub fn unaccent(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            'ý' | 'ÿ' => 'y',
            'Ý' => 'Y',
            other => other,
        })
        .collect()
}

pub const CURRENCY_SYMBOLS: [(&str, &str); 13] = [("R$", "BRL"), ("US$", "USD"), ("C$", "CAD"), ("A$", "AUD"), ("$", "USD"), ("€", "EUR"), ("£", "GBP"), ("¥", "JPY"), ("₹", "INR"), ("₩", "KRW"), ("₽", "RUB"), ("₺", "TRY"), ("₿", "BTC")];

const CURRENCY_WORDS: [(&str, &str); 30] = [
    ("dolar", "USD"),
    ("dolares", "USD"),
    ("dollars", "USD"),
    ("dollar", "USD"),
    ("real", "BRL"),
    ("reais", "BRL"),
    ("euro", "EUR"),
    ("euros", "EUR"),
    ("libra", "GBP"),
    ("libras", "GBP"),
    ("pound", "GBP"),
    ("pounds", "GBP"),
    ("iene", "JPY"),
    ("ienes", "JPY"),
    ("yen", "JPY"),
    ("yens", "JPY"),
    ("yuan", "CNY"),
    ("won", "KRW"),
    ("rublo", "RUB"),
    ("rublos", "RUB"),
    ("bitcoin", "BTC"),
    ("bitcoins", "BTC"),
    ("ethereum", "ETH"),
    ("ether", "ETH"),
    ("solana", "SOL"),
    ("ripple", "XRP"),
    ("dogecoin", "DOGE"),
    ("cardano", "ADA"),
    ("litecoin", "LTC"),
    ("tether", "USDT"),
];

pub const CONNECTORS: &str = "in|to|as|para|em|como|pra|pro";

#[derive(PartialEq, Debug)]
pub struct CurrencyQuery {
    pub amount: f64,
    pub from: String,
    pub to: String,
}

fn currency_code(token: &str, known: &HashSet<String>) -> Option<String> {
    let lower = unaccent(&token.to_lowercase());
    if let Some((_, code)) = CURRENCY_WORDS.iter().find(|(w, _)| *w == lower) {
        return Some(code.to_string());
    }
    let upper = token.to_uppercase();
    if let Some((_, code)) = CURRENCY_SYMBOLS.iter().find(|(s, _)| *s == upper || *s == token) {
        return Some(code.to_string());
    }
    known.contains(&upper).then_some(upper)
}

static CURRENCY: LazyLock<Regex> = LazyLock::new(|| {
    let symbols = CURRENCY_SYMBOLS.iter().map(|(s, _)| regex::escape(s)).collect::<Vec<_>>().join("|");
    let unit = format!(r"(?:{symbols}|\p{{L}}+)");
    Regex::new(&format!(r"(?i)^({unit})?\s*(\d+(?:[.,]\d+)?)\s*({unit})?\s+(?:{CONNECTORS}|=)\s+({unit})$")).unwrap()
});

pub fn parse_currency(query: &str, known: &HashSet<String>) -> Option<CurrencyQuery> {
    let caps = CURRENCY.captures(query.trim())?;
    let (before, after) = (caps.get(1), caps.get(3));
    if before.is_some() == after.is_some() {
        return None;
    }
    let from = currency_code(before.or(after)?.as_str(), known)?;
    let to = currency_code(caps.get(4)?.as_str(), known)?;
    if from == to {
        return None;
    }
    Some(CurrencyQuery { amount: caps[2].replace(',', ".").parse().ok()?, from, to })
}

pub fn convert_currency(query: &CurrencyQuery, rates: &std::collections::HashMap<String, f64>) -> Option<f64> {
    let (from, to) = (rates.get(&query.from)?, rates.get(&query.to)?);
    (*from != 0.0 && *to != 0.0).then(|| query.amount / from * to)
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() < 1e-9)
    }

    #[test]
    fn follows_precedence_parentheses_and_powers() {
        assert_eq!(evaluate("2 + 3 * 4"), Some(14.0));
        assert_eq!(evaluate("(2 + 3) * 4"), Some(20.0));
        assert_eq!(evaluate("2^3^2"), Some(512.0));
        assert_eq!(evaluate("-2^2"), Some(-4.0));
        assert_eq!(evaluate("10 / 4"), Some(2.5));
        assert_eq!(evaluate("6 × 7 ÷ 2"), Some(21.0));
    }

    #[test]
    fn supports_functions_constants_factorial_and_implicit_multiplication() {
        assert_eq!(evaluate("sqrt(16) + 5!"), Some(124.0));
        assert!(close(evaluate("2pi"), 2.0 * std::f64::consts::PI));
        assert_eq!(evaluate("3(4 + 1)"), Some(15.0));
        assert_eq!(evaluate("1,000 * 2"), Some(2000.0));
    }

    #[test]
    fn treats_percentages_like_a_calculator() {
        assert_eq!(evaluate("50%"), Some(0.5));
        assert!(close(evaluate("200 + 10%"), 220.0));
        assert_eq!(evaluate("200 - 25%"), Some(150.0));
        assert_eq!(evaluate("20% * 50"), Some(10.0));
    }

    #[test]
    fn rejects_text_that_is_not_math() {
        assert_eq!(evaluate("hello world"), None);
        assert_eq!(evaluate("2 +"), None);
        assert_eq!(evaluate("1 / 0"), None);
        assert!(!is_expression("time in china"));
        assert!(!is_expression("42"));
        assert!(is_expression("2+2"));
        assert!(is_expression("pi"));
    }

    #[test]
    fn formats_results_and_spells_integers() {
        assert_eq!(format_number(1234567.5), "1,234,567.5");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(plain_number(0.1 + 0.2), "0.3");
        assert_eq!(number_to_words(1214.0).as_deref(), Some("One Thousand Two Hundred Fourteen"));
        assert_eq!(number_to_words(-40.0).as_deref(), Some("Minus Forty"));
        assert_eq!(number_to_words(2.5), None);
        assert_eq!(number_to_words_pt(1214.0).as_deref(), Some("Mil duzentos e quatorze"));
        assert_eq!(number_to_words_pt(2300.0).as_deref(), Some("Dois mil e trezentos"));
        assert_eq!(number_to_words_pt(100.0).as_deref(), Some("Cem"));
        assert_eq!(number_to_words_pt(1_000_001.0).as_deref(), Some("Um milhão e um"));
        assert_eq!(number_to_words_pt(-21.0).as_deref(), Some("Menos vinte e um"));
    }

    #[test]
    fn reads_portuguese_number_words() {
        assert_eq!(spell_numbers("dois mais três").as_deref(), Some("2 + 3"));
        assert_eq!(spell_numbers("cem reais em dólares").as_deref(), Some("100 reais em dolares"));
        assert_eq!(spell_numbers("cento e vinte e cinco dividido por cinco").as_deref(), Some("125 / 5"));
        assert_eq!(spell_numbers("dois mil e trezentos vezes 2").as_deref(), Some("2300 * 2"));
        assert_eq!(spell_numbers("um milhão e duzentos mil").as_deref(), Some("1200000"));
        assert_eq!(spell_numbers("10 mil reais em usd").as_deref(), Some("10000 reais em usd"));
        assert_eq!(spell_numbers("mil e um").as_deref(), Some("1001"));
        assert_eq!(spell_numbers("dez por cento").as_deref(), Some("10 %"));
        assert_eq!(spell_numbers("rock e roll"), None);
        assert_eq!(spell_numbers("100 usd in eur"), None);
        assert_eq!(evaluate(&spell_numbers("dois elevado a dez").unwrap()), Some(1024.0));
    }

    fn known() -> HashSet<String> {
        ["USD", "BRL", "EUR"].iter().map(|s| s.to_string()).collect()
    }

    fn q(amount: f64, from: &str, to: &str) -> Option<CurrencyQuery> {
        Some(CurrencyQuery { amount, from: from.into(), to: to.into() })
    }

    #[test]
    fn parses_currency_symbols_codes_and_words() {
        let known = known();
        assert_eq!(parse_currency("100$ in brl", &known), q(100.0, "USD", "BRL"));
        assert_eq!(parse_currency("€50 to usd", &known), q(50.0, "EUR", "USD"));
        assert_eq!(parse_currency("12,5 eur in R$", &known), q(12.5, "EUR", "BRL"));
        assert_eq!(parse_currency("100 in brl", &known), None);
        assert_eq!(parse_currency("100 xyz in brl", &known), None);
        assert_eq!(parse_currency("100 dolares para brl", &known), q(100.0, "USD", "BRL"));
        assert_eq!(parse_currency("100,50 reais em dolares", &known), q(100.5, "BRL", "USD"));
        assert_eq!(parse_currency("10 dólares para real", &known), q(10.0, "USD", "BRL"));
        assert_eq!(parse_currency("1 euro pra real", &known), q(1.0, "EUR", "BRL"));
        let rates = [("USD", 1.0), ("EUR", 0.5), ("BRL", 5.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        assert_eq!(convert_currency(&q(10.0, "EUR", "BRL").unwrap(), &rates), Some(100.0));
    }
}

