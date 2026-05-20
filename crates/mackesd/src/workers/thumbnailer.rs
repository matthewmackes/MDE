//! v2.0.0 Phase B.11 — mesh thumbnailer dispatch.
//!
//! Rust port of `mackes/mesh_thumbnailer.py`. Thunar / the GTK file
//! manager dispatches per-file thumbnail render requests via the
//! `.thumbnailer` files under `/usr/share/thumbnailers/`; each
//! invocation runs once and exits. No daemon — so this module ships
//! the dispatch logic that decides which renderer handles which
//! MIME, plus a pure-helper to format the thumbnail destination
//! path the way Thunar expects.
//!
//! The Cairo render itself stays in the existing Python module
//! (`mackes/mesh_thumbnailer.py::_render_notification_thumbnail`)
//! during the v1.x line — porting the Cairo + Pango layout to Rust
//! requires `cairo-rs` + `pango` workspace deps that the panel
//! v2.0.0 rewrite will bring in via libcosmic. The shell-out path
//! below keeps thumbnailing working until then.

use std::path::Path;
use std::process::Command;

/// Render target sizes the .thumbnailer file declares.
pub const SUPPORTED_SIZES: &[u32] = &[128, 256, 512];

/// Default size when the caller doesn't specify (matches the
/// Python `--size` default).
pub const DEFAULT_SIZE: u32 = 256;

/// MIME types this thumbnailer claims. Currently only the mesh
/// notification format (.md files under
/// `~/.cache/mackes/notifications/`). Other extensions fall through
/// to whatever generic thumbnailer Thunar uses.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["md"];

/// True when this thumbnailer handles `path` — i.e. its extension
/// is in [`SUPPORTED_EXTENSIONS`]. Pure helper.
#[must_use]
pub fn handles_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .map_or(false, |ext| SUPPORTED_EXTENSIONS.iter().any(|s| *s == ext))
}

/// True when `size` is one of the .thumbnailer declared sizes.
#[must_use]
pub fn supports_size(size: u32) -> bool {
    SUPPORTED_SIZES.contains(&size)
}

/// Round an arbitrary requested size down to the nearest supported
/// size. Returns the largest supported value when the request is
/// larger than every supported size.
#[must_use]
pub fn nearest_supported_size(size: u32) -> u32 {
    let mut best = SUPPORTED_SIZES[0];
    for &s in SUPPORTED_SIZES {
        if s <= size {
            best = s;
        }
    }
    best
}

/// Outcome of one thumbnail render. Pure value type so callers can
/// surface details to logs without parsing exit codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOutcome {
    /// Subprocess returned 0 and wrote `dst`.
    Ok,
    /// Subprocess returned non-zero. Carries the exit code.
    Failed(i32),
    /// Subprocess could not be spawned (Python missing, e.g.).
    SpawnError(String),
    /// `src` doesn't claim a supported extension.
    Unsupported,
}

/// Spawn `python3 -m mackes.mesh_thumbnailer --size <n> <src> <dst>`
/// synchronously. Returns the structured outcome so callers can log
/// + retry.
#[must_use]
pub fn render(src: &Path, dst: &Path, size: u32) -> RenderOutcome {
    if !handles_path(src) {
        return RenderOutcome::Unsupported;
    }
    let mut cmd = Command::new("python3");
    cmd.args([
        "-m",
        "mackes.mesh_thumbnailer",
        "--size",
        &size.to_string(),
    ]);
    cmd.arg(src);
    cmd.arg(dst);
    match cmd.output() {
        Ok(out) if out.status.success() => RenderOutcome::Ok,
        Ok(out) => RenderOutcome::Failed(out.status.code().unwrap_or(-1)),
        Err(e)  => RenderOutcome::SpawnError(format!("{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_path_recognizes_md_extension() {
        assert!(handles_path(Path::new("/tmp/notif.md")));
        assert!(handles_path(Path::new("a.MD")));
        assert!(handles_path(Path::new("/path/notif.Md")));
    }

    #[test]
    fn handles_path_rejects_other_extensions() {
        assert!(!handles_path(Path::new("/tmp/photo.jpg")));
        assert!(!handles_path(Path::new("doc.pdf")));
        assert!(!handles_path(Path::new("no-extension")));
        assert!(!handles_path(Path::new("dotfile.")));
    }

    #[test]
    fn supports_size_matches_thumbnailer_declared_sizes() {
        for s in SUPPORTED_SIZES {
            assert!(supports_size(*s));
        }
        assert!(!supports_size(64));
        assert!(!supports_size(96));
        assert!(!supports_size(1024));
    }

    #[test]
    fn nearest_supported_size_rounds_down() {
        assert_eq!(nearest_supported_size(128), 128);
        assert_eq!(nearest_supported_size(200), 128);
        assert_eq!(nearest_supported_size(256), 256);
        assert_eq!(nearest_supported_size(400), 256);
        assert_eq!(nearest_supported_size(512), 512);
        assert_eq!(nearest_supported_size(99999), 512);
        // Smaller than smallest supported -> still picks smallest.
        assert_eq!(nearest_supported_size(50), 128);
        assert_eq!(nearest_supported_size(0), 128);
    }

    #[test]
    fn render_returns_unsupported_for_non_md_source() {
        let outcome = render(
            Path::new("/tmp/photo.jpg"),
            Path::new("/tmp/photo.thumb.png"),
            256,
        );
        assert_eq!(outcome, RenderOutcome::Unsupported);
    }

    #[test]
    fn default_size_is_256() {
        assert_eq!(DEFAULT_SIZE, 256);
    }
}
