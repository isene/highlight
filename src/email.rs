//! Email-mode rendering shared between scribe (compose tempfiles) and
//! kastrup (right-pane message view). Block colors mirror kastrup's
//! theme defaults so reading mail in kastrup and editing a reply in
//! scribe show identical text.
//!
//! Scope:
//!   * Per-line styling decision → [`line_style_email`]
//!   * Header / body boundary discovery → caller-provided
//!   * Signature delimiter scan → [`find_sig_start`]
//!   * Inline tokens (email addresses, URLs) → [`inline_tokens`]
//!   * SGR + OSC 8 emission with composable attributes → [`emit_email_line`]
//!   * Single-shot email-color overlay for callers that don't need the full
//!     emit machinery → [`color_emails`]

use crust::style;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// xterm-256 palette index used for the curly-underline color on
/// misspelled words. Default 196 (bright red). Mutable at runtime via
/// [`set_miss_color`] so embedding apps (scribe, kastrup) can take the
/// color from their own config.
static MISS_COLOR: AtomicU8 = AtomicU8::new(196);

/// Set the curly-underline color used by [`emit_email_line`] for
/// misspelled words. xterm-256 palette index (0–255).
pub fn set_miss_color(c: u8) { MISS_COLOR.store(c, Ordering::Relaxed); }

/// Currently configured miss color (mostly for diagnostics / popup
/// display).
pub fn miss_color() -> u8 { MISS_COLOR.load(Ordering::Relaxed) }

/// Per-line email styling. Block colors mirror kastrup's right-pane render
/// 1-for-1 (same palette indices), so reading a message in kastrup and
/// composing a reply in scribe shows identical colors. Header KEYs are
/// rendered bold (same color as the value, just bold) — cheap visual
/// distinction without burning a color slot.
///
/// Inline tokens (email addresses → magenta, URLs → blue+underline) are
/// applied on top by the renderer, regardless of which block the line is
/// in. They override the base fg for their span only.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EmailLineStyle {
    /// Default fg (no styling).
    None,
    /// Single foreground color across the whole line.
    Solid(u8),
    /// Header line: whole line in `fg`, with the KEY portion (up through
    /// first `:`) additionally bolded.
    HeaderBold(u8),
}

/// Decide a line's styling from its position relative to the header / body
/// / signature boundaries. The caller pre-computes `header_end` (first
/// blank line, or None for non-email files) and `sig_start` (line index of
/// `-- ` or `--` delimiter; see [`find_sig_start`]).
pub fn line_style_email(
    line: &str,
    line_idx: usize,
    header_end: Option<usize>,
    sig_start: Option<usize>,
) -> EmailLineStyle {
    if let Some(end) = header_end {
        if line_idx < end {
            let trimmed = line.trim_start();
            for key in ["From:", "To:", "Cc:", "Bcc:", "Reply-To:"] {
                if trimmed.starts_with(key) { return EmailLineStyle::HeaderBold(2); }
            }
            if trimmed.starts_with("Subject:") {
                return EmailLineStyle::HeaderBold(1);
            }
            for key in ["Date:", "Message-ID:", "In-Reply-To:", "References:"] {
                if trimmed.starts_with(key) { return EmailLineStyle::HeaderBold(240); }
            }
            if trimmed.starts_with("Attach:") {
                return EmailLineStyle::HeaderBold(208);
            }
            return EmailLineStyle::None;
        }
    }
    if let Some(start) = sig_start {
        if line_idx >= start { return EmailLineStyle::Solid(242); }
    }
    if line.starts_with(">>>>") { return EmailLineStyle::Solid(109); }
    if line.starts_with(">>>")  { return EmailLineStyle::Solid(139); }
    if line.starts_with(">>")   { return EmailLineStyle::Solid(180); }
    if line.starts_with('>')    { return EmailLineStyle::Solid(114); }
    EmailLineStyle::None
}

/// Find the signature delimiter (`-- ` or `--`) at or after `body_start`.
/// Returns the line index. Decoupled from any buffer type — caller hands
/// in a closure that resolves line N to its String content.
pub fn find_sig_start<F>(line_count: usize, body_start: usize, line_at: F) -> Option<usize>
where F: Fn(usize) -> String {
    for i in body_start..line_count {
        let line = line_at(i);
        if line == "-- " || line == "--" { return Some(i); }
    }
    None
}

/// Inline token within a body line — email address or URL — to be overlaid
/// on top of the line's base color. Ranges are byte offsets within the
/// line (not absolute buffer offsets).
#[derive(Clone, Debug)]
pub struct InlineToken {
    pub start: usize,
    pub end: usize,
    /// Override fg for this span. URLs → 4 (kastrup `link`). Email addresses
    /// → 177 (light purple — visible against every block color in the email
    /// scheme, including the gray signature and the green quote1).
    pub fg: u8,
    /// SGR underline (for URLs, in addition to OSC 8 wrap).
    pub underline: bool,
    /// If Some, wrap this span in OSC 8 hyperlink so the host terminal can
    /// click through (kitty / wezterm / foot / glass).
    pub osc8_url: Option<String>,
}

/// Find email + URL tokens on a single line. Returns sorted non-overlapping
/// ranges. URL match wins if a URL contains an email (e.g. `mailto:`).
pub fn inline_tokens(line: &str) -> Vec<InlineToken> {
    static URL_RE: OnceLock<regex::Regex> = OnceLock::new();
    static EMAIL_RE: OnceLock<regex::Regex> = OnceLock::new();
    let url_re = URL_RE.get_or_init(|| {
        // Match http(s):// up to the next whitespace / bracket / quote, then
        // strip trailing punct (.,;:!?'") that's almost certainly sentence
        // terminator rather than part of the URL.
        regex::Regex::new(r#"https?://[^\s<>()\[\]{}'"]+[^\s<>()\[\]{}.,;:!?'"]"#).unwrap()
    });
    let email_re = EMAIL_RE.get_or_init(|| {
        regex::Regex::new(r#"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}"#).unwrap()
    });

    let mut tokens: Vec<InlineToken> = Vec::new();
    for m in url_re.find_iter(line) {
        tokens.push(InlineToken {
            start: m.start(), end: m.end(),
            fg: 4, underline: true,
            osc8_url: Some(m.as_str().to_string()),
        });
    }
    for m in email_re.find_iter(line) {
        if tokens.iter().any(|t| m.start() < t.end && m.end() > t.start) { continue; }
        tokens.push(InlineToken {
            start: m.start(), end: m.end(),
            fg: 177, underline: false, osc8_url: None,
        });
    }
    tokens.sort_by_key(|t| t.start);
    tokens
}

/// Emit one line's worth of styled output into `out`. Composes:
///   * `base_fg`     — the line's block color (None / quote / sig / header).
///   * `bold_until`  — bold the chunk up to this byte offset (header KEY).
///   * `tokens`      — inline overrides (addresses, URLs).
///   * `miss_ranges` — line-relative misspelling spans (curly red underline).
///
/// Walks change-points so SGR opens/closes happen exactly at boundaries —
/// no styling carries over to the next chunk. Uses `\x1b[39m` / `\x1b[22m` /
/// `\x1b[24;59m` (selective resets) instead of `\x1b[0m` so the pane bg is
/// never disturbed mid-line.
pub fn emit_email_line(
    out: &mut String,
    line: &str,
    base_fg: Option<u8>,
    bold_until: Option<usize>,
    tokens: &[InlineToken],
    miss_ranges: &[(usize, usize)],
) {
    if line.is_empty() { return; }
    let mut pos = 0usize;
    while pos < line.len() {
        let bold = bold_until.map_or(false, |k| pos < k);
        let tok = tokens.iter().find(|t| pos >= t.start && pos < t.end);
        let miss = miss_ranges.iter().any(|(s, e)| pos >= *s && pos < *e);
        // For misspellings we recolor the TEXT in `miss_color` as well as
        // emitting the kitty curly-underline SGR. Terminals that support
        // the extended SGR (kitty, wezterm) get curly + colored underline;
        // others (glass and any terminal that just splits ':' like ';')
        // still get a clear red word + plain underline. The fg override
        // takes priority over base_fg / token fg so the spell signal wins.
        let fg = if miss {
            Some(MISS_COLOR.load(Ordering::Relaxed))
        } else {
            tok.map(|t| t.fg).or(base_fg)
        };
        let url = tok.and_then(|t| t.osc8_url.clone());
        let underline = tok.map_or(false, |t| t.underline);

        let mut next = line.len();
        let consider = |x: usize, n: &mut usize| { if x > pos && x < *n { *n = x; } };
        if let Some(k) = bold_until { consider(k, &mut next); }
        for t in tokens {
            consider(t.start, &mut next);
            consider(t.end,   &mut next);
        }
        for (s, e) in miss_ranges {
            consider(*s, &mut next);
            consider(*e, &mut next);
        }
        while next < line.len() && !line.is_char_boundary(next) { next += 1; }

        if let Some(u) = &url {
            out.push_str(&style::hyperlink_open(u));
        }
        // Assemble the SGR parameters, then let crust wrap them: the
        // curly-underline pair (4:3 + 58) has no typed helper.
        let mut params = String::new();
        let mut sep = "";
        if let Some(c) = fg { params.push_str(&format!("{}38;5;{}", sep, c)); sep = ";"; }
        if bold { params.push_str(&format!("{}1", sep)); sep = ";"; }
        if underline { params.push_str(&format!("{}4", sep)); sep = ";"; }
        if miss {
            params.push_str(&format!("{}4:3;58:5:{}", sep, miss_color()));
        }
        if !params.is_empty() { out.push_str(&style::sgr(&params)); }

        out.push_str(&line[pos..next]);

        if miss || underline { out.push_str(&style::sgr("24;59")); }
        if bold { out.push_str(&style::sgr("22")); }
        if fg.is_some() { out.push_str(&style::reset_fg()); }
        if url.is_some() { out.push_str(&style::hyperlink_close()); }

        pos = next;
    }
}

/// Single-shot helper: wrap email addresses in `\x1b[38;5;177m...{restore}`
/// without running the full `emit_email_line` machinery. Used by kastrup's
/// right pane where the line is later wrapped in style::fg(line, color);
/// we need the email's "close" to restore THAT outer color rather than
/// resetting to default.
///
/// `outer_fg = Some(c)` restores fg to color c after the email span; `None`
/// restores to the terminal's default fg via SGR 39.
pub fn color_emails(line: &str, outer_fg: Option<u8>) -> String {
    static EMAIL_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = EMAIL_RE.get_or_init(|| {
        regex::Regex::new(r#"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}"#).unwrap()
    });
    let restore: String = match outer_fg {
        Some(c) => style::set_fg(c),
        None    => style::reset_fg(),
    };
    re.replace_all(line, |caps: &regex::Captures| {
        format!("{}{}{}", style::set_fg(177), &caps[0], restore)
    }).into_owned()
}
