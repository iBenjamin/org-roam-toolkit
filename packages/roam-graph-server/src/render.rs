//! Render `.org` files to HTML using `orgize`, with link rewriting so
//! `id:` links become in-app navigation, `file:`/`attachment:` links
//! become `/file/<rel>` proxy URLs, and external links open in new
//! tabs.

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use lru::LruCache;

use orgize::Org;

/// Render an org-mode source string to HTML, rewriting links relative
/// to `org_root` (the user's `org-roam-directory`).
pub fn render(source: &str, source_path: &Path, org_root: &Path) -> String {
    let parsed = Org::parse(source);
    let raw = parsed.to_html();
    rewrite_links(&raw, source_path, org_root)
}

/// Rewrite the link forms we care about in raw orgize HTML.
fn rewrite_links(html: &str, source_path: &Path, org_root: &Path) -> String {
    rewrite_id_links(&rewrite_external(&rewrite_file_links(html, source_path, org_root)))
}

fn rewrite_id_links(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let needle = r#"<a href="id:"#;
    while let Some(idx) = rest.find(needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        let Some(end_quote) = after.find('"') else {
            out.push_str(&rest[idx..]);
            return out;
        };
        let id = &after[..end_quote];
        let after_quote = &after[end_quote + 1..];
        let Some(close_open) = after_quote.find('>') else {
            out.push_str(&rest[idx..]);
            return out;
        };
        let label_and_close = &after_quote[close_open + 1..];
        let Some(close_idx) = label_and_close.find("</a>") else {
            out.push_str(&rest[idx..]);
            return out;
        };
        let label = &label_and_close[..close_idx];
        out.push_str(&format!(
            r#"<a href="?node={id}" class="roam-link" data-id="{id}">{label}</a>"#
        ));
        rest = &label_and_close[close_idx + "</a>".len()..];
    }
    out.push_str(rest);
    out
}

fn rewrite_external(html: &str) -> String {
    // Add target="_blank" rel="noopener" to http(s) anchors that don't already have it.
    // Naive single-pass scan; sufficient for our use.
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let needles = [r#"<a href="http://"#, r#"<a href="https://"#];
    'outer: while !rest.is_empty() {
        let mut next = None;
        for n in needles {
            if let Some(idx) = rest.find(n) {
                match next {
                    Some((cur_idx, _)) if idx >= cur_idx => {}
                    _ => next = Some((idx, n)),
                }
            }
        }
        let Some((idx, _)) = next else {
            break 'outer;
        };
        out.push_str(&rest[..idx]);
        // Find the closing ">" of this opening tag
        let Some(close) = rest[idx..].find('>') else {
            out.push_str(&rest[idx..]);
            return out;
        };
        let tag = &rest[idx..idx + close];
        let rewritten = if tag.contains(r#"target=""#) {
            tag.to_string()
        } else {
            format!(r#"{tag} target="_blank" rel="noopener""#)
        };
        out.push_str(&rewritten);
        out.push('>');
        rest = &rest[idx + close + 1..];
    }
    out.push_str(rest);
    out
}

/// Return true if an href value (already stripped of any scheme prefix) is a
/// local file path that `rewrite_file_links` should rewrite.
///
/// orgize 0.10-alpha strips the `file:` scheme and emits bare paths like
/// `./pic.png` or `/abs/path.png`.  We still need to catch explicit
/// `file:…` and `attachment:…` hrefs (kept verbatim by orgize for those
/// schemes).  We must NOT match `id:`, `https:`, `?node=…`, `#`, etc.
fn is_local_file_href(href: &str) -> bool {
    // Explicit schemes orgize preserves as-is.
    if href.starts_with("file:") || href.starts_with("attachment:") {
        return true;
    }
    // Bare relative paths orgize emits after stripping `file:`.
    if href.starts_with("./") || href.starts_with("../") {
        return true;
    }
    // Absolute paths.
    if href.starts_with('/') && !href.starts_with("//") {
        return true;
    }
    false
}

fn rewrite_file_links(html: &str, source_path: &Path, org_root: &Path) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    // Match every anchor opening tag; then inspect the href value.
    let needle = r#"<a href=""#;
    while let Some(idx) = rest.find(needle) {
        let after_needle = &rest[idx + needle.len()..];
        let Some(end_quote) = after_needle.find('"') else {
            // Malformed — copy remainder verbatim.
            out.push_str(&rest[idx..]);
            return out;
        };
        let href = &after_needle[..end_quote];

        if !is_local_file_href(href) {
            // Not a local file link — emit up to and including the needle
            // and the href value, then continue scanning from after the
            // closing quote so we don't loop on the same anchor forever.
            out.push_str(&rest[..idx + needle.len() + end_quote + 1]);
            rest = &after_needle[end_quote + 1..];
            continue;
        }

        // Strip well-known scheme prefixes that orgize may still emit.
        let (raw_target, prefix_for_resolve) = if let Some(s) = href.strip_prefix("attachment:") {
            (s, "attachment:")
        } else if let Some(s) = href.strip_prefix("file:") {
            (s, "file:")
        } else {
            // Bare path emitted by orgize after stripping `file:`.
            (href, "file:")
        };

        out.push_str(&rest[..idx]);

        // Advance past needle + href value + closing quote.
        let after_href = &after_needle[end_quote + 1..];
        let resolved = resolve_under_root(raw_target, source_path, org_root, prefix_for_resolve);

        // Find end of the opening tag then the label then </a>.
        let Some(close_open) = after_href.find('>') else {
            out.push_str(&rest[idx..]);
            return out;
        };
        let label_and_close = &after_href[close_open + 1..];
        let Some(close_idx) = label_and_close.find("</a>") else {
            out.push_str(&rest[idx..]);
            return out;
        };
        let label = &label_and_close[..close_idx];
        out.push_str(&format!(
            r#"<a href="/file/{resolved}" target="_blank" rel="noopener">{label}</a>"#
        ));
        rest = &label_and_close[close_idx + "</a>".len()..];
    }
    out.push_str(rest);
    out
}

fn resolve_under_root(
    raw: &str,
    source_path: &Path,
    org_root: &Path,
    prefix: &str,
) -> String {
    let candidate: PathBuf = if prefix.contains("attachment:") {
        org_root.join("data").join(raw)
    } else if let Some(stripped) = raw.strip_prefix("./") {
        source_path
            .parent()
            .map(|p| p.join(stripped))
            .unwrap_or_else(|| PathBuf::from(stripped))
    } else if raw.starts_with('/') {
        PathBuf::from(raw)
    } else {
        source_path
            .parent()
            .map(|p| p.join(raw))
            .unwrap_or_else(|| PathBuf::from(raw))
    };

    candidate
        .strip_prefix(org_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| candidate.to_string_lossy().into_owned())
}

pub struct Cache {
    inner: LruCache<(PathBuf, SystemTime), String>,
}

impl Cache {
    pub fn new(cap: usize) -> Self {
        Self {
            inner: LruCache::new(NonZeroUsize::new(cap).expect("cap > 0")),
        }
    }

    /// Look up cached HTML for `(path, mtime)`. On miss, run `read_org`
    /// to produce the org source, render it, and cache the result.
    pub fn get_or_render(
        &mut self,
        path: &Path,
        mtime: SystemTime,
        org_root: &Path,
        read_org: impl FnOnce() -> String,
    ) -> String {
        let key = (path.to_path_buf(), mtime);
        if let Some(cached) = self.inner.get(&key) {
            return cached.clone();
        }
        let source = read_org();
        let html = render(&source, path, org_root);
        self.inner.put(key, html.clone());
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_one(org: &str) -> String {
        render(
            org,
            Path::new("/tmp/org/note.org"),
            Path::new("/tmp/org"),
        )
    }

    #[test]
    fn id_link_becomes_in_app_anchor() {
        let html = render_one(
            "[[id:aaaaaaaa-1111-1111-1111-111111111111][Alpha]]",
        );
        // orgize may wrap the link in <p> or other structure; check the
        // semantic parts separately rather than one rigid literal.
        assert!(
            html.contains(r#"href="?node=aaaaaaaa-1111-1111-1111-111111111111""#),
            "got: {html}",
        );
        assert!(html.contains(r#"class="roam-link""#), "got: {html}");
        assert!(
            html.contains(r#"data-id="aaaaaaaa-1111-1111-1111-111111111111""#),
            "got: {html}",
        );
        assert!(html.contains(">Alpha</a>"), "got: {html}");
    }

    #[test]
    fn https_link_gets_target_blank() {
        let html = render_one("[[https://example.com][demo]]");
        assert!(html.contains(r#"target="_blank""#), "got: {html}");
        assert!(html.contains(r#"rel="noopener""#), "got: {html}");
    }

    #[test]
    fn file_link_relative_resolves_under_root() {
        let html = render_one("[[file:./pic.png][pic]]");
        assert!(
            html.contains(r#"href="/file/pic.png""#),
            "got: {html}",
        );
    }

    #[test]
    fn attachment_link_routes_via_file_proxy() {
        let html = render_one("[[attachment:doc.pdf][doc]]");
        assert!(
            html.contains(r#"href="/file/data/doc.pdf""#),
            "got: {html}",
        );
    }

    #[test]
    fn plain_text_passes_through() {
        let html = render_one("This is *bold* and /italic/ text.");
        assert!(html.contains("bold"), "got: {html}");
        assert!(html.contains("italic"), "got: {html}");
    }

    #[test]
    fn does_not_double_rewrite_already_external_anchor() {
        // @@html:...@@ raw snippets are passed through verbatim by orgize.
        let html = render_one(r#"@@html:<a href="https://x" target="_blank">ok</a>@@"#);
        let count = html.matches(r#"target="_blank""#).count();
        assert_eq!(count, 1, "got: {html}");
    }

    #[test]
    fn cache_returns_same_html_until_mtime_changes() {
        use std::time::SystemTime;
        let mut cache = Cache::new(8);
        let path = Path::new("/tmp/cache-test.org");
        let org_root = Path::new("/tmp");
        let mtime1 = SystemTime::UNIX_EPOCH;
        let mtime2 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);

        let html_a = cache.get_or_render(path, mtime1, org_root, || {
            "[[id:aaa][A]]".to_string()
        });
        let html_b = cache.get_or_render(path, mtime1, org_root, || {
            // closure should NOT run on cache hit
            panic!("should not re-render on hit");
        });
        assert_eq!(html_a, html_b);

        // mtime changes → re-render
        let html_c = cache.get_or_render(path, mtime2, org_root, || {
            "[[id:bbb][B]]".to_string()
        });
        assert_ne!(html_a, html_c);
    }
}
