//! # highlight - Fe2O3 syntax highlighter
//!
//! Lightweight syntax highlighter shared across pointer, scribe, kastrup,
//! and any future Fe2O3 component that needs colored text rendering.
//! Hand-rolled (no syntect dep), tiny binary footprint, ~18 source
//! languages plus dedicated renderers for HyperList, Markdown, LaTeX,
//! plain text (URL/email/TODO highlighting), and email composition.
//!
//! Crate name on crates.io: `fe2o3-highlight`. Imports as `highlight`
//! via `[lib] name`.
//!
//! ## Modules
//!
//! - [`source`] — programming languages (Rust, Python, Bash, Go, …) +
//!   theme system. Public entry points: [`source::highlight`],
//!   [`source::highlight_hyperlist`], [`source::highlight_markdown`],
//!   [`source::highlight_tex`], [`source::highlight_text`].
//! - [`email`] — RFC 822 header / quote-level / signature rendering
//!   for `.eml` and kastrup compose tempfiles. Mirrors kastrup's
//!   right-pane scheme so reading mail in kastrup and composing a
//!   reply in scribe produces identical colors.
//!
//! ## Top-level re-exports
//!
//! For convenience, the most-used items are re-exported at the crate
//! root:

pub mod source;
pub mod email;

// Theme + dispatch — most callers want these.
pub use source::{
    available_themes, set_theme, theme_by_name,
    highlight, highlight_hyperlist, highlight_markdown, highlight_markdown_source,
    highlight_tex, highlight_text,
    lang_known,
};

// Email mode.
pub use email::{
    EmailLineStyle, line_style_email, find_sig_start,
    InlineToken, inline_tokens, emit_email_line, color_emails,
    set_miss_color, miss_color,
};
