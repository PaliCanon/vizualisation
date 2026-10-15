//! The pages' rich text: two marks only, as on the React site
//! (`src/components/Pages.tsx`'s `rich`) — `**bold**` and `[label](href)`.
//!
//! Copied from search-front/app/src/pages/rich.rs (commit 6d370b4); no
//! change but this header.

/// One run of a rich text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Span<'a> {
    /// Plain text.
    Text(&'a str),
    /// `**…**`.
    Bold(&'a str),
    /// `[label](href)`.
    Link {
        /// What the link says.
        label: &'a str,
        /// Where it goes: `/…` on this site, anything else outside.
        href: &'a str,
    },
}

/// `text` cut into its runs.
pub(crate) fn spans(text: &str) -> Vec<Span<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some((at, mark, len)) = next_mark(rest) {
        if at > 0 {
            out.push(Span::Text(&rest[..at]));
        }
        out.push(mark);
        rest = &rest[at + len..];
    }
    if !rest.is_empty() {
        out.push(Span::Text(rest));
    }
    out
}

/// The first complete mark in `text`: where it starts, the run, its
/// length in bytes.
fn next_mark(text: &str) -> Option<(usize, Span<'_>, usize)> {
    let mut from = 0;
    while let Some(i) = text[from..].find(['*', '[']).map(|i| i + from) {
        if let Some((span, len)) = bold_at(&text[i..]).or_else(|| link_at(&text[i..])) {
            return Some((i, span, len));
        }
        from = i + 1;
    }
    None
}

/// `**…**` at the start of `text` (no `*` inside, not empty).
fn bold_at(text: &str) -> Option<(Span<'_>, usize)> {
    let inner = text.strip_prefix("**")?;
    let end = inner.find('*')?;
    (end > 0 && inner[end..].starts_with("**")).then(|| (Span::Bold(&inner[..end]), end + 4))
}

/// `[label](href)` at the start of `text`.
fn link_at(text: &str) -> Option<(Span<'_>, usize)> {
    let inner = text.strip_prefix('[')?;
    let close = inner.find(']')?;
    let href = inner[close..].strip_prefix("](")?;
    let end = href.find(')')?;
    let (label, href) = (&inner[..close], &href[..end]);
    (!label.is_empty() && !href.is_empty())
        .then(|| (Span::Link { label, href }, 1 + close + 2 + end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_one_run() {
        assert_eq!(spans("no marks here"), [Span::Text("no marks here")]);
    }

    #[test]
    fn bold_and_links_are_cut_out_of_the_text() {
        assert_eq!(
            spans("by **Sujato**, see [SuttaCentral](https://suttacentral.net)."),
            [
                Span::Text("by "),
                Span::Bold("Sujato"),
                Span::Text(", see "),
                Span::Link {
                    label: "SuttaCentral",
                    href: "https://suttacentral.net"
                },
                Span::Text("."),
            ]
        );
    }

    /// An unclosed mark stays text, as the old regex left it.
    #[test]
    fn an_unclosed_mark_stays_text() {
        assert_eq!(spans("a ** b [c] (d"), [Span::Text("a ** b [c] (d")]);
    }
}
