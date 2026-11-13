
use rayon::prelude::*;
use tonic::service::LayerExt;

pub struct ProcessorDefines {
    macros: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
}

impl ProcessorDefines {
    pub fn new() -> Self {
        ProcessorDefines {
            macros: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn msvc_c() -> Self {
        let buildin_macros = std::collections::HashMap::from([
            ("_WIN32".to_string(), None),
            ("_WIN64".to_string(), None),
            ("_MSC_VER".to_string(), Some("1944".to_string())),
            ("_MSC_FULL_VER".to_string(), Some("194435219".to_string())),
            ("_MSC_BUILD".to_string(), None),
            ("_MSC_EXTENSIONS".to_string(), None),
            ("_MSVC_CONSTEXPR_ATTRIBUTE".to_string(), None),
            ("_MSVC_EXECUTION_CHARACTER_SET".to_string(), Some("65001".to_string())),

            ("_M_AMD64".to_string(), Some("100".to_string())),
            ("_M_X64".to_string(), Some("100".to_string())),
            ("__amd64".to_string(), None),
            ("__amd64__".to_string(), None),
            ("__x86_64".to_string(), None),

            ("_MT".to_string(), None),
            ("_M_FP_CONTRACT".to_string(), None),
            ("_M_FP_PRECISE".to_string(), None),
            ("_CRT_USE_BUILTIN_OFFSETOF".to_string(), None),
            ("_INTEGRAL_MAX_BITS".to_string(), Some("64".to_string())),

            ("__STDC_VERSION__".to_string(), Some("201710L".to_string())),
            ("__STDC_HOSTED__".to_string(), None),
            ("__STDC_UTF_16__".to_string(), None),
            ("__STDC_UTF_32__".to_string(), None),
        ]);
        return ProcessorDefines {
            macros: std::sync::Arc::new(std::sync::RwLock::new(buildin_macros)),
        };
    }

    pub fn msvc_cpp() -> Self {
        let buildin_macros = std::collections::HashMap::from([
            ("_WIN32".to_string(), None),
            ("_WIN64".to_string(), None),
            ("_MSC_VER".to_string(), Some("1944".to_string())),
            ("_MSC_FULL_VER".to_string(), Some("194435219".to_string())),
            ("_MSC_BUILD".to_string(), None),
            ("_MSC_EXTENSIONS".to_string(), None),
            ("_MSVC_CONSTEXPR_ATTRIBUTE".to_string(), None),
            ("_MSVC_EXECUTION_CHARACTER_SET".to_string(), Some("65001".to_string())),

            ("_M_AMD64".to_string(), Some("100".to_string())),
            ("_M_X64".to_string(), Some("100".to_string())),
            ("__amd64".to_string(), None),
            ("__amd64__".to_string(), None),
            ("__x86_64".to_string(), None),

            ("_MT".to_string(), None),
            ("_M_FP_CONTRACT".to_string(), None),
            ("_M_FP_PRECISE".to_string(), None),
            ("_CRT_USE_BUILTIN_OFFSETOF".to_string(), None),
            ("_INTEGRAL_MAX_BITS".to_string(), Some("64".to_string())),

            ("_MSVC_LANG".to_string(), Some("201402L".to_string())),
            ("__cplusplus".to_string(), Some("201402L".to_string())),
        ]);
        return ProcessorDefines {
            macros: std::sync::Arc::new(std::sync::RwLock::new(buildin_macros)),
        };
    }
}

fn trim(b: &[u8]) -> &[u8] {
    let b = b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b);
    let n = b.iter().position(|&c| c != b' ' && c != b'\t').unwrap_or(b.len());
    let m = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\n' && c != b'\r');

    match m {
        Some(idx) if idx >= n => &b[n..=idx],
        _ => &b[0..0],
    }
}

fn first_ident(s: &str) -> &str {
    let s = s.trim_start();
    let end = s.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(s.len());
    &s[..end]
}

fn extract_line_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut in_quotes = false;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_quotes = !in_quotes;
        } else if !in_quotes && i + 1 < bytes.len() && bytes[i] == b'/' {
            if bytes[i + 1] == b'/' || bytes[i + 1] == b'*' {
                return s[..i].trim_end();
            }
        }
        i += 1;
    }
    s.trim_end()
}

fn split_directive(b: &[u8]) -> (&[u8], &[u8]) {
    let end = b.iter().position(|&c| c == b' ' || c == b'\t').unwrap_or(b.len());
    (&b[..end], if end < b.len() { trim(&b[end..]) } else { b"" })
}

fn literal_include_operand(s: &str) -> Option<&str> {
    let closing = match s.as_bytes().first().copied()? {
        b'"' => '"',
        b'<' => '>',
        _ => return None,
    };
    let operand = &s[1..];
    Some(&operand[..operand.find(closing)?])
}

fn expand_has_include(
    expr: &str,
    current_file: &std::path::Path,
    user_includes: &[std::path::PathBuf],
    sys_includes: &[std::path::PathBuf],
) -> String {
    const KW: &str = "__has_include";
    if !expr.contains(KW) { return expr.to_string(); }
    let mut out = String::new();
    let mut s = expr;
    loop {
        match s.find(KW) {
            None => { out.push_str(s); break; }
            Some(pos) => {
                let before_ok = pos == 0 || {
                    let c = s.as_bytes()[pos - 1];
                    !c.is_ascii_alphanumeric() && c != b'_'
                };
                let end_kw = pos + KW.len();
                let after_ok = end_kw >= s.len() || {
                    let c = s.as_bytes()[end_kw];
                    !c.is_ascii_alphanumeric() && c != b'_'
                };
                if !before_ok || !after_ok {
                    out.push_str(&s[..pos + 1]);
                    s = &s[pos + 1..];
                    continue;
                }
                out.push_str(&s[..pos]);
                let rest = s[end_kw..].trim_start();
                if !rest.starts_with('(') {
                    out.push_str("0");
                    s = rest;
                    continue;
                }
                let inner = rest[1..].trim_start();
                let (path, is_angle, tail) = if inner.starts_with('"') {
                    match inner[1..].find('"') {
                        None => { out.push_str("0"); s = &rest[1..]; continue; }
                        Some(e) => {
                            let p = &inner[1..e+1];
                            let a = inner[e+2..].trim_start();
                            let a = if a.starts_with(')') { &a[1..] } else { a };
                            (p, false, a)
                        }
                    }
                } else if inner.starts_with('<') {
                    match inner[1..].find('>') {
                        None => { out.push_str("0"); s = &rest[1..]; continue; }
                        Some(e) => {
                            let p = &inner[1..e+1];
                            let a = inner[e+2..].trim_start();
                            let a = if a.starts_with(')') { &a[1..] } else { a };
                            (p, true, a)
                        }
                    }
                } else {
                    out.push_str("0");
                    s = &rest[1..];
                    continue;
                };
                let found = if !is_angle {
                    current_file.parent()
                        .map(|p| p.join(path).is_file()).unwrap_or(false)
                    || user_includes.iter().chain(sys_includes.iter())
                        .any(|inc| inc.join(path).is_file())
                } else {
                    user_includes.iter().chain(sys_includes.iter())
                        .any(|inc| inc.join(path).is_file())
                };
                out.push_str(if found { "1" } else { "0" });
                s = tail;
            }
        }
    }
    out
}

mod expr_eval {
    use std::collections::HashMap;
    pub fn eval(expr: &str, defines: &HashMap<String, Option<String>>) -> i64 {
        let tokens = tokenize(expr);
        let mut pos = 0usize;
        parse_ternary(&tokens, &mut pos, defines)
    }
    #[derive(Debug, Clone, PartialEq)]
    enum Tok {
        Num(i64), Ident(String),
        LParen, RParen,
        Bang, Tilde, Plus, Minus, Star, Slash, Percent,
        LShift, RShift,
        Lt, Gt, Le, Ge, Eq, Ne,
        Amp, Caret, Pipe, AmpAmp, PipePipe,
        Question, Colon, Eof,
    }
    fn tokenize(s: &str) -> Vec<Tok> {
        let b = s.as_bytes();
        let mut pos = 0;
        let mut out = Vec::new();
        while pos < b.len() {
            while pos < b.len() && b[pos].is_ascii_whitespace() { pos += 1; }
            if pos >= b.len() { break; }
            let c = b[pos];
            if c.is_ascii_digit() {
                let start = pos;
                while pos < b.len() && (b[pos].is_ascii_alphanumeric() || b[pos] == b'_') { pos += 1; }
                out.push(Tok::Num(parse_int(&s[start..pos]))); continue;
            }
            if c.is_ascii_alphabetic() || c == b'_' {
                let start = pos;
                while pos < b.len() && (b[pos].is_ascii_alphanumeric() || b[pos] == b'_') { pos += 1; }
                out.push(Tok::Ident(s[start..pos].to_string())); continue;
            }
            pos += 1;
            match c {
                b'(' => out.push(Tok::LParen), b')' => out.push(Tok::RParen),
                b'~' => out.push(Tok::Tilde), b'*' => out.push(Tok::Star),
                b'/' => {
                    if pos < b.len() && b[pos] == b'/' {
                        break;
                    } else if pos < b.len() && b[pos] == b'*' {
                        pos += 1;
                        while pos + 1 < b.len() && !(b[pos] == b'*' && b[pos + 1] == b'/') {
                            pos += 1;
                        }
                        if pos + 1 < b.len() { pos += 2; } else { break; }
                    } else {
                        out.push(Tok::Slash);
                    }
                }
                b'%' => out.push(Tok::Percent),
                b'+' => out.push(Tok::Plus),  b'-' => out.push(Tok::Minus),
                b'?' => out.push(Tok::Question), b':' => out.push(Tok::Colon),
                b'^' => out.push(Tok::Caret),
                b'!' => if pos < b.len() && b[pos]==b'=' { pos+=1; out.push(Tok::Ne); } else { out.push(Tok::Bang); }
                b'<' => if pos < b.len() && b[pos]==b'<' { pos+=1; out.push(Tok::LShift); }
                        else if pos < b.len() && b[pos]==b'=' { pos+=1; out.push(Tok::Le); }
                        else { out.push(Tok::Lt); }
                b'>' => if pos < b.len() && b[pos]==b'>' { pos+=1; out.push(Tok::RShift); }
                        else if pos < b.len() && b[pos]==b'=' { pos+=1; out.push(Tok::Ge); }
                        else { out.push(Tok::Gt); }
                b'=' => if pos < b.len() && b[pos]==b'=' { pos+=1; out.push(Tok::Eq); }
                b'&' => if pos < b.len() && b[pos]==b'&' { pos+=1; out.push(Tok::AmpAmp); } else { out.push(Tok::Amp); }
                b'|' => if pos < b.len() && b[pos]==b'|' { pos+=1; out.push(Tok::PipePipe); } else { out.push(Tok::Pipe); }
                _ => {}
            }
        }
        out.push(Tok::Eof); out
    }
    fn parse_int(s: &str) -> i64 {
        let s = s.trim_end_matches(|c: char| matches!(c, 'u'|'U'|'l'|'L'));
        if s.starts_with("0x") || s.starts_with("0X") { i64::from_str_radix(&s[2..], 16).unwrap_or(0) }
        else if s.starts_with("0b") || s.starts_with("0B") { i64::from_str_radix(&s[2..], 2).unwrap_or(0) }
        else if s.len() > 1 && s.starts_with('0') { i64::from_str_radix(&s[1..], 8).unwrap_or(0) }
        else { s.parse().unwrap_or(0) }
    }
    fn peek(t: &[Tok], p: &usize) -> Tok { t.get(*p).cloned().unwrap_or(Tok::Eof) }
    fn eat(t: &[Tok], p: &mut usize, tok: &Tok) { if peek(t, p) == *tok { *p += 1; } }
    fn parse_ternary(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let c = parse_or(t, p, d);
        if peek(t, p) == Tok::Question {
            *p += 1; let y = parse_or(t, p, d); eat(t, p, &Tok::Colon); let n = parse_or(t, p, d);
            if c != 0 { y } else { n }
        } else { c }
    }
    fn parse_or(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_and(t, p, d);
        while peek(t,p)==Tok::PipePipe { *p+=1; let r=parse_and(t,p,d); v=if v!=0||r!=0{1}else{0}; } v
    }
    fn parse_and(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_bitor(t, p, d);
        while peek(t,p)==Tok::AmpAmp { *p+=1; let r=parse_bitor(t,p,d); v=if v!=0&&r!=0{1}else{0}; } v
    }
    fn parse_bitor(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_bitxor(t, p, d);
        while peek(t,p)==Tok::Pipe { *p+=1; v|=parse_bitxor(t,p,d); } v
    }
    fn parse_bitxor(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_bitand(t, p, d);
        while peek(t,p)==Tok::Caret { *p+=1; v^=parse_bitand(t,p,d); } v
    }
    fn parse_bitand(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_eq(t, p, d);
        while peek(t,p)==Tok::Amp { *p+=1; v&=parse_eq(t,p,d); } v
    }
    fn parse_eq(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_rel(t, p, d);
        loop { match peek(t,p) {
            Tok::Eq=>{*p+=1;let r=parse_rel(t,p,d);v=if v==r{1}else{0};}
            Tok::Ne=>{*p+=1;let r=parse_rel(t,p,d);v=if v!=r{1}else{0};}
            _=>break,
        }} v
    }
    fn parse_rel(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_shift(t, p, d);
        loop { match peek(t,p) {
            Tok::Lt=>{*p+=1;let r=parse_shift(t,p,d);v=if v<r{1}else{0};}
            Tok::Gt=>{*p+=1;let r=parse_shift(t,p,d);v=if v>r{1}else{0};}
            Tok::Le=>{*p+=1;let r=parse_shift(t,p,d);v=if v<=r{1}else{0};}
            Tok::Ge=>{*p+=1;let r=parse_shift(t,p,d);v=if v>=r{1}else{0};}
            _=>break,
        }} v
    }
    fn parse_shift(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_add(t, p, d);
        loop { match peek(t,p) {
            Tok::LShift=>{*p+=1;let r=parse_add(t,p,d);v<<=r;}
            Tok::RShift=>{*p+=1;let r=parse_add(t,p,d);v>>=r;}
            _=>break,
        }} v
    }
    fn parse_add(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_mul(t, p, d);
        loop { match peek(t,p) {
            Tok::Plus =>{*p+=1;v+=parse_mul(t,p,d);}
            Tok::Minus=>{*p+=1;v-=parse_mul(t,p,d);}
            _=>break,
        }} v
    }
    fn parse_mul(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        let mut v = parse_unary(t, p, d);
        loop { match peek(t,p) {
            Tok::Star   =>{*p+=1;v*=parse_unary(t,p,d);}
            Tok::Slash  =>{*p+=1;let r=parse_unary(t,p,d);if r!=0{v/=r};}
            Tok::Percent=>{*p+=1;let r=parse_unary(t,p,d);if r!=0{v%=r};}
            _=>break,
        }} v
    }
    fn parse_unary(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        match peek(t,p) {
            Tok::Bang =>{*p+=1;let v=parse_unary(t,p,d);if v==0{1}else{0}}
            Tok::Tilde=>{*p+=1;!parse_unary(t,p,d)}
            Tok::Minus=>{*p+=1;-parse_unary(t,p,d)}
            Tok::Plus =>{*p+=1;parse_unary(t,p,d)}
            _=>parse_primary(t,p,d),
        }
    }
    fn parse_primary(t: &[Tok], p: &mut usize, d: &HashMap<String, Option<String>>) -> i64 {
        match peek(t,p) {
            Tok::LParen=>{*p+=1;let v=parse_ternary(t,p,d);eat(t,p,&Tok::RParen);v}
            Tok::Num(n)=>{let n=n;*p+=1;n}
            Tok::Ident(name)=>{
                let name=name;*p+=1;
                if name == "defined" {
                    let macro_name = if peek(t,p)==Tok::LParen {
                        *p+=1;
                        let n=if let Tok::Ident(n)=peek(t,p){*p+=1;n}else{String::new()};
                        eat(t,p,&Tok::RParen); n
                    } else {
                        if let Tok::Ident(n)=peek(t,p){*p+=1;n}else{String::new()}
                    };
                    if d.contains_key(&macro_name) { 1 } else { 0 }
                } else {
                    match d.get(&name) {
                        Some(Some(s)) => { let s=s.clone(); eval(&s, d) }
                        Some(None)    => 1,
                        None          => 0,
                    }
                }
            }
            _=>{*p+=1;0}
        }
    }
}

fn parse_define(s: &str, defines: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>) {
    let s = extract_line_comment(s).trim();
    let (name, val) = if let Some(paren) = s.find('(') {
        let space = s.find(char::is_whitespace).unwrap_or(s.len());
        if paren < space { 
            (&s[..paren], None) 
        }
        else { 
            let v = s[space..].trim(); 
            (&s[..space], if v.is_empty() { None } else { Some(v) }) 
        }
    } else {
        let space = s.find(char::is_whitespace).unwrap_or(s.len());
        let v = s[space..].trim();
        (&s[..space], if v.is_empty() { None } else { Some(v) })
    };
    if !name.is_empty() {
        defines.write().unwrap().insert(name.to_string(), val.map(|s| s.to_string()));
    }
}

fn first_ident_bytes(b: &[u8]) -> &[u8] {
    let end = b.iter().position(|&c| !c.is_ascii_alphanumeric() && c != b'_').unwrap_or(b.len());
    &b[..end]
}

fn join_lines(lines: &[&[u8]]) -> String {
    let capacity = lines.iter().map(|line| line.len() + 1).sum::<usize>().saturating_sub(1);
    let mut out = String::with_capacity(capacity);
    for (i, &line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&String::from_utf8_lossy(line));
    }
    out
}

// Includes is a resolved literal include outside a conditional group.
// Define stores a standalone directive or one complete conditional group.
#[derive(Debug, PartialEq, Eq)]
pub enum ExtractIncludeItem {
    Includes(String),
    Define(String),
}

enum SourceItem<'a> {
    RawLine(&'a [u8]),
    IncludeLine(&'a str),
}

pub fn extract_sourcefile_includes(content: &[u8], include_dirs: &Vec<std::path::PathBuf>) -> Vec<ExtractIncludeItem> {
    struct ConditionalGroup { start: usize, has_effect: bool }
    enum GuardState<'a> {
        Initial,
        PendingIfndef { line: &'a [u8], ident: &'a [u8] },
        Done,
    }

    let mut items = Vec::new();
    let mut group_lines: Vec<&[u8]> = Vec::new();
    let mut groups: Vec<ConditionalGroup> = Vec::new();
    let mut guard_state = GuardState::Initial;
    let mut comment_stack_depth = 0usize;

    fn trim_left(b: &[u8]) -> &[u8] {
        &b[b.iter().position(|&c| c != b' ' && c != b'\t').unwrap_or(b.len())..]
    }
    fn trim_right(b: &[u8]) -> &[u8] {
        &b[..b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\r').map_or(0, |i| i + 1)]
    }
    
    let content = content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content);
    for line in content.split(|&b| b == b'\n') {
        let line = trim_left(line);
        if line.is_empty() || line[0] == b'\r' {
            continue;
        }
        if line[0] != b'#' {
            if line.starts_with(b"/*") {
                if !trim_right(line).ends_with(b"*/") {
                    comment_stack_depth += 1;
                }
            } else if comment_stack_depth > 0 && trim_right(line).ends_with(b"*/") {
                comment_stack_depth -= 1;
            }
            continue;
        }
        if comment_stack_depth > 0 {
            continue;
        }

        let trimmed = trim_right(line);
        let after_hash = trim_left(&trimmed[1..]);
        let dir_end = after_hash.iter().position(|&c| c == b' ' || c == b'\t').unwrap_or(after_hash.len());
        let dir = &after_hash[..dir_end];

        if !matches!(dir, b"if" | b"ifdef" | b"ifndef" | b"elif" | b"else" | b"endif" | b"include" | b"define" | b"undef") {
            continue;
        }

        match guard_state {
            GuardState::Initial => {
                if dir == b"ifndef" {
                    let ident = first_ident_bytes(trim_left(&after_hash[dir_end..]));
                    if !ident.is_empty() {
                        guard_state = GuardState::PendingIfndef { line: trimmed, ident };
                        continue;
                    }
                }
                guard_state = GuardState::Done;
            }
            GuardState::PendingIfndef { line: ifndef_line, ident: guard_ident } => {
                guard_state = GuardState::Done;
                if dir == b"define" && first_ident_bytes(trim_left(&after_hash[dir_end..])) == guard_ident {
                    continue;
                }
                groups.push(ConditionalGroup { start: group_lines.len(), has_effect: false });
                group_lines.push(ifndef_line);
            }
            GuardState::Done => {}
        }

        match dir {
            b"if" | b"ifdef" | b"ifndef" => {
                groups.push(ConditionalGroup { start: group_lines.len(), has_effect: false });
                group_lines.push(trimmed);
            }
            b"elif" | b"else" => {
                if !groups.is_empty() {
                    group_lines.push(trimmed);
                }
            }
            b"endif" => {
                if let Some(group) = groups.pop() {
                    if group.has_effect {
                        group_lines.push(trimmed);
                        if let Some(parent) = groups.last_mut() {
                            parent.has_effect = true;
                        } else {
                            items.push(ExtractIncludeItem::Define(join_lines(&group_lines)));
                            group_lines.clear();
                        }
                    } else {
                        group_lines.truncate(group.start);
                    }
                }
            }
            b"include" => {
                if let Some(parent) = groups.last_mut() {
                    group_lines.push(trimmed);
                    parent.has_effect = true;
                } else {
                    let rest = trim_left(&after_hash[dir_end..]);
                    if let Ok(s) = std::str::from_utf8(rest) {
                        if let Some(operand) = literal_include_operand(s) {
                            if let Some(path) = check_local_include_dir_and_files_2(include_dirs, operand) {
                                items.push(ExtractIncludeItem::Includes(path.to_string_lossy().into_owned()));
                                continue;
                            }
                        }
                    }
                    items.push(ExtractIncludeItem::Define(String::from_utf8_lossy(trimmed).into_owned()));
                }
            }
            b"define" | b"undef" => {
                if let Some(parent) = groups.last_mut() {
                    group_lines.push(trimmed);
                    parent.has_effect = true;
                } else {
                    items.push(ExtractIncludeItem::Define(String::from_utf8_lossy(trimmed).into_owned()));
                }
            }
            _ => {}
        }
    }

    while let Some(group) = groups.pop() {
        if group.has_effect {
            if let Some(parent) = groups.last_mut() {
                parent.has_effect = true;
            } else if !group_lines.is_empty() {
                items.push(ExtractIncludeItem::Define(join_lines(&group_lines)));
                group_lines.clear();
            }
        } else {
            group_lines.truncate(group.start);
        }
    }

    items
}

pub type ExtractedDependencyCode = std::sync::Arc<dashmap::DashMap<
    String,
    std::sync::Arc<tokio::sync::RwLock<Vec<ExtractIncludeItem>>>,
>>;

fn extract_include_entry(path: &str, includes: &ExtractedDependencyCode) -> (
    std::sync::Arc<tokio::sync::RwLock<Vec<ExtractIncludeItem>>>,
    Option<tokio::sync::OwnedRwLockWriteGuard<Vec<ExtractIncludeItem>>>,
) {
    match includes.entry(path.to_string()) {
        dashmap::mapref::entry::Entry::Occupied(entry) => (entry.get().clone(), None),
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            let lock = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new()));
            let writer = lock.clone().try_write_owned().expect("new lock must be free");
            entry.insert(lock.clone());
            (lock, Some(writer))
        }
    }
}

fn extract_include_entry_is_current(
    path: &str,
    includes: &ExtractedDependencyCode,
    lock: &std::sync::Arc<tokio::sync::RwLock<Vec<ExtractIncludeItem>>>,
) -> bool {
    includes.get(path).is_some_and(|entry| std::sync::Arc::ptr_eq(entry.value(), lock))
}

pub fn include_dirs_with_parent<'a>(
    path: &std::path::Path,
    include_dirs: &'a Vec<std::path::PathBuf>,
) -> std::borrow::Cow<'a, Vec<std::path::PathBuf>> {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    if include_dirs.first().is_some_and(|dir| dir == parent) {
        return std::borrow::Cow::Borrowed(include_dirs);
    }

    let mut dirs = Vec::with_capacity(include_dirs.len() + 1);
    dirs.push(parent.to_path_buf());
    dirs.extend(include_dirs.iter().filter(|dir| dir.as_path() != parent).cloned());
    std::borrow::Cow::Owned(dirs)
}


pub fn parser_sourcefile_dependency(content: &[u8], defines: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
        include_dirs: &std::vec::Vec::<std::path::PathBuf>
    ) -> std::vec::Vec<String> {
    let content = content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content);
    parser_sourcefile_dependency_impl::<true, _, _>(content.split(|&b| b == b'\n').take(36000).map(SourceItem::RawLine), &defines, include_dirs, |_| {})
}

/// Visits includes in source order without retaining a second list of paths.
pub fn parser_sourcefile_dependency_visit_extracted<F>(items: &[ExtractIncludeItem], defines: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
        include_dirs: &std::vec::Vec::<std::path::PathBuf>, on_include: F
    ) where F: FnMut(&str) {

    let _ = parser_sourcefile_dependency_impl::<false, _, _>(items.iter().flat_map(|item| {
        let (raw, fixed) = match item {
            ExtractIncludeItem::Includes(path) => (None, Some(path.as_str())),
            ExtractIncludeItem::Define(text) => (Some(text.as_bytes()), None),
        };
        raw.into_iter()
            .flat_map(|bytes| bytes.split(|&b| b == b'\n'))
            .map(SourceItem::RawLine)
            .chain(fixed.into_iter().map(SourceItem::IncludeLine))
    }), defines, include_dirs, on_include);
}

fn parser_sourcefile_dependency_impl<'a, const COLLECT: bool, F, I>(lines: I, defines: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
        include_dirs: &std::vec::Vec::<std::path::PathBuf>, mut on_include: F
    ) -> std::vec::Vec<String> where F: FnMut(&str), I: Iterator<Item = SourceItem<'a>> {

    struct ConditionFrame { parent_active: bool, branch_taken: bool }
    let mut condition_stack: Vec<ConditionFrame> = Vec::new();
    let mut active = true;

    let mut comment_stack_depth = 0;

    let mut includes = std::vec::Vec::new();

    for item in lines {
        let line = match item {
            SourceItem::RawLine(line) => line,
            SourceItem::IncludeLine(path) => {
                if active {
                    on_include(path);
                    if COLLECT { includes.push(path.to_string()); }
                }
                continue;
            }
        };

        if line.is_empty() || line[0] == b'\r' { continue; }
        let trimmed = trim(line);
        if trimmed.is_empty() { continue; }
        if trimmed[0] != b'#' {
            if trimmed.starts_with(b"/*") {
                if trimmed.ends_with(b"*/") {
                } 
                else {
                    comment_stack_depth += 1;
                }
            } else if trimmed.ends_with(b"*/") && comment_stack_depth > 0 {
                comment_stack_depth -= 1;
            }
            continue; 
        }

        if comment_stack_depth > 0 { continue; }

        let after = trim(&trimmed[1..]);
        let (dir, rest) = split_directive(after);

        match dir {
            b"ifdef" | b"ifndef" | b"if" => {
                let selected = if !active {
                    false
                } else if dir == b"ifdef" || dir == b"ifndef" {
                    let name = first_ident(std::str::from_utf8(rest).unwrap_or(""));
                    let defined = !name.is_empty() && defines.read().unwrap().contains_key(name);
                    if dir == b"ifdef" { defined } else { !defined }
                } else {
                    let expr = std::str::from_utf8(rest).unwrap_or("");
                    let expanded = expand_has_include(expr, std::path::Path::new(""), include_dirs, &[]);
                    expr_eval::eval(&expanded, &defines.read().unwrap()) != 0
                };
                condition_stack.push(ConditionFrame { parent_active: active, branch_taken: selected });
                active = selected;
            }
            b"elif" => {
                if let Some(frame) = condition_stack.last_mut() {
                    active = if frame.parent_active && !frame.branch_taken {
                        let expr = std::str::from_utf8(rest).unwrap_or("");
                        let expanded = expand_has_include(expr, std::path::Path::new(""), include_dirs, &[]);
                        expr_eval::eval(&expanded, &defines.read().unwrap()) != 0
                    } else {
                        false
                    };
                    frame.branch_taken |= active;
                }
            }
            b"else" => {
                if let Some(frame) = condition_stack.last_mut() {
                    active = frame.parent_active && !frame.branch_taken;
                    frame.branch_taken = true;
                }
            }
            b"endif" => {
                if let Some(frame) = condition_stack.pop() { active = frame.parent_active; }
            }
            b"define" if active => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    parse_define(s, defines);
                }
            }
            b"undef" if active => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    let name = first_ident(s);
                    if !name.is_empty() {
                        defines.write().unwrap().remove(name);
                    }
                }
            }
            b"include" if active => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    let resolved = if let Some(operand) = literal_include_operand(s) {
                        check_local_include_dir_and_files_2(include_dirs, operand)
                    } else if s.starts_with('"') || s.starts_with('<') {
                        None
                    } else {
                        let macro_name = first_ident(s);
                        if let Some(Some(expanded)) = defines.read().unwrap().get(macro_name) {
                            let expanded = expanded.trim();
                            let operand = literal_include_operand(expanded).unwrap_or(expanded);
                            check_local_include_dir_and_files_2(include_dirs, operand)
                        } else {
                            None
                        }
                    };

                    if let Some(p) = resolved {
                        let include = p.to_string_lossy().to_string();
                        on_include(&include);
                        if COLLECT {
                            includes.push(include);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    includes
}

fn check_local_include_dir_and_files(include_dir_files: &std::vec::Vec::<(String, std::sync::Arc<std::collections::HashSet<String>>)>, dep: &str) -> Option<std::path::PathBuf> {

    for (dir, files) in include_dir_files.iter() {
        if dep.contains("..") {
            let p = tools::utils::normalize_lexical(dir.to_owned() + "\\" + dep).to_string_lossy().to_string();
            for file in files.iter() {
                if dir.to_owned() + "\\" + file == p {
                    return Some(std::path::PathBuf::from(p));
                }
            }
        }
        else if files.contains(&dep.replace("/", "\\")) {
            return Some(std::path::PathBuf::from(dir).join(dep));
        }
        else {
            if std::path::PathBuf::from(dep).extension() == Some(std::ffi::OsStr::new("cpp")) {
                let path = std::path::PathBuf::from(dir).join(dep);
                if std::fs::exists(&path).unwrap() {
                    return Some(path);
                }
            }
        }
    }

    if include_dir_files.is_empty() {
        return Some(std::path::PathBuf::from(dep));
    }
    else {
        return None;
    }
}

fn check_local_include_dir_and_files_2(include_dirs: &std::vec::Vec::<std::path::PathBuf>, dep: &str) -> Option<std::path::PathBuf> {
    if include_dirs.is_empty() {
        return Some(std::path::PathBuf::from(dep));
    }

    for dir in include_dirs {
        let p = if dep.contains("..") {
            let Some(dir) = dir.to_str() else { continue; };
            tools::utils::normalize_lexical(format!("{}\\{}", dir, dep))
        } else if dep.starts_with("./") {
            dir.join(&dep[2..])
        } else if dep.contains("/") {
            dir.join(dep.replace("/", std::path::MAIN_SEPARATOR_STR))
        } else {
            dir.join(dep)
        };

        let indexed = crate::compiler::model::WALK_FS_NODE.read().unwrap().as_ref()
            .is_some_and(|reader| reader.exists(&p, crate::compiler::walkdir::NodeKind::File));
        if indexed {
            return Some(p);
        }
        if p.is_file() {
            let mut index = crate::compiler::model::WALK_FS_NODE.write().unwrap();
            if let Some(reader) = index.as_mut() {
                match std::fs::metadata(&p) {
                    Ok(metadata) => reader.insfile(&p, crate::compiler::walkdir::file_size_in_kib(metadata.len())),
                    Err(error) => log::warn!("failed to read metadata for {:?}: {:?}", p, error),
                }
            }
            return Some(p);
        }
    }
    None
}

//.inc;.rc;.resx;.idl;.rc2;.def
//.odl;.asm;.asmx;.xsd;.bin;.rgs;.html;.htm;.manifest
//.cpp;.cxx;.cc;.c;.c++;.cppm;.ixx;.inl;.ipp
//.h;.hh;.hpp;.hxx;.h++;.hm

pub fn query_include_dir(include_dirs: &Vec<std::path::PathBuf>) -> std::vec::Vec::<(String, std::sync::Arc<std::collections::HashSet<String>>)> { 
    include_dirs.par_iter().map(|dir| {
        let mut set = std::collections::HashSet::<String>::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if entry.file_type().unwrap().is_file() {
                    if path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()).filter(|ext| ext == "h" || ext == "hpp" 
                        || ext == "hh" || ext == "hxx" || ext == "h++" || ext == "hm" || ext == "inl").is_some() {
                        if let Ok(p) = path.strip_prefix(&dir) {
                            set.insert(p.to_string_lossy().to_string());
                        }
                    }
                }
                else if entry.file_type().unwrap().is_dir() {
                    let name = entry.file_name();
                    if name.to_str().unwrap().starts_with('.') {
                        continue;
                    }
                    query_include_dir_recursive(path, &dir, &mut set);
                }
            }
        }
        (dir.to_string_lossy().to_string(), std::sync::Arc::new(set))
    }).collect()
}

pub fn query_include_dir_ignore(dir: &std::path::PathBuf) {

    let walker =ignore::WalkBuilder::new(dir)
            .hidden(true) 
            .git_ignore(false)
            .build_parallel();

    let (tx, rx) = std::sync::mpsc::channel::<std::path::PathBuf>();
    let dir_ = dir.clone();
    let walk_thread = std::thread::spawn(move || {
        walker.run(|| {

            let tx_ = tx.clone();

            let dir_ = dir_.clone();
            Box::new(move |result| {
                if let Ok(entry) = result {

                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        if let Ok(p) = entry.path().strip_prefix(&dir_) {
                            println!("Found file: {} at depth {}", p.display(), entry.depth());
                            let _ = tx_.send(p.to_path_buf());                           
                        }
                    }
                }
                
                ignore::WalkState::Continue
            })
        });
    });


    let mut files: Vec<std::path::PathBuf> = Vec::new();
    
    for path in rx {
        files.push(path);
    }

    walk_thread.join().unwrap();
}

fn query_include_dir_recursive(dir: std::path::PathBuf, start: &std::path::PathBuf, set: &mut std::collections::HashSet<String>) {

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().unwrap().is_file() {
                if path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()).filter(|ext| ext == "h" || ext == "hpp" 
                    || ext == "hh" || ext == "hxx" || ext == "h++" || ext == "hm" || ext == "inl").is_some() {
                    if let Ok(p) = path.strip_prefix(&start) {
                        set.insert(p.to_string_lossy().to_string());
                    }
                }
            }
            else if entry.file_type().unwrap().is_dir() {
                query_include_dir_recursive(path, &start, set);
            }
        }
    }
}

pub struct DependencyPrepareRequest {
    pub path: String,
    pub items: std::sync::Arc<tokio::sync::RwLock<Vec<ExtractIncludeItem>>>,
    pub writer: Option<tokio::sync::OwnedRwLockWriteGuard<Vec<ExtractIncludeItem>>>,
}

struct PrepareQueueState {
    pending: std::collections::VecDeque<DependencyPrepareRequest>,
    visited: std::collections::HashSet<String>,
    capacity: usize,
    closed: bool,
}

pub struct PrepareQueue {
    state: std::sync::Mutex<PrepareQueueState>,
    cv: std::sync::Condvar,
}

impl PrepareQueue {
    pub fn new() -> std::sync::Arc<Self> {
        Self::with_capacity(usize::MAX)
    }

    fn with_capacity(capacity: usize) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            state: std::sync::Mutex::new(PrepareQueueState {
                pending: std::collections::VecDeque::new(),
                visited: std::collections::HashSet::new(),
                capacity,
                closed: false,
            }),
            cv: std::sync::Condvar::new(),
        })
    }

    fn try_push(&self, request: DependencyPrepareRequest) -> Result<(), DependencyPrepareRequest> {
        let mut state = self.state.lock().unwrap();
        if state.closed || state.pending.len() >= state.capacity {
            return Err(request);
        }
        if request.writer.is_none() && !state.visited.insert(request.path.clone()) {
            return Err(request);
        }
        state.visited.insert(request.path.clone());
        state.pending.push_back(request);
        drop(state);
        self.cv.notify_one();
        Ok(())
    }

    fn take_writer_by_path(
        &self,
        path: &str,
    ) -> Option<tokio::sync::OwnedRwLockWriteGuard<Vec<ExtractIncludeItem>>> {
        let mut state = self.state.lock().unwrap();
        let pos = state.pending.iter().position(|req| req.path == path && req.writer.is_some())?;
        state.pending.remove(pos)?.writer
    }

    pub fn pop(&self) -> Option<DependencyPrepareRequest> {
        let mut state = self.state.lock().unwrap();
        loop {
            if let Some(request) = state.pending.pop_front() {
                return Some(request);
            }
            if state.closed {
                return None;
            }
            state = self.cv.wait(state).unwrap();
        }
    }

    #[cfg(test)]
    pub fn try_pop(&self) -> Option<DependencyPrepareRequest> {
        self.state.lock().unwrap().pending.pop_front()
    }

    pub fn is_closed(&self) -> bool {
        self.state.lock().unwrap().closed
    }

    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        drop(state);
        self.cv.notify_all();
    }
}

pub fn prefetch_unconditional_includes(
    items: &[ExtractIncludeItem],
    includes: &ExtractedDependencyCode,
    cache: Option<&std::sync::Arc<dashmap::DashSet<String>>>,
    queue: &PrepareQueue,
    visited: &std::collections::HashSet<String>,
) {
    if queue.is_closed() {
        return;
    }
    for item in items {
        let ExtractIncludeItem::Includes(path) = item else { continue; };
        let path = path.replace('/', "\\");
        if visited.contains(&path) { continue; }
        let (entry, writer) = extract_include_entry(&path, includes);
        match writer {
            Some(writer) => {
                let request = DependencyPrepareRequest {
                    path: path.clone(),
                    items: entry.clone(),
                    writer: Some(writer),
                };
                if let Err(request) = queue.try_push(request) {
                    includes.remove(&path);
                    drop(entry);
                    drop(request);
                }
            }
            None => {
                if cache.is_some_and(|sent| !sent.contains(&path)) {
                    let request = DependencyPrepareRequest {
                        path,
                        items: entry,
                        writer: None,
                    };
                    let _ = queue.try_push(request);
                }
            }
        }
    }
}

pub fn enqueue_dependency_source(
    path: &str,
    solution: &std::sync::Arc<String>,
    project: &std::sync::Arc<String>,
    cache: &std::sync::Arc<dashmap::DashSet<String>>,
    stream: &tokio::sync::mpsc::Sender<crate::communicate::packager::ArchiveArgs>,
) -> bool {
    if !cache.insert(path.to_string()) { return true; }
    let content = match std::fs::read(path) {
        Ok(content) => bytes::Bytes::from(content),
        Err(error) => {
            log::error!("failed to read file {:?}: {:?}", path, error);
            cache.remove(path);
            return false;
        }
    };
    sync_reserved_dependency_source(path, content, solution, project, cache, stream);
    true
}

fn sync_reserved_dependency_source(
    path: &str,
    content: bytes::Bytes,
    solution: &std::sync::Arc<String>,
    project: &std::sync::Arc<String>,
    cache: &std::sync::Arc<dashmap::DashSet<String>>,
    stream: &tokio::sync::mpsc::Sender<crate::communicate::packager::ArchiveArgs>,
) {
    let name = std::path::Path::new(path).file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_string());

    let archive = crate::communicate::packager::ArchiveArgs {
        file_type: crate::communicate::packager::FileType::SourceFiles,
        solution: solution.as_ref().clone(),
        project: project.as_ref().clone(),
        name,
        path: path.to_string(),
        content,
    };

    if let Err(error) = stream.blocking_send(archive) {
        log::error!("failed to send source file {:?}: {:?}", path, error);
        cache.remove(path);
    }
}

 pub fn extract_sourcefile_and_sync_dependency(
        path: &str,
        mut writer: tokio::sync::OwnedRwLockWriteGuard<Vec<ExtractIncludeItem>>,
        search_dirs: &Vec<std::path::PathBuf>,
        solution: &std::sync::Arc<String>,
        project: &std::sync::Arc<String>,
        cache: &std::sync::Arc<dashmap::DashSet<String>>,
        includes: &ExtractedDependencyCode,
        stream: &tokio::sync::mpsc::Sender<crate::communicate::packager::ArchiveArgs>,
    ) -> bool {
        let content = match std::fs::read(path) {
            Ok(content) => bytes::Bytes::from(content),
            Err(error) => {
                log::error!("failed to read file {:?}: {:?}", path, error);
                includes.remove(path);
                return false;
            }
        };
        
        *writer = extract_sourcefile_includes(&content, search_dirs);
        drop(writer);

        if cache.insert(path.to_string()) {
            sync_reserved_dependency_source(path, content, solution, project, cache, stream);
        }
        true
}

pub fn parser_sourcefile_with_macro_definition_recursive(
    path: &str,
    defines: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
    include_dirs: &std::vec::Vec<std::path::PathBuf>,
    solution: &std::sync::Arc<String>,
    project: &std::sync::Arc<String>,
    cache: &std::sync::Arc<dashmap::DashSet<String>>,
    resolve_dependency_includes: &ExtractedDependencyCode,
    stream: &tokio::sync::mpsc::Sender<crate::communicate::packager::ArchiveArgs>,
    prepare_queue: Option<&PrepareQueue>,
    visited: &mut std::collections::HashSet<String>,
    depth: usize,
    recursive_headers: &mut Vec<String>, 
    ) {
    let path = path.replace('/', "\\");

    if !visited.insert(path.clone()) {
        return;
    }
    let p = std::path::Path::new(&path);
    let search_dirs = include_dirs_with_parent(p, include_dirs);

    let items = loop {
        let (items, writer) = extract_include_entry(&path, resolve_dependency_includes);
        let writer = writer.or_else(|| {
            prepare_queue.and_then(|queue| queue.take_writer_by_path(&path))
        });
        if let Some(writer) = writer {
            if !extract_sourcefile_and_sync_dependency(
                &path, writer, search_dirs.as_ref(), solution, project, cache,
                resolve_dependency_includes, stream,
            ) {
                visited.remove(&path);
                return;
            }
            break items;
        }
        
        drop(items.blocking_read());
        if extract_include_entry_is_current(&path, resolve_dependency_includes, &items) {
            break items;
        }
    };

    if !enqueue_dependency_source(&path, solution, project, cache, stream) {
        visited.remove(&path);
        return;
    }

    if depth > 0 {
        recursive_headers.push(format!("{:indent$}{}", "", path, indent = (depth - 1) * 2));
    }

    let items = items.blocking_read();
    if let Some(queue) = prepare_queue {
        prefetch_unconditional_includes(&items, resolve_dependency_includes, Some(cache), queue, visited);
    }

    parser_sourcefile_dependency_visit_extracted(
        &items, defines, search_dirs.as_ref(), |dependency| {
            parser_sourcefile_with_macro_definition_recursive(
                dependency, defines, include_dirs, solution, project, cache,
                resolve_dependency_includes, stream, prepare_queue, visited,
                depth + 1,
                recursive_headers,
            );
        }
    );
}

pub fn parser_sourcefile_with_macro_definition (
    path: &str,
    defines: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
    include_dirs: &std::vec::Vec<std::path::PathBuf>,
    solution: &std::sync::Arc<String>,
    project: &std::sync::Arc<String>,
    cache: &std::sync::Arc<dashmap::DashSet<String>>,
    resolve_dependency_includes: &ExtractedDependencyCode,
    stream: &tokio::sync::mpsc::Sender<crate::communicate::packager::ArchiveArgs>,
    prepare_queue: Option<&PrepareQueue>,
    visited: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    let mut recursive_headers = Vec::new();
    parser_sourcefile_with_macro_definition_recursive(
        path, defines, include_dirs, solution, project, cache,
        resolve_dependency_includes, stream, prepare_queue, visited,
        0,
        &mut recursive_headers,
    );
    recursive_headers
}

#[cfg(test)]
mod tests {

use super::*;
    #[test]
    fn test_nested_includes_update_macros_before_later_lines() {
        let files = std::collections::HashMap::from([
            ("a.h", extract_sourcefile_includes(b"#define FROM_A\n#include \"c.h\"\n", &Vec::new())),
            ("c.h", extract_sourcefile_includes(b"#define FROM_C\n", &Vec::new())),
            ("b.h", extract_sourcefile_includes(b"#ifdef FROM_A\n#include \"nested.h\"\n#endif\n", &Vec::new())),
        ]);
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut visited = Vec::new();

        fn scan(
            source: &[ExtractIncludeItem],
            files: &std::collections::HashMap<&str, Vec<ExtractIncludeItem>>,
            defines: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
            visited: &mut Vec<String>,
        ) {
            parser_sourcefile_dependency_visit_extracted(source, defines, &Vec::new(), |path| {
                visited.push(path.to_string());
                if let Some(header) = files.get(path) {
                    scan(header, files, defines, visited);
                }
            });
        }

        let source = extract_sourcefile_includes(
            b"#include \"a.h\"\n#ifdef FROM_C\n#include \"after.h\"\n#endif\n#include \"b.h\"\n",
            &Vec::new(),
        );
        scan(
            &source, &files, &defines, &mut visited,
        );
        assert_eq!(visited, ["a.h", "c.h", "after.h", "b.h", "nested.h"]);
    }

    #[test]
    fn test_extracted_source_keeps_macro_context_separate() {
        let content = b"/*\n#include \"ignored.h\"\n*/\n#ifdef FEATURE\n#include \"enabled.h\"\n#else\n#include \"disabled.h\"\n#endif\n";
        let include_dirs = Vec::new();
        let source = extract_sourcefile_includes(content, &include_dirs);
        let off = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let on = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([
            ("FEATURE".to_string(), None),
        ])));
        let mut off_includes = Vec::new();
        let mut on_includes = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&source, &off, &include_dirs, |path| off_includes.push(path.to_string()));
        parser_sourcefile_dependency_visit_extracted(&source, &on, &include_dirs, |path| on_includes.push(path.to_string()));
        assert_eq!(off_includes, ["disabled.h"]);
        assert_eq!(on_includes, ["enabled.h"]);
        let raw_off = parser_sourcefile_dependency(
            content,
            std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            &include_dirs,
        );
        assert_eq!(off_includes, raw_off);
    }

    #[test]
    fn test_extracted_source_replays_macro_include_and_undef_in_order() {
        let content = b"#define HEADER first.h\n#include HEADER\n#undef HEADER\n#ifdef HEADER\n#include \"wrong.h\"\n#else\n#include \"second.h\"\n#endif\n";
        let include_dirs = Vec::new();
        let source = extract_sourcefile_includes(content, &include_dirs);
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut includes = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&source, &defines, &include_dirs, |path| includes.push(path.to_string()));
        assert_eq!(includes, ["first.h", "second.h"]);
        assert!(!defines.read().unwrap().contains_key("HEADER"));

        let raw = parser_sourcefile_dependency(
            content,
            std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            &include_dirs,
        );
        assert_eq!(includes, raw);
    }

    #[test]
    fn test_extracted_source_fixed_includes_keep_order() {
        let source = extract_sourcefile_includes(
            b"#include \"first.h\"\n#include \"second.h\"\n",
            &Vec::new(),
        );
        assert!(source.iter().all(|item| matches!(item, ExtractIncludeItem::Includes(_))));
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut includes = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&source, &defines, &Vec::new(), |path| includes.push(path.to_string()));
        assert_eq!(includes, ["first.h", "second.h"]);
    }

    #[test]
    fn test_strip_discards_condition_groups_without_dependency_effects() {
        let content = b"#if UNUSED\n#pragma message(ignored)\n#if NESTED\n#pragma warning(disable: 1)\n#endif\n#else\n#pragma message(also_ignored)\n#endif\n#include \"needed.h\"\n";
        let items = extract_sourcefile_includes(content, &Vec::new());
        assert_eq!(items, [ExtractIncludeItem::Includes("needed.h".to_string())]);
    }

    #[test]
    fn test_extract_keeps_nested_conditions_around_includes() {
        let content = b"#if OUTER\n#if UNUSED\n#pragma message(ignored)\n#endif\n#if INNER\n#include \"nested.h\"\n#endif\n#else\n#include \"other.h\"\n#endif\n";
        let items = extract_sourcefile_includes(content, &Vec::new());
        assert!(!items.iter().any(|item| matches!(item, ExtractIncludeItem::Define(line) if line.contains("UNUSED"))));
        let off = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let on = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([
            ("OUTER".to_string(), Some("1".to_string())),
            ("INNER".to_string(), Some("1".to_string())),
        ])));
        let mut off_includes = Vec::new();
        let mut on_includes = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&items, &off, &Vec::new(), |path| off_includes.push(path.to_string()));
        parser_sourcefile_dependency_visit_extracted(&items, &on, &Vec::new(), |path| on_includes.push(path.to_string()));
        assert_eq!(off_includes, ["other.h"]);
        assert_eq!(on_includes, ["nested.h"]);
    }

    #[test]
    fn test_extract_stores_each_conditional_group_as_one_item() {
        let content = b"#define FEATURE 1\n#if FEATURE\n#include \"set.h\"\n#if FROM_SET\n#include \"next.h\"\n#endif\n#else\n#include \"other.h\"\n#endif\n#include \"tail.h\"\n";
        let items = extract_sourcefile_includes(content, &Vec::new());
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ExtractIncludeItem::Define("#define FEATURE 1".to_string()));
        assert!(matches!(&items[1], ExtractIncludeItem::Define(group)
            if group.starts_with("#if FEATURE\n") && group.ends_with("#endif") && group.contains("#if FROM_SET\n")));
        assert_eq!(items[2], ExtractIncludeItem::Includes("tail.h".to_string()));

        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut included = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&items, &defines, &Vec::new(), |path| {
            included.push(path.to_string());
            if path == "set.h" {
                defines.write().unwrap().insert("FROM_SET".to_string(), Some("1".to_string()));
            }
        });
        assert_eq!(included, ["set.h", "next.h", "tail.h"]);
    }

    #[test]
    fn test_extract_conditions_select_one_branch_for_each_macro_state() {
        let items = extract_sourcefile_includes(
            b"#if A && !B || C\n#include \"first.h\"\n#elif D\n#include \"second.h\"\n#else\n#include \"fallback.h\"\n#endif\n#include \"after.h\"\n",
            &Vec::new(),
        );
        for (macros, expected) in [
            ([("A", "1"), ("D", "1")].as_slice(), ["first.h", "after.h"]),
            ([("A", "1"), ("B", "1"), ("D", "1")].as_slice(), ["second.h", "after.h"]),
            ([].as_slice(), ["fallback.h", "after.h"]),
        ] {
            let defines = std::sync::Arc::new(std::sync::RwLock::new(macros.iter().map(|(name, value)| {
                (name.to_string(), Some(value.to_string()))
            }).collect()));
            let mut includes = Vec::new();
            parser_sourcefile_dependency_visit_extracted(&items, &defines, &Vec::new(), |path| includes.push(path.to_string()));
            assert_eq!(includes, expected);
        }
    }

    #[test]
    fn test_extract_keeps_macro_changes_for_later_files() {
        let content = b"#if FLAG\n#define SHARED 1\n#else\n#undef SHARED\n#endif\n";
        let items = extract_sourcefile_includes(content, &Vec::new());
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([
            ("FLAG".to_string(), Some("1".to_string())),
        ])));
        parser_sourcefile_dependency_visit_extracted(&items, &defines, &Vec::new(), |_| {});
        assert_eq!(defines.read().unwrap().get("SHARED"), Some(&Some("1".to_string())));
    }

    #[test]
    fn test_extract_source_ignores_incomplete_include_in_inactive_branch() {
        let content = b"#if 0\n#include \"\n#endif\n#include \"valid.h\"\n";
        let include_dirs = Vec::new();
        let items = extract_sourcefile_includes(content, &include_dirs);
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut includes = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&items, &defines, &include_dirs, |path| includes.push(path.to_string()));
        assert_eq!(includes, ["valid.h"]);
    }

    #[test]
    fn test_extract_source_ignores_quotes_in_include_comments() {
        let content = b"#include \"valid.h\" // \"comment\"\n#include <system.h> // >\n";
        let include_dirs = Vec::new();
        let items = extract_sourcefile_includes(content, &include_dirs);
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut includes = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&items, &defines, &include_dirs, |path| includes.push(path.to_string()));
        assert_eq!(includes, ["valid.h", "system.h"]);
    }

    #[test]
    fn test_extract_top_level_include_guard_exposes_unconditional_includes() {
        let content = b"#ifndef MY_GUARD_H\n#define MY_GUARD_H\n#include \"a.h\"\n#ifdef FLAG\n#include \"b.h\"\n#endif\n#include \"c.h\"\n#endif // MY_GUARD_H\n";
        let include_dirs = Vec::new();
        let items = extract_sourcefile_includes(content, &include_dirs);
        assert_eq!(items, vec![
            ExtractIncludeItem::Includes("a.h".to_string()),
            ExtractIncludeItem::Define("#ifdef FLAG\n#include \"b.h\"\n#endif".to_string()),
            ExtractIncludeItem::Includes("c.h".to_string()),
        ]);

        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let mut visited = Vec::new();
        parser_sourcefile_dependency_visit_extracted(&items, &defines, &include_dirs, |p| visited.push(p.to_string()));
        assert_eq!(visited, ["a.h", "c.h"]);
        assert!(!defines.read().unwrap().contains_key("MY_GUARD_H"));
    }

    #[test]
    fn test_utf8_bom_on_first_line_is_parsed() {
        let content = b"\xEF\xBB\xBF#include \"bom_header.h\"\n#include \"second.h\"\n";
        let include_dirs = Vec::new();
        let items = extract_sourcefile_includes(content, &include_dirs);
        assert_eq!(items, vec![
            ExtractIncludeItem::Includes("bom_header.h".to_string()),
            ExtractIncludeItem::Includes("second.h".to_string()),
        ]);
        let raw = parser_sourcefile_dependency(
            content,
            std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            &include_dirs,
        );
        assert_eq!(raw, ["bom_header.h", "second.h"]);
    }

    #[test]
    fn test_parser_sourcefile_dependency() {
        let content = b"// This is a comment\n#include <stdio.h>\n#include \"myheader.h\"\nint main() { return 0; }";
        let dependencies = parser_sourcefile_dependency(content, std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.contains(&"<stdio.h>".to_string()), true);
        assert_eq!(dependencies.contains(&"\"myheader.h\"".to_string()), true);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_condition() {
        let content = r#"
        // This is a comment
        #include <stdio.h>
        #include "myheader.h"
        #ifdef YES
        #include "conditional.h"
        #endif
        int main() { 
            return 0; 
        }"#;
        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies.contains(&"<stdio.h>".to_string()), true);
        assert_eq!(dependencies.contains(&"\"myheader.h\"".to_string()), true);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("YES".to_string(), Some("1".to_string()))]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 3);
        assert_eq!(dependencies.contains(&"<stdio.h>".to_string()), true);
        assert_eq!(dependencies.contains(&"\"myheader.h\"".to_string()), true);
        assert_eq!(dependencies.contains(&"\"conditional.h\"".to_string()), true);

        let content = r#"
        #include <stdio.h>
        #include "myheader.h"
        \n
        #define YES
        #ifdef YES
        #include "conditional.h"
        #endif
        \n
        #undef YES
        \n
        #ifdef YES
        #include "conditional_second.h"
        #endif
        \n
        int main() {
            return 0;
        }"#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 3);
        assert_eq!(dependencies.contains(&"<stdio.h>".to_string()), true);
        assert_eq!(dependencies.contains(&"\"myheader.h\"".to_string()), true);
        assert_eq!(dependencies.contains(&"\"conditional.h\"".to_string()), true);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_ifelse() {

        let content = r#"
        #if defined(Win) && !defined(Linux)
        #  include "tps/dirent.h"
        #  include "tps/dirent.c"
        #else
        #  include <sys/types.h>
        #  include <dirent.h>
        #  include <unistd.h>
        #endif"#;        

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 3);
        assert_eq!(dependencies.contains(&"<sys/types.h>".to_string()), true);
        assert_eq!(dependencies.contains(&"<dirent.h>".to_string()), true);
        assert_eq!(dependencies.contains(&"<unistd.h>".to_string()), true);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("Win".to_string(), None)]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies.contains(&"\"tps/dirent.h\"".to_string()), true);
        assert_eq!(dependencies.contains(&"\"tps/dirent.c\"".to_string()), true);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_elif() {
        let content = r#"
        #if defined(HAVE_STDINT_H)
        #  include <stdint.h>
        #elif defined(HAVE_INTTYPES_H)
        #  include <inttypes.h>
        #elif defined(HAVE_SYS_TYPES_H)
        #  include <sys/types.h>
        #endif"#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 0);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("HAVE_STDINT_H".to_string(), None)]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies.contains(&"<stdint.h>".to_string()), true);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("HAVE_INTTYPES_H".to_string(), None)]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies.contains(&"<inttypes.h>".to_string()), true);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("HAVE_SYS_TYPES_H".to_string(), None)]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies.contains(&"<sys/types.h>".to_string()), true);
    }
    #[test]
    fn test_parser_sourcefile_dependency_with_multi_level_nesting() {

        let content = r#"
        #if defined(LINUX) || defined(ANDROID)
        # if defined(__LP64__)
        #  include <time.h>
        # else
        #  include <time64.h>
        # endif
        #endif"#;        

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new()); 
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 0);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("LINUX".to_string(), None)]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies.contains(&"<time64.h>".to_string()), true);

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("LINUX".to_string(), None), ("__LP64__".to_string(), None)]))), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies.contains(&"<time.h>".to_string()), true);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_comment() {

        let content = r#"
        #include <stdio.h> /*comment*/
        #include "myheader.h" /*comment*/"#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies.contains(&"<stdio.h>".to_string()), true);

        let content = r#"
        #include <stdio.h> //comment
        #include "myheader.h" //comment"#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies.contains(&"<stdio.h>".to_string()), true);

        let content = r#"
        //#include <stdio.h>
        #include "myheader.h""#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);

        let content = r#"
        /*
        #include <stdio.h>
        */
        #include "myheader.h""#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);

        let content = r#"
        /*#include <stdio.h>*/
        #include "myheader.h""#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);

        let content = r#"
        /*
        //#include <stdio.h>
        */
        #include "myheader.h""#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);

        let content = r#"
        /*
            /*
        #include <stdio.h>
            */
        */
        #include "myheader.h""#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_relative_path() {
        
        let content = r#"
        #include "./types.h"
        #include "myheader.h""#;

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 2);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_define_value() {
        let content = r#"
        #include <stdio.h>
        #include "myheader.h"
        #define YES OKK
        #if YES == OK
            #include "conditional.h"
        #elif YES == OKK
            #include "conditional_second.h"
        #else
        \n
        #endif

        #endif
        int main() {
            return 0; 
        }"#;
        let dependencies = parser_sourcefile_dependency(content.as_bytes(), std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())), &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 3);

        let content = r#"
            #ifdef __STDC_VERSION__
            #  ifndef STDC
            #    define STDC
            #  endif
            #  if __STDC_VERSION__ >= 199901L
            #    ifndef STDC99
            #      define STDC99
            #    endif
            #  endif
            #endif

            #if defined(GEN) || !defined(STDC)
            #else
            #  include "trees.h"
            #endif
        "#;

        let compiler_predefined: &[(&str, Option<&str>)] = &[
            ("_WIN32", None), ("_WIN64", None),
            ("_M_X64", Some("100")), ("_M_AMD64", Some("100")),
            ("_MSC_VER", Some("1944")), ("_MSC_FULL_VER", Some("194435219")),
            ("_MSC_BUILD", None), ("_MSVC_LANG", Some("201402L")),
            ("__cplusplus", Some("201402L")), ("_MT", None), ("_DEBUG", None),
            ("_INTEGRAL_MAX_BITS", Some("64")),
            ("__STDC_HOSTED__", None), ("__STDC_VERSION__", Some("201710L")),
            ("_WCHAR_T_DEFINED", None), ("_NATIVE_WCHAR_T_DEFINED", None),
        ];
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));

        for &(k, v) in compiler_predefined {
            defines.write().unwrap().insert(k.to_string(), v.map(|s| s.to_string()));
        }
        let dependencies = parser_sourcefile_dependency(content.as_bytes(), defines, &std::vec::Vec::new());
        println!("{:?}", &dependencies);
        assert_eq!(dependencies.len(), 1);
    }

    #[test]
    fn test_parser_sourcefile_dependency_with_include_define() {
        let content = r#"
            #include SOME_HEADER_FILE
        "#;

        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::from([("SOME_HEADER_FILE".to_string(), Some("myheader.h".to_string()))])));

        let include_dirs = &std::vec::Vec::new();

        let dependencies = parser_sourcefile_dependency(content.as_bytes(), defines, include_dirs);
        println!("{:?}", &dependencies);
    }
    //TODO: 
    //#if __has_include(<version>)
    //#  include <version>
    //#endif

    #[test]
    fn query_include_dir_test() {
        let now = std::time::Instant::now();
        let _ = query_include_dir(&vec![std::path::PathBuf::from("D:\\Webex")]);
        println!("query_include_dir_test: {:?}", now.elapsed());
    }

    #[test]
    fn query_include_dir_ignore_test() {
        let now = std::time::Instant::now();
        query_include_dir_ignore(&std::path::PathBuf::from("D:\\WorkSpace\\OpenSource\\ZLMediaKit"));
        println!("query_include_dir_ignore_test: {:?}", now.elapsed());
    }
    /*
    #ifndef KDEITEMMODELS_EXPORT_H
    #define KDEITEMMODELS_EXPORT_H

    #include <QtCore/qglobal.h>

    #ifdef KITEMMODELS_STATICLIB
    #  undef KITEMMMODELS_SHAREDLIB
    #  define KITEMMODELS_EXPORT
    #else
    #  ifdef MAKE_KITEMMODELS_LIB
    #    define KITEMMODELS_EXPORT Q_DECL_EXPORT
    #  else
    #    define KITEMMODELS_EXPORT Q_DECL_IMPORT
    #  endif
    #endif

    #endif
    */

    #[test]
    fn test_prefetch_only_unconditional_includes_and_drops_full_queue_entry() {
        let includes: ExtractedDependencyCode = std::sync::Arc::new(dashmap::DashMap::new());
        let items = vec![
            ExtractIncludeItem::Includes("first.h".to_string()),
            ExtractIncludeItem::Define("#if FEATURE\n#include \"conditional.h\"\n#endif".to_string()),
            ExtractIncludeItem::Includes("second.h".to_string()),
        ];
        let queue = PrepareQueue::with_capacity(1);
        prefetch_unconditional_includes(
            &items, &includes, None, &queue, &std::collections::HashSet::new(),
        );
        let request = queue.try_pop().unwrap();
        assert_eq!(request.path, "first.h");
        assert!(includes.contains_key("first.h"));
        assert!(!includes.contains_key("conditional.h"));
        assert!(!includes.contains_key("second.h"));
        drop(request);
    }

    #[test]
    fn test_include_dirs_with_parent_prefers_current_header_directory() {
        let parent = std::path::PathBuf::from("project").join("sub");
        let path = parent.join("header.h");
        let fixed = vec![std::path::PathBuf::from("project"), parent.clone(), std::path::PathBuf::from("sdk")];
        let search = include_dirs_with_parent(&path, &fixed);
        assert_eq!(search.as_ref(), &vec![parent.clone(), std::path::PathBuf::from("project"), std::path::PathBuf::from("sdk")]);

        let already_first = vec![parent, std::path::PathBuf::from("sdk")];
        assert!(matches!(include_dirs_with_parent(&path, &already_first), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn test_strip_include_cache_waits_for_one_initializer() {
        let cache: ExtractedDependencyCode = std::sync::Arc::new(dashmap::DashMap::new());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let initializations = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut threads = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let barrier = barrier.clone();
            let initializations = initializations.clone();
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                let (items, writer) = extract_include_entry("shared.h", &cache);
                if let Some(mut writer) = writer {
                    initializations.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    *writer = extract_sourcefile_includes(b"#define SHARED\n", &Vec::new());
                }
                let items = items.blocking_read();
                assert_eq!(items.as_slice(), &[ExtractIncludeItem::Define("#define SHARED".to_string())]);
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(initializations.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_waiter_rejects_removed_cache_entry_after_read_failure() {
        let dependency: ExtractedDependencyCode = std::sync::Arc::new(dashmap::DashMap::new());
        let (_stale, writer) = extract_include_entry("missing.h", &dependency);
        let writer = writer.unwrap();
        let (ready, received) = std::sync::mpsc::channel();
        let cache_for_reader = dependency.clone();
        let reader = std::thread::spawn(move || {
            let (items, pending_writer) = extract_include_entry("missing.h", &cache_for_reader);
            assert!(pending_writer.is_none());
            ready.send(()).unwrap();
            drop(items.blocking_read());
            extract_include_entry_is_current("missing.h", &cache_for_reader, &items)
        });
        received.recv().unwrap();
        dependency.remove("missing.h");
        drop(writer);
        assert!(!reader.join().unwrap());
    }

    #[test]
    fn test_failed_source_send_does_not_mark_file_as_sent() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let source = std::env::temp_dir().join(format!("crew-send-{}-{unique}.cpp", std::process::id()));
        std::fs::write(&source, b"#define PRESENT\n").unwrap();
        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let sent = std::sync::Arc::new(dashmap::DashSet::new());
        let dependency: ExtractedDependencyCode = std::sync::Arc::new(dashmap::DashMap::new());
        let (stream, receiver) = tokio::sync::mpsc::channel(1);
        drop(receiver);
        parser_sourcefile_with_macro_definition(
            source.to_str().unwrap(), &defines, &Vec::new(),
            &std::sync::Arc::new(String::new()), &std::sync::Arc::new(String::new()),
            &sent, &dependency, &stream, None,
            &mut std::collections::HashSet::new(),
        );
        assert!(sent.is_empty());
        assert!(defines.read().unwrap().contains_key("PRESENT"));
        std::fs::remove_file(source).unwrap();
    }

    #[test]
    fn test_pragma_once_skips_repeated_header_after_macro_changes() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("crew-pragma-once-{}-{unique}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let source = dir.join("source.cpp");
        let header = dir.join("once.h");
        let unexpected = dir.join("unexpected.h");
        std::fs::write(&unexpected, b"").unwrap();
        std::fs::write(&header, format!(
            "#pragma once\n#ifdef INCLUDED\n#include \"{}\"\n#endif\n#define INCLUDED\n",
            unexpected.display(),
        )).unwrap();
        std::fs::write(&source, format!(
            "#include \"{}\"\n#include \"{}\"\n",
            header.display(), header.display(),
        )).unwrap();

        let defines = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let sent = std::sync::Arc::new(dashmap::DashSet::new());
        let dependency: ExtractedDependencyCode = std::sync::Arc::new(dashmap::DashMap::new());
        let (stream, mut receiver) = tokio::sync::mpsc::channel(8);
        parser_sourcefile_with_macro_definition(
            source.to_str().unwrap(), &defines, &Vec::new(),
            &std::sync::Arc::new(String::new()), &std::sync::Arc::new(String::new()),
            &sent, &dependency, &stream, None,
            &mut std::collections::HashSet::new(),
        );
        drop(stream);
        let mut paths = Vec::new();
        while let Ok(archive) = receiver.try_recv() { paths.push(archive.path); }
        assert_eq!(paths.len(), 2);
        assert!(!paths.contains(&unexpected.to_string_lossy().replace('/', "\\")));
        assert!(defines.read().unwrap().contains_key("INCLUDED"));

        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(header).unwrap();
        std::fs::remove_file(unexpected).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

}
