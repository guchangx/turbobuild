
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
    let n = b.iter().position(|&c| c != b' ' && c != b'\t').unwrap_or(b.len());
    let m = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\n' && c != b'\r');

    match m {
        Some(idx) if idx >= n => &b[n..=idx],
        _ => &b[0..0],
    }
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

fn parse_define(s: &str, defines: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>) {
    let s = s.trim();
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
    defines.write().unwrap().insert(name.to_string(), val.map(|s| s.to_string()));
}

pub fn parser_sourcefile_dependency(content: &[u8], defines: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Option<String>>>>,
        include_dirs: &std::vec::Vec::<std::path::PathBuf>
    ) -> std::vec::Vec<String> {

    #[derive(Clone, PartialEq)]
    enum Branch { Active, SkipToElse, SkipToEndif }
    let mut condition_stack: Vec<Branch> = Vec::new();
    let mut inactive_depth: i32 = 0;

    let mut comment_stack_depth = 0;

    let mut includes = std::vec::Vec::new();

    for line in content.split(|&b| b == b'\n').take(180) {

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
            b"ifdef" => {
                let name = std::str::from_utf8(rest).unwrap();
                if inactive_depth > 0 {
                    condition_stack.push(Branch::SkipToEndif);
                    inactive_depth += 1;
                } else if defines.read().unwrap().contains_key(name) {
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
                } else if !defines.read().unwrap().contains_key(name) {
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
                    let val = expr_eval::eval(&expanded, &defines.read().unwrap());
                    if val != 0 {
                        condition_stack.push(Branch::Active);
                    } else {
                        condition_stack.push(Branch::SkipToElse);
                        inactive_depth += 1;
                    }
                }
            }
            b"elif" => {
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
                        if expr_eval::eval(&expanded, &defines.read().unwrap()) != 0 {
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
            b"pragma" if inactive_depth == 0 => {

            }
            b"define" if inactive_depth == 0 => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    parse_define(s, defines.clone());
                }
            }
            b"undef" if inactive_depth == 0 => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    defines.write().unwrap().remove(s.split_whitespace().next().unwrap_or(""));
                }
            }
            b"include" if inactive_depth == 0 => {
                if let Ok(s) = std::str::from_utf8(rest) {
                    let resolved = if s.starts_with('"') || s.starts_with('<') {
                        let pos = s.rfind('"').unwrap_or(s.rfind('>').unwrap_or(s.len())) + 1;
                        check_local_include_dir_and_files_2(include_dirs, &s[1..pos-1])
                    } else {
                        let macro_name = s.split(|c: char| c.is_whitespace() || c == '/')
                            .next().unwrap_or("").trim();
                        if let Some(Some(expanded)) = defines.read().unwrap().get(macro_name) {
                            check_local_include_dir_and_files_2(include_dirs, expanded)
                        } else {
                            None
                        }
                    };

                    if let Some(p) = resolved {
                        includes.push(p.to_string_lossy().to_string());
                    }
                }
            }
            _ => {}
        }
    }
    return includes;
}

fn check_local_include_dir_and_files(include_dir_files: &std::vec::Vec::<(String, std::sync::Arc<std::collections::HashSet<String>>)>, dep: &str) -> Option<std::path::PathBuf> {

    for (dir, files) in include_dir_files.iter() {
        if dep.starts_with("..") {
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
    
    if dep.starts_with("..") {
        let reader = unsafe { crate::compiler::model::WALK_FS_NODE.as_ref().unwrap() };
        for dir in include_dirs {
            let p = tools::utils::normalize_lexical(dir.to_owned().into_os_string().into_string().unwrap() + "\\" + dep);

            if reader.exists(&p) {
                return Some(p);
            }
            else {
                if p.exists() {
                    return Some(p);
                }
            }

            if std::path::PathBuf::from(&p).exists() {
                return Some(std::path::PathBuf::from(p));
            }
        }
    }
    else {
        let reader = unsafe { crate::compiler::model::WALK_FS_NODE.as_ref().unwrap() };
        for dir in include_dirs {
            let p = dir.join(dep);
            if reader.exists(&p) {
                return Some(p);
            }
            else {
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    return None;
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
            if path.is_file() {
                if path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()).filter(|ext| ext == "h" || ext == "hpp" 
                    || ext == "hh" || ext == "hxx" || ext == "h++" || ext == "hm" || ext == "inl").is_some() {
                    if let Ok(p) = path.strip_prefix(&start) {
                        set.insert(p.to_string_lossy().to_string());
                    }
                }
            }
            else if path.is_dir() {
                query_include_dir_recursive(path, &start, set);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
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
}
