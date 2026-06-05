
fn trim(b: &[u8]) -> &[u8] {
    let n = b.iter().position(|&c| c != b' ' && c != b'\t').unwrap_or(b.len());
    &b[n..]
}

fn split_directive(b: &[u8]) -> (&[u8], &[u8]) {
    let end = b.iter().position(|&c| c == b' ' || c == b'\t').unwrap_or(b.len());
    (&b[..end], if end < b.len() { trim(&b[end..]) } else { b"" })
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
                b'/' => out.push(Tok::Slash), b'%' => out.push(Tok::Percent),
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

fn parse_define(s: &str, defines: &mut std::collections::HashMap<String, Option<String>>) {
    let s = s.trim();
    let (name, val) = if let Some(paren) = s.find('(') {
        let space = s.find(char::is_whitespace).unwrap_or(s.len());
        if paren < space { (&s[..paren], None) }
        else { let v = s[space..].trim(); (&s[..space], if v.is_empty() { None } else { Some(v) }) }
    } else {
        let space = s.find(char::is_whitespace).unwrap_or(s.len());
        let v = s[space..].trim();
        (&s[..space], if v.is_empty() { None } else { Some(v) })
    };
    defines.insert(name.to_string(), val.map(|s| s.to_string()));
}

fn resolve_include(
    rest: &str,
    current_file: &std::path::Path,
    user_includes: &[std::path::PathBuf],
    sys_includes: &[std::path::PathBuf],
) -> Option<std::path::PathBuf> {
    if rest.starts_with('"') {
        if let Some(end) = rest[1..].find('"') {
            let header = &rest[1..end + 1];
            if let Some(parent) = current_file.parent() {
                let c = parent.join(header);
                if c.is_file() { return Some(c); }
            }
            for inc in user_includes.iter().chain(sys_includes.iter()) {
                let c = inc.join(header);
                if c.is_file() { return Some(c); }
            }
        }
    } else if rest.starts_with('<') {
        if let Some(end) = rest[1..].find('>') {
            let header = &rest[1..end + 1];
            for inc in user_includes.iter().chain(sys_includes.iter()) {
                let c = inc.join(header);
                if c.is_file() { return Some(c); }
            }
        }
    }
    None
}

fn resolve_include_cached(
    rest: &str,
    current_file: &std::path::Path,
    user_includes: &[std::path::PathBuf],
    sys_includes: &[std::path::PathBuf],
    cache: &dashmap::DashMap<String, Option<std::path::PathBuf>>,
) -> Option<std::path::PathBuf> {
    let key = if rest.starts_with('<') {
        rest.to_string()
    } else {
        let parent = current_file.parent()
            .map(|p| p.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        format!("{}:{}", parent, rest)
    };
    if let Some(v) = cache.get(&key) { return v.clone(); }
    let result = resolve_include(rest, current_file, user_includes, sys_includes);
    cache.insert(key, result.clone());
    result
}

fn get_canonical(
    path: &std::path::Path,
    cache: &dashmap::DashMap<String, Option<std::path::PathBuf>>,
) -> Option<std::path::PathBuf> {
    let key = path.to_string_lossy().into_owned();
    if let Some(v) = cache.get(&key) { return v.clone(); }
    let result = std::fs::canonicalize(path).ok();
    cache.insert(key, result.clone());
    result
}

fn normalize_path(p: &std::path::Path) -> String {
    p.to_string_lossy().to_lowercase().replace("\\\\?\\", "")
}

// Replace C/C++ line comments (//) and block comments (/* */) with spaces,
// preserving newlines so line numbers stay intact.
//
fn strip_comments(content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len());
    let mut i = 0;
    let mut in_block = false;
    while i < content.len() {
        if in_block {
            if i + 1 < content.len() && content[i] == b'*' && content[i+1] == b'/' {
                out.push(b' '); out.push(b' ');
                i += 2;
                in_block = false;
            } else if content[i] == b'\n' {
                out.push(b'\n'); i += 1;
            } else {
                out.push(b' '); i += 1;
            }
        } else if i + 1 < content.len() && content[i] == b'/' && content[i+1] == b'*' {
            out.push(b' '); out.push(b' ');
            i += 2;
            in_block = true;
        } else if i + 1 < content.len() && content[i] == b'/' && content[i+1] == b'/' {
            out.push(b' '); out.push(b' ');
            i += 2;
            while i < content.len() && content[i] != b'\n' {
                out.push(b' '); i += 1;
            }
        } else {
            out.push(content[i]); i += 1;
        }
    }
    out
}

fn read_cached(
    path: &std::path::Path,
    key: &str,
    cache: &dashmap::DashMap<String, Vec<u8>>,
) -> Vec<u8> {
    if let Some(v) = cache.get(key) { return v.clone(); }
    let raw = std::fs::read(path).unwrap_or_default();
    // Store comment-stripped content so scan_content needn't strip again.
    let stripped = strip_comments(&raw);
    cache.insert(key.to_string(), stripped.clone());
    stripped
}

fn scan_file_inner(
    path: &std::path::Path,
    is_main: bool,
    user_includes: &[std::path::PathBuf],
    sys_includes: &[std::path::PathBuf],
    defines: &mut std::collections::HashMap<String, Option<String>>,
    pragma_once: &mut std::collections::HashSet<String>,
    visited: &mut std::collections::HashSet<String>,
    result: &mut std::collections::HashSet<String>,
    file_cache: &dashmap::DashMap<String, Vec<u8>>,
    canon_cache: &dashmap::DashMap<String, Option<std::path::PathBuf>>,
    resolve_cache: &dashmap::DashMap<String, Option<std::path::PathBuf>>,
) {
    let cur_canon = match get_canonical(path, canon_cache) {
        Some(c) => c,
        None => return,
    };
    let cur_str = normalize_path(&cur_canon);
    if pragma_once.contains(&cur_str) || visited.contains(&cur_str) { return; }
    visited.insert(cur_str.clone());
    if !is_main {
        result.insert(cur_str.clone());
    }
    let content = read_cached(&cur_canon, &cur_str, file_cache);
    if content.is_empty() { return; }
    // continue parse dependencies
}

fn parser_sourcefile_dependency(content: &[u8], defines: &mut std::collections::HashMap<String, Option<String>>) -> std::collections::HashSet<String> {

    #[derive(Clone, PartialEq)]
    enum Branch { Active, SkipToElse, SkipToEndif }
    let mut condition_stack: Vec<Branch> = Vec::new();
    let mut inactive_depth: i32 = 0;

    let mut includes = std::collections::HashSet::new();

    for line in content.split(|&b| b == b'\n').take(200) {

        if line.is_empty() { continue; }
        let trimmed = trim(line);
        if trimmed[0] != b'#' { continue; }

        let after = trim(&trimmed[1..]);
        let (dir, rest) = split_directive(after);

        match dir {
            b"ifdef" => {
                let name = std::str::from_utf8(rest).unwrap();
                if inactive_depth > 0 {
                    condition_stack.push(Branch::SkipToEndif);
                    inactive_depth += 1;
                } else if defines.contains_key(name) {
                    condition_stack.push(Branch::Active);
                } else {
                    condition_stack.push(Branch::SkipToElse);
                    inactive_depth += 1;
                }
            }
            b"ifndef" => {
                let name = std::str::from_utf8(rest).unwrap();
                if inactive_depth > 0 {
                    condition_stack.push(Branch::SkipToEndif);
                    inactive_depth += 1;
                } else if !defines.contains_key(name) {
                    condition_stack.push(Branch::Active);
                } else {
                    condition_stack.push(Branch::SkipToElse);
                    inactive_depth += 1;
                }
            }
            b"if" => {
                if inactive_depth > 0 {
                    condition_stack.push(Branch::SkipToEndif);
                    inactive_depth += 1;
                } else {
                    let expr = std::str::from_utf8(rest).unwrap_or("");
                    let expanded = expand_has_include(expr, std::path::Path::new(""), &[], &[]);
                    let val = expr_eval::eval(&expanded, defines);
                    if val != 0 {
                        condition_stack.push(Branch::Active);
                    } else {
                        condition_stack.push(Branch::SkipToElse);
                        inactive_depth += 1;
                    }
                }
            }
            b"elif" => {
                // outer_ok: are all items *except* the top of stack active?
                let outer_ok = match condition_stack.last() {
                    None => true,
                    Some(Branch::Active) => inactive_depth == 0,
                    Some(_) => inactive_depth == 1,
                };
                match condition_stack.last_mut() {
                    Some(b) if *b == Branch::Active => {
                        *b = Branch::SkipToEndif;
                        inactive_depth += 1;
                    }
                    Some(b) if *b == Branch::SkipToElse && outer_ok => {
                        let expr = std::str::from_utf8(rest).unwrap_or("");
                        let expanded = expand_has_include(expr, std::path::Path::new(""), &[], &[]);
                        if expr_eval::eval(&expanded, defines) != 0 {
                            *b = Branch::Active;
                            inactive_depth -= 1;
                        }
                    }
                    _ => {}
                }
            }
            b"else" => {
                let outer_ok = match condition_stack.last() {
                    None => true,
                    Some(Branch::Active) => inactive_depth == 0,
                    Some(_) => inactive_depth == 1,
                };
                match condition_stack.last_mut() {
                    Some(b) if *b == Branch::Active => {
                        *b = Branch::SkipToEndif;
                        inactive_depth += 1;
                    }
                    Some(b) if *b == Branch::SkipToElse && outer_ok => {
                        *b = Branch::Active;
                        inactive_depth -= 1;
                    }
                    _ => {}
                }
            }
            b"endif" => {
                if let Some(old) = condition_stack.pop() {
                    if old != Branch::Active { inactive_depth -= 1; }
                }
            }
                        b"endif" => {
                if let Some(old) = condition_stack.pop() {
                    if old != Branch::Active { inactive_depth -= 1; }
                }
            }
            b"pragma" if inactive_depth == 0 => {

            }
            b"define" if inactive_depth == 0 => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    parse_define(s, defines);
                }
            }
            b"undef" if inactive_depth == 0 => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    defines.remove(s.split_whitespace().next().unwrap_or(""));
                }
            }
            b"include" if inactive_depth == 0 => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    let resolved = if s.starts_with('"') || s.starts_with('<') {
                        let resolve_cache = dashmap::DashMap::new();
                        resolve_include_cached(s, std::path::Path::new(""), &[], &[], &resolve_cache)
                    } else {
                        let macro_name = s.split(|c: char| c.is_whitespace() || c == '/')
                            .next().unwrap_or("").trim();
                        if let Some(Some(expanded)) = defines.get(macro_name) {
                            let expanded = expanded.clone();
                            let resolve_cache = dashmap::DashMap::new();
                            resolve_include_cached(&expanded, std::path::Path::new(""), &[], &[], &resolve_cache)
                        } else {
                            None
                        }
                    };
                    if let Some(p) = resolved {
                        //scan_file_inner(&p, false, &[], &[], defines, &[], &[], &[], &[], &[], &[]);
                    }
                }
            }
            _ => {}
        }
    }
    return includes;
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_parser_sourcefile_dependency() {
        let content = b"// This is a comment\n#include <stdio.h>\n#include \"myheader.h\"\nint main() { return 0; }";
        let dependencies = parser_sourcefile_dependency(content, &mut std::collections::HashMap::new());
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
        let dependencies = parser_sourcefile_dependency(content.as_bytes(), &mut std::collections::HashMap::new());
    }
}
