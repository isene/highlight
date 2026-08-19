//! Lightweight syntax highlighter for common file types.
//! Replaces bat for fast, zero-spawn preview highlighting.
//! Supports multiple color themes via Theme struct.

use crust::style;
use std::sync::Mutex;

#[derive(Clone, Copy)]
pub struct Theme {
    pub keyword: u8,
    pub string: u8,
    pub comment: u8,
    pub number: u8,
    pub typ: u8,
    pub func: u8,
    pub preproc: u8,
    pub punct: u8,
    /// Markdown header colors per level. Levels 4..6 share `md_h_other`
    /// since the visual hierarchy doesn't need 6 distinct shades.
    pub md_h1: u8,
    pub md_h2: u8,
    pub md_h3: u8,
    pub md_h_other: u8,
}

static ACTIVE_THEME: Mutex<Option<Theme>> = Mutex::new(None);

// scribe's `\M` markup toggle. When CONCEAL_SPANS is set, inline colour/font
// span tags are hidden on every line EXCEPT REVEAL_LINE (the cursor's line, so
// the markup stays editable where the cursor sits). CUR_LINE is updated by the
// per-line render loops so color_span_ansi knows which line it's on.
static CONCEAL_SPANS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static REVEAL_LINE: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(usize::MAX);
static CUR_LINE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Toggle span-tag concealment. `reveal_line` (0-based) is always shown with
/// full markup — pass the cursor's line so it stays editable.
pub fn set_span_conceal(on: bool, reveal_line: usize) {
    use std::sync::atomic::Ordering::Relaxed;
    CONCEAL_SPANS.store(on, Relaxed);
    REVEAL_LINE.store(reveal_line, Relaxed);
}

fn conceal_current_line() -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    CONCEAL_SPANS.load(Relaxed) && CUR_LINE.load(Relaxed) != REVEAL_LINE.load(Relaxed)
}

pub fn set_theme(name: &str) {
    if let Ok(mut t) = ACTIVE_THEME.lock() {
        *t = Some(theme_by_name(name));
    }
}

fn theme() -> Theme {
    ACTIVE_THEME.lock().ok()
        .and_then(|t| *t)
        .unwrap_or_else(|| theme_by_name("monokai"))
}

pub fn theme_by_name(name: &str) -> Theme {
    match name {
        "monokai" => Theme {
            keyword: 197, string: 78, comment: 242, number: 141,
            typ: 81, func: 148, preproc: 197, punct: 248,
            md_h1: 51, md_h2: 117, md_h3: 220, md_h_other: 165,
        },
        "solarized" => Theme {
            keyword: 136, string: 64, comment: 245, number: 125,
            typ: 33, func: 166, preproc: 136, punct: 240,
            md_h1: 33, md_h2: 136, md_h3: 166, md_h_other: 125,
        },
        "nord" => Theme {
            keyword: 110, string: 108, comment: 60, number: 176,
            typ: 73, func: 222, preproc: 110, punct: 103,
            md_h1: 110, md_h2: 73, md_h3: 222, md_h_other: 176,
        },
        "dracula" => Theme {
            keyword: 212, string: 84, comment: 61, number: 141,
            typ: 117, func: 228, preproc: 212, punct: 189,
            md_h1: 212, md_h2: 117, md_h3: 228, md_h_other: 141,
        },
        "gruvbox" => Theme {
            keyword: 167, string: 142, comment: 245, number: 175,
            typ: 109, func: 214, preproc: 167, punct: 223,
            md_h1: 142, md_h2: 167, md_h3: 214, md_h_other: 109,
        },
        "plain" => Theme {
            keyword: 252, string: 252, comment: 245, number: 252,
            typ: 252, func: 252, preproc: 252, punct: 245,
            md_h1: 252, md_h2: 252, md_h3: 252, md_h_other: 252,
        },
        _ => theme_by_name("monokai"),
    }
}

pub fn available_themes() -> &'static [&'static str] {
    &["monokai", "solarized", "nord", "dracula", "gruvbox", "plain"]
}

struct Lang {
    line_comment: &'static [&'static str],
    block_start: &'static str,
    block_end: &'static str,
    keywords: &'static [&'static str],
    types: &'static [&'static str],
}

fn lang_for(ext: &str) -> Option<Lang> {
    match ext {
        "rs" => Some(Lang {
            line_comment: &["//"],
            block_start: "/*", block_end: "*/",
            keywords: &["fn","let","mut","pub","use","mod","struct","enum","impl","trait",
                "for","while","loop","if","else","match","return","break","continue",
                "where","as","in","ref","self","Self","super","crate","async","await",
                "move","dyn","type","const","static","unsafe","extern"],
            types: &["i8","i16","i32","i64","i128","u8","u16","u32","u64","u128",
                "f32","f64","bool","char","str","String","Vec","Option","Result",
                "Box","Rc","Arc","HashMap","HashSet","usize","isize"],
        }),
        "py" => Some(Lang {
            line_comment: &["#"],
            block_start: "\"\"\"", block_end: "\"\"\"",
            keywords: &["def","class","if","elif","else","for","while","return","import",
                "from","as","with","try","except","finally","raise","yield","lambda",
                "pass","break","continue","and","or","not","in","is","None","True","False",
                "global","nonlocal","assert","del","async","await"],
            types: &["int","float","str","bool","list","dict","tuple","set","bytes",
                "type","object","Exception"],
        }),
        "rb" | "gemspec" => Some(Lang {
            line_comment: &["#"],
            block_start: "=begin", block_end: "=end",
            keywords: &["def","class","module","if","elsif","else","unless","while","until",
                "for","do","end","return","yield","begin","rescue","ensure","raise",
                "require","require_relative","include","extend","prepend","puts","print","p",
                "attr_accessor","attr_reader","attr_writer","alias","defined?",
                "nil","true","false","self","super","then","when","case","and","or","not",
                "lambda","proc","block_given?","loop","open","each","map","select","reject",
                "freeze","frozen?","dup","clone","respond_to?","send","method_missing"],
            types: &["String","Integer","Float","Array","Hash","Symbol","Proc","IO","File",
                "Dir","Regexp","Range","Struct","Class","Module","Kernel","Object",
                "NilClass","TrueClass","FalseClass","Numeric","Comparable","Enumerable"],
        }),
        "js" | "ts" | "jsx" | "tsx" => Some(Lang {
            line_comment: &["//"],
            block_start: "/*", block_end: "*/",
            keywords: &["function","const","let","var","if","else","for","while","return",
                "class","extends","import","export","from","default","new","this",
                "try","catch","finally","throw","async","await","yield","switch","case",
                "break","continue","typeof","instanceof","delete","void","in","of"],
            types: &["string","number","boolean","any","void","null","undefined","never",
                "object","Array","Promise","Map","Set","Record","Partial"],
        }),
        "go" => Some(Lang {
            line_comment: &["//"],
            block_start: "/*", block_end: "*/",
            keywords: &["func","var","const","type","struct","interface","map","chan",
                "if","else","for","range","switch","case","default","return","break",
                "continue","go","defer","select","package","import","fallthrough"],
            types: &["int","int8","int16","int32","int64","uint","uint8","uint16",
                "uint32","uint64","float32","float64","string","bool","byte","rune",
                "error","nil","true","false","iota"],
        }),
        "c" | "h" | "cpp" | "hpp" | "cc" => Some(Lang {
            line_comment: &["//"],
            block_start: "/*", block_end: "*/",
            keywords: &["if","else","for","while","do","switch","case","default","return",
                "break","continue","goto","typedef","struct","union","enum","sizeof",
                "static","extern","inline","const","volatile","register","auto",
                "class","public","private","protected","virtual","template","namespace",
                "using","throw","try","catch","new","delete","this","nullptr"],
            types: &["int","char","float","double","void","long","short","unsigned",
                "signed","bool","size_t","string","vector","map","set","auto"],
        }),
        "sh" | "bash" | "zsh" | "fish" => Some(Lang {
            line_comment: &["#"],
            block_start: "", block_end: "",
            keywords: &["if","then","else","elif","fi","for","while","do","done","case",
                "esac","in","function","return","exit","local","export","readonly",
                "source","alias","unset","shift","set","eval","exec","trap","true","false"],
            types: &[],
        }),
        "lua" => Some(Lang {
            line_comment: &["--"],
            block_start: "--[[", block_end: "]]",
            keywords: &["function","local","if","then","else","elseif","end","for","while",
                "do","repeat","until","return","break","in","and","or","not",
                "nil","true","false","require"],
            types: &["string","number","table","boolean","thread","userdata"],
        }),
        "java" | "kt" | "kts" | "scala" => Some(Lang {
            line_comment: &["//"],
            block_start: "/*", block_end: "*/",
            keywords: &["class","interface","extends","implements","import","package",
                "public","private","protected","static","final","abstract","void",
                "new","return","if","else","for","while","do","switch","case","break",
                "continue","try","catch","finally","throw","throws","this","super",
                "null","true","false","instanceof","synchronized","volatile"],
            types: &["int","long","float","double","boolean","char","byte","short",
                "String","Integer","Long","Float","Double","Object","List","Map","Set"],
        }),
        "toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" => Some(Lang {
            line_comment: &["#"],
            block_start: "", block_end: "",
            keywords: &["true","false","yes","no","null","none","on","off"],
            types: &[],
        }),
        "sql" => Some(Lang {
            line_comment: &["--"],
            block_start: "/*", block_end: "*/",
            keywords: &["SELECT","FROM","WHERE","INSERT","UPDATE","DELETE","CREATE","DROP",
                "ALTER","TABLE","INDEX","VIEW","JOIN","LEFT","RIGHT","INNER","OUTER",
                "ON","AND","OR","NOT","IN","IS","NULL","AS","ORDER","BY","GROUP",
                "HAVING","LIMIT","OFFSET","UNION","VALUES","SET","INTO","EXISTS",
                "DISTINCT","BETWEEN","LIKE","COUNT","SUM","AVG","MAX","MIN",
                "select","from","where","insert","update","delete","create","drop",
                "alter","table","index","view","join","left","right","inner","outer",
                "on","and","or","not","in","is","null","as","order","by","group",
                "having","limit","offset","union","values","set","into","exists"],
            types: &["INTEGER","TEXT","REAL","BLOB","VARCHAR","BOOLEAN","TIMESTAMP",
                "BIGINT","SMALLINT","SERIAL","UUID"],
        }),
        "css" | "scss" | "less" => Some(Lang {
            line_comment: &["//"],
            block_start: "/*", block_end: "*/",
            keywords: &["import","media","keyframes","font-face","charset","supports",
                "important","none","auto","inherit","initial","unset"],
            types: &[],
        }),
        "html" | "htm" | "xml" | "svg" => Some(Lang {
            line_comment: &[],
            block_start: "<!--", block_end: "-->",
            keywords: &[],
            types: &[],
        }),
        "asm" | "s" => Some(Lang {
            line_comment: &[";"],
            block_start: "", block_end: "",
            keywords: &["section","global","extern","mov","push","pop","call","ret","jmp",
                "je","jne","jz","jnz","jg","jl","jge","jle","cmp","test","add","sub",
                "mul","div","xor","and","or","not","shl","shr","lea","syscall","int",
                "db","dw","dd","dq","resb","resw","resd","resq","equ","times","incbin"],
            types: &["rax","rbx","rcx","rdx","rsi","rdi","rsp","rbp","r8","r9","r10",
                "r11","r12","r13","r14","r15","eax","ebx","ecx","edx","al","bl","cl","dl"],
        }),
        "pl" | "pm" => Some(Lang {
            line_comment: &["#"],
            block_start: "=pod", block_end: "=cut",
            keywords: &["my","our","local","sub","if","elsif","else","unless","while","until",
                "for","foreach","do","last","next","redo","return","die","warn","print",
                "say","use","require","package","BEGIN","END","eval","chomp","chop",
                "push","pop","shift","unshift","splice","grep","map","sort","keys","values",
                "defined","undef","ref","bless","new","open","close","read","write"],
            types: &["STDIN","STDOUT","STDERR","ARGV","ENV","INC"],
        }),
        "xrpn" => Some(Lang {
            line_comment: &["//"],
            block_start: "", block_end: "",
            keywords: &["LBL","GTO","XEQ","RTN","END","PSE","STOP","ISG","DSE",
                "X<>Y","X<0?","X>0?","X=0?","X!=0?","X<=0?","X>=0?","X<Y?","X>Y?",
                "X=Y?","X!=Y?","X<=Y?","X>=Y?","SF","CF","FS?","FC?",
                "STO","RCL","VIEW","AVIEW","PROMPT","INPUT","CLA","CLX",
                "R^","Rv","LASTX","ENTER","SIGN","ABS","IP","FP","RND","MOD",
                "PI","SEED","RAN","COMB","PERM","FACT","GAMMA","GCD",
                "SIN","COS","TAN","ASIN","ACOS","ATAN","SINH","COSH","TANH",
                "LN","LOG","EXP","10^X","SQRT","X^2","Y^X","1/X",
                "BASE","OCT","HEX","DEC","BIN","AND","OR","XOR","NOT","ROTL","ROTR",
                "SIZE","CLRG","CLST","CLR",
                "FIX","SCI","ENG","ALL","WSIZE","BSIGNED","BUNSGN",
                "AOFF","AON","TONE","BEEP"],
            types: &[],
        }),
        "hl" => Some(Lang {
            line_comment: &["#"],
            block_start: "", block_end: "",
            keywords: &["AND","OR","THEN","IF","ELSE","ALSO",
                "EXAMPLE","CONDITION","ENCRYPTION"],
            types: &[],
        }),
        "zig" => Some(Lang {
            line_comment: &["//"],
            block_start: "", block_end: "",
            keywords: &["fn","pub","const","var","if","else","for","while","return",
                "break","continue","switch","struct","enum","union","error","defer",
                "try","catch","comptime","inline","export","extern","test","unreachable"],
            types: &["i8","i16","i32","i64","u8","u16","u32","u64","f16","f32","f64",
                "bool","void","usize","isize","anytype","type"],
        }),
        _ => None,
    }
}

/// Check if we have a language definition for this extension.
pub fn lang_known(ext: &str) -> Option<()> {
    // Markdown, LaTeX, and HyperList have dedicated highlighters
    // (`highlight_markdown`, `highlight_tex`, `highlight_hyperlist`)
    // that don't go through the keyword/comment `Lang` table — list
    // them explicitly so the editor's `detect_kind` routes `.md`,
    // `.tex`, `.hl`, `.woim` to `FileKind::Source(ext)` instead of
    // falling through to `Plain`.
    match ext {
        "md" | "markdown" | "tex" | "latex" | "hl" | "woim" => Some(()),
        _ => lang_for(ext).map(|_| ()),
    }
}

// HyperList color constants — match hyperlist.vim's `hi ... ctermfg=`
// declarations. Vim cterm names map to the bright basic palette
// (indices 8–15) on most modern terminals, which is what the user's
// vim screenshot shows.
const HL_RED:     u8 = 9;    // ctermfg=Red       — Property, dates
const HL_GREEN:   u8 = 10;   // ctermfg=Green     — Qualifier `[…]`, checkboxes
const HL_BLUE:    u8 = 12;   // ctermfg=Blue      — Operator ALL-CAPS:
const HL_MAGENTA: u8 = 13;   // ctermfg=Magenta   — Identifier, Starter `+ `,
                              // Reference, END/SKIP
const HL_CYAN:    u8 = 14;   // ctermfg=Cyan      — Comment `(…)`, Quote `"…"`
const HL_HASH:    u8 = 184;  // ctermfg=184       — Hashtag `#tag`
const HL_SUB:     u8 = 157;  // ctermfg=157       — Substitution `{…}`
const HL_TYPE:    u8 = 10;   // HLsc → linked to Type — Semicolon. Most colour
                              // schemes (and the user's vim) render Type as
                              // green; matches the test-suite line that says
                              // "the preceding semicolon in green".
const HL_GRAY:    u8 = 245;  // dim / truncation

/// HyperList-specific highlighting. Each rule mirrors the corresponding
/// `syn match` / `syn keyword` declaration in
/// `~/Main/G/GIT-isene/hyperlist.vim/syntax/hyperlist.vim` so colors
/// and accepted character classes are identical to vim's behaviour.
///
/// Element priority (must run in this order — overlap is resolved by
/// "first matched, longest wins" the same way vim does):
///
///   1. `^vim:.*`                     → HLvim (gray)
///   2. Indent (TAB / `*`)            → no color (passed through)
///   3. `+ ` / `- ` Starter           → HLmulti (magenta)
///   4. `\` literal marker            → HLlit (italic)
///   5. `<digits>(<digits.>)* ` ident → HLident (magenta)
///   6. `WORD: ` operator/property    → HLop (blue) / HLtag (red)
///   7. Body: walk char-by-char with these rules:
///         `[…]` qualifier            → HLqual (green)
///         `{…}` substitution         → HLsub  (157)
///         `<…>` / `<<…>>` reference  → HLref  (magenta)
///         `(…)` comment              → HLcomment (cyan)
///         `"…"` quote                → HLquote   (cyan)
///         `#tag` hashtag             → HLhash    (184)
///         `END` `SKIP` keywords      → HLkey     (magenta)
///         `TODO` `FIXME` keywords    → HLtodo    (black on yellow)
///         `;` semicolon              → HLsc      (green via Type)
///         ` *…* `                    → HLb (bold)
///         ` /…/ `                    → HLi (italic)
///         ` _…_ `                    → HLu (underline)
pub fn highlight_hyperlist(text: &str, max_lines: usize) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    let mut count = 0;
    let mut in_literal = false;
    for line in text.lines() {
        if count >= max_lines {
            result.push_str(&style::fg("\n...", HL_GRAY));
            break;
        }
        if count > 0 { result.push('\n'); }
        count += 1;

        // Literal block markers: a line whose body is just `\` (after
        // indent) toggles a no-syntax region. Vim's HLlit highlights
        // the `\` itself in italic; HLlc inside the block disables
        // syntax. Track state across lines.
        let trimmed = line.trim_start_matches(|c: char| c == '\t' || c == '*');
        let indent_len = line.len() - trimmed.len();
        if trimmed == "\\" {
            result.push_str(&line[..indent_len]);
            result.push_str(&style::italic("\\"));
            in_literal = !in_literal;
            continue;
        }
        if in_literal {
            // Plain emit — no syntax inside a literal block.
            result.push_str(line);
            continue;
        }
        emit_hl_line(&mut result, line);
    }
    result
}

fn emit_hl_line(out: &mut String, line: &str) {
    // 1. `^vim:.*` modeline.
    if line.starts_with("vim:") {
        out.push_str(&style::fg(line, HL_GRAY));
        return;
    }
    // 2. Strip leading `\t` / `*` indent (passed through verbatim). Spaces
    //    count too: callers that expand tabs before highlighting would
    //    otherwise hide every line-anchored rule behind the indent.
    let indent_len: usize = line
        .chars()
        .take_while(|c| *c == '\t' || *c == '*' || *c == ' ')
        .count();
    let (indent, body) = line.split_at(indent_len);
    out.push_str(indent);
    if body.is_empty() { return; }

    // 3. Multi-line indicator: `+ ` at start. Vim's HLmulti pattern
    //    `^(\t|\*)*+ ` only matches the `+ ` itself (with the indent
    //    in the lookbehind), so ONLY the `+ ` is coloured — the rest of
    //    the line falls through to normal body highlighting.
    // HyperList 2.8 made `-` a neutral Starter alongside the multi-line
    // `+`; both take the same colour.
    let starter = body.starts_with("+ ") || body.starts_with("- ");
    let body = if starter {
        out.push_str(&style::fg(&body[..2], HL_MAGENTA));
        &body[2..]
    } else {
        body
    };
    // 4. Literal-block marker `\` on its own line.
    if body == "\\" {
        out.push_str(&style::italic("\\"));
        return;
    }

    let work: Vec<char> = body.chars().collect();
    let len = work.len();
    let mut i = 0;

    // 5. Identifier `[0-9.]+ ` — at most one leading space-terminated
    //    sequence of digits/dots. Vim's pattern is `[0-9.]* ` which
    //    technically allows zero digits, but rendering an empty span
    //    isn't useful; require at least one digit.
    {
        let mut j = 0;
        while j < len && (work[j].is_ascii_digit() || work[j] == '.') { j += 1; }
        if j > 0 && j < len && work[j] == ' '
            && work[..j].iter().any(|c| c.is_ascii_digit())
        {
            let ident: String = work[..j + 1].iter().collect();
            out.push_str(&style::fg(&ident, HL_MAGENTA));
            i = j + 1;
        }
    }

    // 6. Property / Operator header at the START of each `;`-segment
    //    (semicolons begin a new item on the same line, per the
    //    HyperList definition). For each segment, look for a `: `
    //    INSIDE that segment only — never crossing the next `;`.
    let mut seg_start = i;
    macro_rules! try_header {
        ($from:expr) => {{
            // End of this segment is the next `;` or end-of-line.
            let mut seg_end = $from;
            while seg_end < len && work[seg_end] != ';' { seg_end += 1; }
            if let Some((hdr_end, is_op)) = detect_hl_header(&work[$from..seg_end]) {
                let hdr: String = work[$from..$from + hdr_end].iter().collect();
                let color = if is_op { HL_BLUE } else { HL_RED };
                // A `(comment)` inside an Operator/Property head keeps its
                // cyan, per hyperlist.vim's contains=HLcomment on both.
                let mut rest = hdr.as_str();
                while let Some(a) = rest.find('(') {
                    let Some(b) = rest[a..].find(')') else { break };
                    out.push_str(&style::fg(&rest[..a], color));
                    out.push_str(&style::fg(&rest[a..a + b + 1], HL_CYAN));
                    rest = &rest[a + b + 1..];
                }
                out.push_str(&style::fg(rest, color));
                i = $from + hdr_end;
            } else if work[$from..seg_end].starts_with(&['S', ':', ' '])
                   || work[$from..seg_end].starts_with(&['T', ':', ' '])
            {
                // S: / T: state/transition marker.
                let mark: String = work[$from..$from + 3].iter().collect();
                out.push_str(&style::fg(&mark, HL_BLUE));
                i = $from + 3;
            } else if $from + 2 <= len && work[$from] == '|' && work[$from + 1] == ' ' {
                out.push_str(&style::fg("| ", HL_BLUE));
                i = $from + 2;
            } else if $from + 2 <= len && $from == 0 && work[$from] == '/' && work[$from + 1] == ' ' {
                out.push_str(&style::fg("/ ", HL_BLUE));
                i = $from + 2;
            }
        }};
    }
    try_header!(seg_start);

    // 7. Body walk.
    while i < len {
        let ch = work[i];

        // Semicolon: emit + restart header detection for the next segment.
        if ch == ';' {
            out.push_str(&style::fg(";", HL_TYPE));
            i += 1;
            // Skip optional space after `;`, but DO render it.
            // (Vim's HLtag char class allows leading space, so the
            // header detect handles it; here we just retry.)
            seg_start = i;
            try_header!(seg_start);
            continue;
        }

        // Checkbox: `[ ]` `[_]` `[-]` `[X]` `[x]` `[O]` `[o]`.
        if ch == '[' && i + 2 < len && work[i + 2] == ']'
            && matches!(work[i + 1], 'X' | 'x' | 'O' | 'o' | '-' | ' ' | '_')
        {
            let s: String = work[i..i + 3].iter().collect();
            out.push_str(&style::fg(&s, HL_GREEN));
            i += 3;
            continue;
        }
        // Qualifier `[…]` (greedy non-greedy: stop at first `]`).
        if ch == '[' {
            let start = i;
            i += 1;
            while i < len && work[i] != ']' { i += 1; }
            if i < len { i += 1; }
            let s: String = work[start..i].iter().collect();
            out.push_str(&style::fg(&s, HL_GREEN));
            continue;
        }
        // Substitution `{…}`.
        if ch == '{' {
            let start = i;
            i += 1;
            while i < len && work[i] != '}' { i += 1; }
            if i < len { i += 1; }
            let s: String = work[start..i].iter().collect();
            out.push_str(&style::fg(&s, HL_SUB));
            continue;
        }
        // Reference `<…>` or `<<…>>`. Restricted char class (see
        // `hl_ref_match`) so a `<` inside a `[...]` qualifier or a
        // crossed bracket isn't mistaken for a reference.
        if ch == '<' {
            if let Some(end) = hl_ref_match(&work, i) {
                let s: String = work[i..end].iter().collect();
                out.push_str(&style::fg(&s, HL_MAGENTA));
                i = end;
                continue;
            }
            // Not a reference — emit the literal `<` and move on.
            out.push('<');
            i += 1;
            continue;
        }
        // Comment `(…)` (nested-aware). Inner refs / hashtags / TODOs
        // keep their own colors per vim's `contains=` clauses.
        if ch == '(' {
            let start = i;
            i += 1;
            let mut depth = 1;
            while i < len && depth > 0 {
                if work[i] == '(' { depth += 1; }
                else if work[i] == ')' { depth -= 1; }
                i += 1;
            }
            let s: String = work[start..i].iter().collect();
            emit_with_inner(out, &s, HL_CYAN);
            continue;
        }
        // Quote `"…"`. Inner refs / hashtags / TODOs keep their own
        // colors (vim's HLquote contains HLref, HLhash, HLtodo).
        if ch == '"' {
            let start = i;
            i += 1;
            while i < len && work[i] != '"' { i += 1; }
            if i < len { i += 1; }
            let s: String = work[start..i].iter().collect();
            emit_with_inner(out, &s, HL_CYAN);
            continue;
        }
        // Change markup `##<` / `##>` / `##->` / `##text##` — vim
        // colours the trailing markup in `Error` / `HLmove` (red).
        if ch == '#' && i + 1 < len && work[i + 1] == '#' {
            let start = i;
            i += 2;
            while i < len && !work[i].is_whitespace() { i += 1; }
            let s: String = work[start..i].iter().collect();
            out.push_str(&style::fg(&s, HL_RED));
            continue;
        }
        // Hashtag `#tag` — vim's char class:
        //   [a-zA-ZæøåÆØÅáéóúãõâêôçàÁÉÓÚÃÕÂÊÔÇÀü0-9.:/_&?%=+\-\*]+
        if ch == '#' && i + 1 < len && hl_hash_char(work[i + 1]) {
            let start = i;
            i += 1;
            while i < len && hl_hash_char(work[i]) { i += 1; }
            let s: String = work[start..i].iter().collect();
            out.push_str(&style::fg(&s, HL_HASH));
            continue;
        }
        // Reserved keywords END / SKIP — only as standalone words.
        if ch == 'E' || ch == 'S' {
            let start = i;
            while i < len && work[i].is_ascii_uppercase() { i += 1; }
            let word: String = work[start..i].iter().collect();
            if matches!(word.as_str(), "END" | "SKIP")
                && (i >= len || !work[i].is_alphanumeric())
            {
                out.push_str(&style::fg(&word, HL_MAGENTA));
                continue;
            }
            i = start;
        }
        // TODO / FIXME — black on yellow.
        if ch == 'T' || ch == 'F' {
            let start = i;
            while i < len && work[i].is_ascii_uppercase() { i += 1; }
            let word: String = work[start..i].iter().collect();
            if matches!(word.as_str(), "TODO" | "FIXME")
                && (i >= len || !work[i].is_alphanumeric())
            {
                out.push_str(&style::bg(&style::fg(&word, 0), 11));
                continue;
            }
            i = start;
        }
        // (Semicolon handled above so it can re-trigger header detection
        // for the new segment.)
        // `*bold*` — must be preceded by space/tab/newline AND the
        // closing `*` must be followed by space/EOL.
        if ch == '*'
            && (i == 0 || work[i - 1] == ' ' || work[i - 1] == '\t')
            && i + 1 < len && work[i + 1] != ' ' && work[i + 1] != '*'
        {
            if let Some(rel) = work[i + 1..].iter().position(|&c| c == '*') {
                let close = i + 1 + rel;
                let after_ok = close + 1 >= len || work[close + 1] == ' ';
                if after_ok {
                    let s: String = work[i..=close].iter().collect();
                    out.push_str(&style::bold(&s));
                    i = close + 1;
                    continue;
                }
            }
        }
        // `/italic/` — same boundary rules as bold.
        if ch == '/'
            && (i == 0 || work[i - 1] == ' ' || work[i - 1] == '\t')
            && i + 1 < len && work[i + 1] != ' ' && work[i + 1] != '/'
        {
            if let Some(rel) = work[i + 1..].iter().position(|&c| c == '/') {
                let close = i + 1 + rel;
                let after_ok = close + 1 >= len || work[close + 1] == ' ';
                if after_ok {
                    let s: String = work[i..=close].iter().collect();
                    out.push_str(&style::italic(&s));
                    i = close + 1;
                    continue;
                }
            }
        }
        // `_underline_` — same boundary rules.
        if ch == '_'
            && (i == 0 || work[i - 1] == ' ' || work[i - 1] == '\t')
            && i + 1 < len && work[i + 1] != ' ' && work[i + 1] != '_'
        {
            if let Some(rel) = work[i + 1..].iter().position(|&c| c == '_') {
                let close = i + 1 + rel;
                let after_ok = close + 1 >= len || work[close + 1] == ' ';
                if after_ok {
                    let s: String = work[i..=close].iter().collect();
                    out.push_str(&style::underline(&s));
                    i = close + 1;
                    continue;
                }
            }
        }
        out.push(ch);
        i += 1;
    }
}

/// Emit a delimited region (comment, quote) in `outer_color` while
/// still highlighting nested elements that vim's `contains=` clause
/// keeps active inside: References (HLref → magenta), Hashtags
/// (HLhash → 184), and TODO / FIXME (HLtodo → black on yellow).
fn emit_with_inner(out: &mut String, full: &str, outer_color: u8) {
    let chars: Vec<char> = full.chars().collect();
    let n = chars.len();
    out.push_str(&style::set_fg(outer_color));
    let mut i = 0;
    while i < n {
        let c = chars[i];

        // Reference `<…>` / `<<…>>` — close outer, emit ref in
        // magenta, reopen outer. Same restricted char class as the
        // top-level scanner (`hl_ref_match`).
        if c == '<' {
            if let Some(end) = hl_ref_match(&chars, i) {
                let s: String = chars[i..end].iter().collect();
                out.push_str(&style::reset_fg());
                out.push_str(&style::fg(&s, HL_MAGENTA));
                out.push_str(&style::set_fg(outer_color));
                i = end;
                continue;
            }
            // Not a reference — emit the literal `<` in outer color.
            out.push('<');
            i += 1;
            continue;
        }
        // Hashtag `#tag`.
        if c == '#' && i + 1 < n && hl_hash_char(chars[i + 1]) {
            let start = i;
            i += 1;
            while i < n && hl_hash_char(chars[i]) { i += 1; }
            let s: String = chars[start..i].iter().collect();
            out.push_str(&style::reset_fg());
            out.push_str(&style::fg(&s, HL_HASH));
            out.push_str(&style::set_fg(outer_color));
            continue;
        }
        // TODO / FIXME — black on yellow.
        if c == 'T' || c == 'F' {
            let start = i;
            while i < n && chars[i].is_ascii_uppercase() { i += 1; }
            let word: String = chars[start..i].iter().collect();
            if matches!(word.as_str(), "TODO" | "FIXME")
                && (i >= n || !chars[i].is_alphanumeric())
            {
                out.push_str(&style::reset_fg());
                out.push_str(&style::bg(&style::fg(&word, 0), 11));
                out.push_str(&style::set_fg(outer_color));
                continue;
            }
            i = start;
        }

        out.push(c);
        i += 1;
    }
    out.push_str(&style::reset_fg());
}

/// Char class for HLref content (matches vim's hyperlist.vim regex
/// `HLref = '<\{1,2}[a-zA-Z…ü0-9,.:/ _~&@?%=\+\-\*#]\+>\{1,2}'`).
/// Crucially this EXCLUDES `[ ] " ( ) { } < >`, so a `<` inside a
/// qualifier like `[<+YYYY-MM-DD]` or a crossed bracket like
/// `<Item]>` is NOT a reference — matching vim.
fn hl_ref_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, ',' | '.' | ':' | '/' | ' ' | '_' | '~' | '&' | '@'
                      | '?' | '%' | '=' | '+' | '-' | '*' | '#')
        || matches!(c, 'æ'|'ø'|'å'|'Æ'|'Ø'|'Å'|'á'|'é'|'ó'|'ú'|'ã'|'õ'|'â'|'ê'|'ô'|'ç'|'à'
                      |'Á'|'É'|'Ó'|'Ú'|'Ã'|'Õ'|'Â'|'Ê'|'Ô'|'Ç'|'À'|'ü')
}

/// If a valid HyperList reference starts at `chars[start]` (which must
/// be `<`), return the exclusive end index. A reference is `<`/`<<`,
/// one or more `hl_ref_char`s, then `>`/`>>`. Returns `None` when the
/// body is empty or no closing `>` immediately follows the body — in
/// which case the `<` is a literal (e.g. the time-relation operator
/// in `[<+YYYY-MM-DD]`), not a reference.
fn hl_ref_match(chars: &[char], start: usize) -> Option<usize> {
    let n = chars.len();
    let mut j = start + 1;
    if j < n && chars[j] == '<' { j += 1; } // optional second '<'
    let body_start = j;
    while j < n && hl_ref_char(chars[j]) { j += 1; }
    if j == body_start { return None; }      // empty body
    if j >= n || chars[j] != '>' { return None; } // must close with '>'
    j += 1;
    if j < n && chars[j] == '>' { j += 1; }  // optional second '>'
    Some(j)
}

/// Char class for HLhash content (matches vim's regex):
///   [a-zA-Zæøå…ü0-9.:/_&?%=+\-\*]+
fn hl_hash_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '.' | ':' | '/' | '_' | '&' | '?' | '%' | '=' | '+' | '-' | '*')
        || matches!(c, 'æ'|'ø'|'å'|'Æ'|'Ø'|'Å'|'á'|'é'|'ó'|'ú'|'ã'|'õ'|'â'|'ê'|'ô'|'ç'|'à'
                      |'Á'|'É'|'Ó'|'Ú'|'Ã'|'Õ'|'Â'|'Ê'|'Ô'|'Ç'|'À'|'ü')
}

/// Detect a HyperList Property (HLtag) or Operator (HLop) line header.
/// Returns Some((end_index_inclusive_of_colon_space, is_operator)) or None.
/// Matches vim: first `: ` not inside [..], (..), "..", {..}, <..>.
/// Operator = prefix is all-caps (with allowed punct), Property otherwise.
fn detect_hl_header(work: &[char]) -> Option<(usize, bool)> {
    let mut depth_sq = 0i32;
    let mut depth_pa = 0i32;
    let mut depth_br = 0i32;
    let mut depth_an = 0i32;
    let mut in_quote = false;
    for i in 0..work.len() {
        let c = work[i];
        if in_quote {
            if c == '"' { in_quote = false; }
            continue;
        }
        match c {
            '"' => in_quote = true,
            '[' => depth_sq += 1,
            ']' => depth_sq -= 1,
            '(' => depth_pa += 1,
            ')' => depth_pa -= 1,
            '{' => depth_br += 1,
            '}' => depth_br -= 1,
            '<' => depth_an += 1,
            '>' => depth_an -= 1,
            ':' => {
                if depth_sq <= 0 && depth_pa <= 0 && depth_br <= 0 && depth_an <= 0 {
                    // Followed by space or end-of-line?
                    let next_is_space = i + 1 >= work.len() || work[i + 1] == ' ';
                    if next_is_space {
                        let end = if i + 1 < work.len() { i + 2 } else { i + 1 };
                        // Determine op vs prop: prefix all-caps?
                        let prefix: String = work[..i].iter().collect();
                        let trimmed_prefix = prefix.trim();
                        if trimmed_prefix.is_empty() { return None; }
                        // Operator: all letters in prefix are uppercase (at least one letter),
                        // and only allowed chars: A-Z, space, _, -, (, ), /
                        let has_letter = trimmed_prefix.chars().any(|c| c.is_ascii_alphabetic());
                        let all_upper_or_punct = trimmed_prefix.chars().all(|c|
                            c.is_ascii_uppercase() || matches!(c, ' ' | '_' | '-' | '(' | ')' | '/'));
                        let is_op = has_letter && all_upper_or_punct;
                        return Some((end, is_op));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Highlight source code. Returns ANSI-colored string.
pub fn highlight(text: &str, ext: &str, max_lines: usize) -> String {
    let lang = match lang_for(ext) {
        Some(l) => l,
        None => return plain_with_limit(text, max_lines),
    };

    let mut result = String::with_capacity(text.len() * 2);
    let mut in_block_comment = false;
    // When a string literal hits end-of-line without closing, we carry
    // the delimiter into the next line so the rest of the string keeps
    // its color. Common case: Rust `"..."` literals that wrap, Python
    // triple-quoted blocks, JS template literals, bash heredoc-like
    // multi-line strings. None means we're not currently inside one.
    let mut in_string: Option<char> = None;
    let mut line_count = 0;

    for line in text.lines() {
        if line_count >= max_lines {
            result.push_str(&style::fg("...", theme().comment));
            break;
        }
        if line_count > 0 { result.push('\n'); }
        line_count += 1;

        // Block comment continuation
        if in_block_comment {
            if !lang.block_end.is_empty() {
                if let Some(pos) = line.find(lang.block_end) {
                    result.push_str(&style::fg(&line[..pos + lang.block_end.len()], theme().comment));
                    in_block_comment = false;
                    let rest = &line[pos + lang.block_end.len()..];
                    if !rest.is_empty() {
                        in_string = highlight_line(rest, &lang, &mut result, in_string);
                    }
                } else {
                    result.push_str(&style::fg(line, theme().comment));
                }
            } else {
                result.push_str(&style::fg(line, theme().comment));
            }
            continue;
        }

        // String continuation from previous line — consume up to the
        // matching delimiter (or end-of-line if it doesn't close here),
        // emit as string, then fall through to normal highlighting.
        if let Some(quote) = in_string {
            let mut end = line.len();
            let mut i = 0;
            let bytes = line.as_bytes();
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() { i += 2; continue; }
                if bytes[i] == quote as u8 { end = i + 1; in_string = None; break; }
                i += 1;
            }
            result.push_str(&style::fg(&line[..end], theme().string));
            if in_string.is_some() {
                // Whole line was string content; skip normal pass.
                continue;
            }
            // Closed mid-line; remainder still needs highlighting.
            let rest = &line[end..];
            if !rest.is_empty() {
                in_string = highlight_line(rest, &lang, &mut result, in_string);
            }
            continue;
        }

        // Check for line comment
        let trimmed = line.trim_start();
        let mut is_line_comment = false;
        for lc in lang.line_comment {
            if trimmed.starts_with(lc) {
                is_line_comment = true;
                break;
            }
        }
        if is_line_comment {
            result.push_str(&style::fg(line, theme().comment));
            continue;
        }

        // Check for preprocessor (#include, #define, etc.)
        if trimmed.starts_with('#') && matches!(ext, "c" | "h" | "cpp" | "hpp" | "cc") {
            result.push_str(&style::fg(line, theme().preproc));
            continue;
        }

        // Check for block comment start
        if !lang.block_start.is_empty() && trimmed.contains(lang.block_start) {
            if let Some(pos) = line.find(lang.block_start) {
                if !lang.block_end.is_empty() {
                    if let Some(end) = line[pos + lang.block_start.len()..].find(lang.block_end) {
                        // Single-line block comment
                        in_string = highlight_line(&line[..pos], &lang, &mut result, in_string);
                        let comment_end = pos + lang.block_start.len() + end + lang.block_end.len();
                        result.push_str(&style::fg(&line[pos..comment_end], theme().comment));
                        let rest = &line[comment_end..];
                        if !rest.is_empty() {
                            in_string = highlight_line(rest, &lang, &mut result, in_string);
                        }
                        continue;
                    }
                }
                // Multi-line block comment starts
                in_string = highlight_line(&line[..pos], &lang, &mut result, in_string);
                result.push_str(&style::fg(&line[pos..], theme().comment));
                in_block_comment = true;
                continue;
            }
        }

        in_string = highlight_line(line, &lang, &mut result, in_string);
    }

    result
}

/// Returns `Some(quote_char)` if a string literal is still open at
/// end-of-line (so the caller can carry the state into the next line);
/// `None` if the line ended with all strings closed.
fn highlight_line(line: &str, lang: &Lang, out: &mut String, _carry: Option<char>) -> Option<char> {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    // Carried-over string state is consumed BEFORE this function is
    // called (see `highlight`); only fresh-on-this-line strings can
    // remain open at end-of-line.
    let mut unclosed: Option<char> = None;

    while i < len {
        let ch = chars[i];

        // Strings
        if ch == '"' || ch == '\'' || ch == '`' {
            let quote = ch;
            let start = i;
            i += 1;
            let mut closed = false;
            while i < len {
                if chars[i] == '\\' && i + 1 < len {
                    i += 2; // skip escaped char
                } else if chars[i] == quote {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            out.push_str(&style::fg(&s, theme().string));
            if !closed { unclosed = Some(quote); }
            continue;
        }

        // Ruby/Perl globals ($var), instance (@var), class (@@var)
        if (ch == '$' || ch == '@') && i + 1 < len && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_' || chars[i + 1] == '@') {
            let start = i;
            i += 1;
            if i < len && chars[i] == '@' { i += 1; } // @@class_var
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push_str(&style::fg(&s, theme().typ));
            continue;
        }

        // Ruby symbols :name
        if ch == ':' && i + 1 < len && chars[i + 1].is_alphabetic() {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push_str(&style::fg(&s, theme().string));
            continue;
        }

        // CLI flags: --flag or -f (only after whitespace or start of line)
        if ch == '-' && i + 1 < len && chars[i + 1].is_ascii_alphabetic()
            && (i == 0 || chars[i - 1].is_ascii_whitespace())
        {
            let start = i;
            i += 1;
            if i < len && chars[i] == '-' { i += 1; } // skip second dash
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push_str(&style::fg(&s, theme().keyword));
            continue;
        }

        // Numbers
        if ch.is_ascii_digit() && (i == 0 || !chars[i - 1].is_alphanumeric()) {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == 'x' || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push_str(&style::fg(&s, theme().number));
            continue;
        }

        // Words (identifiers / keywords)
        if ch.is_alphanumeric() || ch == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if lang.keywords.contains(&word.as_str()) {
                out.push_str(&style::fg(&word, theme().keyword));
            } else if lang.types.contains(&word.as_str()) {
                out.push_str(&style::fg(&word, theme().typ));
            } else if i < len && chars[i] == '(' {
                out.push_str(&style::fg(&word, theme().func));
            } else if word.len() > 1 && word.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) {
                // ALL_CAPS constants
                out.push_str(&style::fg(&word, theme().typ));
            } else {
                out.push_str(&word);
            }
            continue;
        }

        // Punctuation
        if matches!(ch, '{' | '}' | '(' | ')' | '[' | ']' | ';' | ':' | ',' | '.' | '-' | '+' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '%' | '?' | '@') {
            out.push_str(&style::fg(&ch.to_string(), theme().punct));
            i += 1;
            continue;
        }

        out.push(ch);
        i += 1;
    }
    unclosed
}

fn plain_with_limit(text: &str, max_lines: usize) -> String {
    let mut result = String::with_capacity(text.len());
    let mut count = 0;
    for line in text.lines() {
        if count >= max_lines {
            result.push_str(&style::fg("\n...", theme().comment));
            break;
        }
        if count > 0 { result.push('\n'); }
        result.push_str(line);
        count += 1;
    }
    result
}

// Markdown color slots — header levels come from the active theme;
// the remaining elements piggy-back on the theme's general fields so
// changing theme moves the whole md palette in lockstep.
fn md_h1()        -> u8 { theme().md_h1 }
fn md_h2()        -> u8 { theme().md_h2 }
fn md_h3()        -> u8 { theme().md_h3 }
fn md_h_other()   -> u8 { theme().md_h_other }
fn md_bold()      -> u8 { theme().keyword }
fn md_code()      -> u8 { theme().string }
fn md_link_text() -> u8 { theme().typ }
fn md_link_url()  -> u8 { theme().comment }
fn md_quote()     -> u8 { theme().comment }
fn md_bullet()    -> u8 { theme().func }
fn md_rule()      -> u8 { theme().punct }
fn md_html()      -> u8 { theme().preproc }

const TEX_CMD: u8 = 51;      // bright cyan
const TEX_ENV: u8 = 117;     // cyan (bold at callsite)
const TEX_COMMENT: u8 = 245; // dim
const TEX_MATH: u8 = 228;    // bright yellow
const TEX_MATH_DELIM: u8 = 220; // yellow
const TEX_BRACE: u8 = 248;   // light gray
const TEX_OPT: u8 = 176;     // mauve (optional args)

const TXT_URL: u8 = 81;      // bright blue
const TXT_EMAIL: u8 = 78;    // green
const TXT_TODO: u8 = 208;    // orange

/// Markdown highlighter for VIEW contexts (kastrup body, pointer
/// preview): expands `| col1 | col2 |` Markdown tables into
/// Unicode-box-drawn tables before per-line highlighting. The
/// rendered output may have a DIFFERENT line count than the input
/// because each source table block becomes multi-row box drawing.
pub fn highlight_markdown(text: &str, max_lines: usize) -> String {
    let text = crust::text::format_markdown_tables(text, 100);
    highlight_markdown_inner(&text, max_lines)
}

/// Markdown highlighter for SOURCE contexts (scribe editor): line
/// count is preserved 1:1 with the input. Tables stay as their
/// `| col1 | col2 |` source form. Use this whenever rendered rows
/// must align with buffer line indices (cursor positioning, edit
/// operations, scroll math).
pub fn highlight_markdown_source(text: &str, max_lines: usize) -> String {
    highlight_markdown_inner(text, max_lines)
}

fn highlight_markdown_inner(text: &str, max_lines: usize) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut in_fence = false;
    let mut fence_marker = String::new();
    let mut count = 0;

    for line in text.lines() {
        if count >= max_lines {
            out.push_str(&style::fg("\n...", md_rule()));
            break;
        }
        if count > 0 { out.push('\n'); }
        CUR_LINE.store(count, std::sync::atomic::Ordering::Relaxed);
        count += 1;

        let trimmed = line.trim_start();

        // Fenced code block detection
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = &trimmed[..3];
            if in_fence {
                if fence_marker == marker {
                    in_fence = false;
                    fence_marker.clear();
                }
            } else {
                in_fence = true;
                fence_marker = marker.to_string();
            }
            out.push_str(&style::fg(line, md_code()));
            continue;
        }
        if in_fence {
            out.push_str(&style::fg(line, md_code()));
            continue;
        }

        // Horizontal rule
        let no_ws: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if no_ws.len() >= 3
            && (no_ws.chars().all(|c| c == '-')
                || no_ws.chars().all(|c| c == '*')
                || no_ws.chars().all(|c| c == '_'))
        {
            out.push_str(&style::fg(line, md_rule()));
            continue;
        }

        // Headers
        if let Some(rest) = trimmed.strip_prefix("###### ") {
            out.push_str(&style::bold(&style::fg(&format!("###### {}", rest), md_h_other())));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("##### ") {
            out.push_str(&style::bold(&style::fg(&format!("##### {}", rest), md_h_other())));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#### ") {
            out.push_str(&style::bold(&style::fg(&format!("#### {}", rest), md_h_other())));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            out.push_str(&style::bold(&style::fg(&format!("### {}", rest), md_h3())));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            out.push_str(&style::bold(&style::fg(&format!("## {}", rest), md_h2())));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            out.push_str(&style::bold(&style::fg(&format!("# {}", rest), md_h1())));
            continue;
        }

        // Blockquote
        if trimmed.starts_with('>') {
            out.push_str(&style::italic(&style::fg(line, md_quote())));
            continue;
        }

        // Reproduce leading whitespace before styled content
        let lead_ws = &line[..line.len() - trimmed.len()];
        out.push_str(lead_ws);

        // List item marker
        let (marker_end, rest_after_marker) = detect_list_marker(trimmed);
        if marker_end > 0 {
            out.push_str(&style::bold(&style::fg(&trimmed[..marker_end], md_bullet())));
            inline_md(rest_after_marker, &mut out);
            continue;
        }

        inline_md(trimmed, &mut out);
    }

    out
}

/// Return (bytes_consumed_by_marker, remainder) if trimmed starts with a list
/// marker ("- ", "* ", "+ ", or "N. "), else (0, trimmed).
fn detect_list_marker(trimmed: &str) -> (usize, &str) {
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 {
        let c = bytes[0];
        if (c == b'-' || c == b'*' || c == b'+') && bytes[1] == b' ' {
            return (2, &trimmed[2..]);
        }
    }
    // Ordered list "123. "
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
    if i > 0 && i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] == b' ' {
        return (i + 2, &trimmed[i + 2..]);
    }
    (0, trimmed)
}

/// Inline markdown: **bold**, *italic* or _italic_, `code`, [text](url),
/// autolinks <url>, HTML tags.
/// If `rest` begins with an inline colour span
/// (`<span style="color:#rrggbb;background-color:#rrggbb">TEXT</span>`),
/// return (chars consumed, ANSI rendering): inner TEXT in the span's
/// colours, the tags dimmed. Otherwise None. Used by `inline_md` so the
/// colour shows live; the span itself is real HTML that exports cleanly.
fn color_span_ansi(rest: &str) -> Option<(usize, String)> {
    if !rest.starts_with("<span style=\"") { return None; }
    let gt = rest.find('>')?;                 // end of the opening tag
    let open = &rest[..=gt];
    let key_start = "<span style=\"".len();
    let q2 = open.rfind('"')?;
    if q2 <= key_start { return None; }
    let decls = &open[key_start..q2];
    let fg = css_hex(decls, "color");
    let bg = css_hex(decls, "background-color");
    // Also dim the tags of font spans (scribe's `\F`), even though the
    // terminal can't render the actual font — the inner text shows plain,
    // the `<span>` clutter dims away, and the font applies on export.
    let has_font = decls.contains("font-family") || decls.contains("font-size");
    if fg.is_none() && bg.is_none() && !has_font { return None; }
    let after = &rest[gt + 1..];
    let close = after.find("</span>")?;
    let inner = &after[..close];
    let mut s = String::new();
    // `\M` conceal: on every line but the cursor's, show only the styled inner
    // (drop the `<span>` tags) so the prose reads clean. The cursor's line keeps
    // the tags so the markup stays editable there.
    if conceal_current_line() {
        s.push_str(&style::coded_rgb(inner, fg, bg));
    } else {
        s.push_str(&style::fg(open, 240));
        s.push_str(&style::coded_rgb(inner, fg, bg));
        s.push_str(&style::fg("</span>", 240));
    }
    let consumed = open.chars().count() + inner.chars().count() + "</span>".chars().count();
    Some((consumed, s))
}

/// Pull a `#rrggbb` value for `key` out of a CSS declaration list
/// (`color:#..;background-color:#..`). Exact-key match so `color`
/// doesn't also match inside `background-color`.
fn css_hex(decls: &str, key: &str) -> Option<(u8, u8, u8)> {
    for decl in decls.split(';') {
        let mut it = decl.splitn(2, ':');
        let k = it.next().unwrap_or("").trim();
        let v = it.next().unwrap_or("").trim();
        if k == key { return style::parse_hex_color(v); }
    }
    None
}

/// Plain text, but with scribe's inline colour/font spans rendered (inner
/// styled, `<span>` tags dimmed). No Markdown styling is applied — for a
/// no-extension / `.txt` prose buffer that just wants the span markup to look
/// right before it's saved as `.md`/`.html`.
pub fn highlight_plain_spans(text: &str, max_lines: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 4);
    let mut count = 0;
    for line in text.lines() {
        if count >= max_lines {
            break;
        }
        if count > 0 {
            out.push('\n');
        }
        CUR_LINE.store(count, std::sync::atomic::Ordering::Relaxed);
        count += 1;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '<'
                && i + 5 <= chars.len()
                && chars[i + 1] == 's' && chars[i + 2] == 'p'
                && chars[i + 3] == 'a' && chars[i + 4] == 'n'
            {
                let rest: String = chars[i..].iter().collect();
                if let Some((consumed, rendered)) = color_span_ansi(&rest) {
                    out.push_str(&rendered);
                    i += consumed;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn inline_md(line: &str, out: &mut String) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Inline colour span: <span style="color:#..;background-color:#..">…</span>
        // Render the inner text in those colours; keep the tags dimmed so
        // source columns still line up with the buffer (same idea as the
        // *bold* / _italic_ markers below). Scribe's `\C` writes these.
        if chars[i] == '<'
            && i + 5 <= chars.len()
            && chars[i + 1] == 's' && chars[i + 2] == 'p'
            && chars[i + 3] == 'a' && chars[i + 4] == 'n'
        {
            let rest: String = chars[i..].iter().collect();
            if let Some((consumed, rendered)) = color_span_ansi(&rest) {
                out.push_str(&rendered);
                i += consumed;
                continue;
            }
        }
        // Inline code `...`
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                let content: String = chars[i..=i + 1 + end].iter().collect();
                out.push_str(&style::fg(&content, md_code()));
                i += 2 + end;
                continue;
            }
        }
        // Bold **...** — preserve markers in output so source columns
        // match buffer columns exactly. CHAR-index walk: don't use
        // `String::find` here, its byte index would skip chars when
        // the bolded span contains multi-byte chars (e.g. `₂` in
        // `[Fe₂O₃]`).
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            let end = chars[i + 2..].windows(2)
                .position(|w| w[0] == '*' && w[1] == '*');
            if let Some(end) = end {
                let content: String = chars[i + 2..i + 2 + end].iter().collect();
                out.push_str(&style::bold(&style::fg(&format!("**{}**", content), md_bold())));
                i += 4 + end;
                continue;
            }
        }
        // Italic *...* (single) or _..._ — preserve markers in output.
        if chars[i] == '*' || chars[i] == '_' {
            let delim = chars[i];
            if i + 1 < chars.len() && chars[i + 1] != delim && chars[i + 1] != ' ' {
                if let Some(end) = chars[i + 1..].iter().position(|&c| c == delim) {
                    let content: String = chars[i + 1..i + 1 + end].iter().collect();
                    if !content.contains('\n') && !content.is_empty() {
                        let with_markers = format!("{}{}{}", delim, content, delim);
                        out.push_str(&style::italic(&with_markers));
                        i += 2 + end;
                        continue;
                    }
                }
            }
        }
        // Markdown link [text](url) — preserve every source char 1:1
        // (brackets and parens included) so this is safe to use in
        // editor / source contexts where rendered column count must
        // equal buffer column count. Brackets render dim; the link
        // text is underlined; the URL inside parens is dim.
        if chars[i] == '[' {
            if let Some(close_txt) = chars[i + 1..].iter().position(|&c| c == ']') {
                let after = i + 1 + close_txt + 1;
                if after < chars.len() && chars[after] == '(' {
                    if let Some(close_url) = chars[after + 1..].iter().position(|&c| c == ')') {
                        let text: String = chars[i + 1..i + 1 + close_txt].iter().collect();
                        let url: String = chars[after + 1..after + 1 + close_url].iter().collect();
                        out.push_str(&style::fg("[", md_link_url()));
                        out.push_str(&style::underline(&style::fg(&text, md_link_text())));
                        out.push_str(&style::fg("]", md_link_url()));
                        out.push_str(&style::fg(&format!("({})", url), md_link_url()));
                        i = after + 1 + close_url + 1;
                        continue;
                    }
                }
            }
        }
        // Autolink <http://...> or HTML tag
        if chars[i] == '<' {
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == '>') {
                let content: String = chars[i + 1..i + 1 + close].iter().collect();
                let seq: String = chars[i..=i + 1 + close].iter().collect();
                if content.starts_with("http://") || content.starts_with("https://") {
                    out.push_str(&style::underline(&style::fg(&seq, md_link_text())));
                } else {
                    out.push_str(&style::fg(&seq, md_html()));
                }
                i += 2 + close;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
}

/// LaTeX/TeX highlighter: commands, environments, comments, math, braces.
pub fn highlight_tex(text: &str, max_lines: usize) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut count = 0;
    let mut in_math_block = false;

    for line in text.lines() {
        if count >= max_lines {
            out.push_str(&style::fg("\n...", TEX_COMMENT));
            break;
        }
        if count > 0 { out.push('\n'); }
        count += 1;

        highlight_tex_line(line, &mut out, &mut in_math_block);
    }
    out
}

fn highlight_tex_line(line: &str, out: &mut String, in_math_block: &mut bool) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Line comment %
        if chars[i] == '%' && (i == 0 || chars[i - 1] != '\\') {
            let rest: String = chars[i..].iter().collect();
            out.push_str(&style::fg(&rest, TEX_COMMENT));
            return;
        }
        // Display math $$...$$
        if i + 1 < chars.len() && chars[i] == '$' && chars[i + 1] == '$' {
            *in_math_block = !*in_math_block;
            out.push_str(&style::fg("$$", TEX_MATH_DELIM));
            i += 2;
            continue;
        }
        // Inline math $...$
        if chars[i] == '$' && (i == 0 || chars[i - 1] != '\\') {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '$') {
                let content: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push_str(&style::fg("$", TEX_MATH_DELIM));
                out.push_str(&style::fg(&content, TEX_MATH));
                out.push_str(&style::fg("$", TEX_MATH_DELIM));
                i += 2 + end;
                continue;
            }
        }
        if *in_math_block {
            out.push_str(&style::fg(&chars[i].to_string(), TEX_MATH));
            i += 1;
            continue;
        }
        // Commands \foo, including \begin{env}, \end{env}
        if chars[i] == '\\' && i + 1 < chars.len() {
            let start = i;
            i += 1;
            // \ followed by single non-letter punct is itself a command (e.g. \\, \&, \$)
            if !chars[i].is_ascii_alphabetic() {
                let cmd: String = chars[start..=i].iter().collect();
                out.push_str(&style::fg(&cmd, TEX_CMD));
                i += 1;
                continue;
            }
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '*') {
                i += 1;
            }
            let cmd: String = chars[start..i].iter().collect();
            let is_env = cmd == "\\begin" || cmd == "\\end";
            if is_env {
                out.push_str(&style::bold(&style::fg(&cmd, TEX_ENV)));
                // Consume {env} with env name in bold
                if i < chars.len() && chars[i] == '{' {
                    if let Some(close) = chars[i + 1..].iter().position(|&c| c == '}') {
                        out.push_str(&style::fg("{", TEX_BRACE));
                        let env: String = chars[i + 1..i + 1 + close].iter().collect();
                        out.push_str(&style::bold(&style::fg(&env, TEX_ENV)));
                        out.push_str(&style::fg("}", TEX_BRACE));
                        i = i + 1 + close + 1;
                        continue;
                    }
                }
            } else {
                out.push_str(&style::fg(&cmd, TEX_CMD));
                // Optional args [...]
                if i < chars.len() && chars[i] == '[' {
                    if let Some(close) = chars[i + 1..].iter().position(|&c| c == ']') {
                        let opt: String = chars[i..=i + 1 + close].iter().collect();
                        out.push_str(&style::fg(&opt, TEX_OPT));
                        i = i + 1 + close + 1;
                        continue;
                    }
                }
            }
            continue;
        }
        // Braces
        if chars[i] == '{' || chars[i] == '}' {
            out.push_str(&style::fg(&chars[i].to_string(), TEX_BRACE));
            i += 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
}

/// Plain text highlighter: URLs, emails, TODO/FIXME/NOTE markers.
pub fn highlight_text(text: &str, max_lines: usize) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut count = 0;
    for line in text.lines() {
        if count >= max_lines {
            out.push_str(&style::fg("\n...", md_rule()));
            break;
        }
        if count > 0 { out.push('\n'); }
        count += 1;
        highlight_text_line(line, &mut out);
    }
    out
}

fn highlight_text_line(line: &str, out: &mut String) {
    // Tokenize on whitespace to find URLs/emails, then scan each word
    // for TODO etc. Must iterate by `char_indices` — a byte-level
    // `bytes[i] as char` test mis-classifies UTF-8 continuation bytes:
    // e.g. '∅' (U+2205) = E2 88 85, and 0x85 cast straight to char is
    // U+0085 (NEXT LINE) which `is_whitespace()` returns true for.
    // The byte loop would then split inside the '∅' and `&line[start..i]`
    // panic on the char-boundary check.
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let total = chars.len();
    let line_len = line.len();
    let mut last = 0usize;
    let mut idx = 0usize;
    while idx < total {
        // Skip whitespace chars.
        while idx < total && chars[idx].1.is_whitespace() { idx += 1; }
        if idx >= total { break; }
        let start = chars[idx].0;
        // Walk to next whitespace.
        while idx < total && !chars[idx].1.is_whitespace() { idx += 1; }
        let end = if idx < total { chars[idx].0 } else { line_len };
        let i = end;  // alias: rest of the function uses `i`
        let word = &line[start..i];
        // Flush prior segment.
        out.push_str(&line[last..start]);
        last = i;

        if word.starts_with("http://") || word.starts_with("https://") || word.starts_with("ftp://") {
            // Trim common trailing punctuation from the url portion
            let (url, tail) = split_url_tail(word);
            out.push_str(&style::underline(&style::fg(url, TXT_URL)));
            out.push_str(tail);
            continue;
        }
        if is_email_like(word) {
            out.push_str(&style::fg(word, TXT_EMAIL));
            continue;
        }
        // TODO/FIXME/NOTE/HACK/XXX
        let core = word.trim_end_matches(|c: char| c == ':' || c == ',' || c == '.' || c == '!');
        if matches!(core, "TODO" | "FIXME" | "NOTE" | "HACK" | "XXX" | "BUG" | "WARN") {
            out.push_str(&style::bold(&style::fg(word, TXT_TODO)));
            continue;
        }
        out.push_str(word);
    }
    out.push_str(&line[last..]);
}

fn split_url_tail(s: &str) -> (&str, &str) {
    let cut = s.trim_end_matches(|c: char|
        matches!(c, '.' | ',' | ';' | ':' | ')' | ']' | '>' | '!' | '?' | '"' | '\'')).len();
    (&s[..cut], &s[cut..])
}

fn is_email_like(word: &str) -> bool {
    let core = word.trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim_end_matches(|c: char| !c.is_alphanumeric());
    if let Some(at) = core.find('@') {
        let (user, domain) = core.split_at(at);
        let domain = &domain[1..];
        !user.is_empty() && domain.contains('.') && !domain.starts_with('.')
    } else {
        false
    }
}
