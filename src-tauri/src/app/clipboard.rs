//! A native fallback for copying the current selection on macOS.
//!
//! Cmd+C is routed by AppKit to the webview's first responder. When something
//! else holds first responder — a closed sheet, a just-rebuilt webview, a toast
//! — the keystroke is dropped and the selection stays on screen with nothing in
//! the pasteboard, which reads as "copy works sometimes" (a user's report,
//! 2026-11-23).
//!
//! The page decides when to call this: it watches for the document's own `copy`
//! event, and only asks the host to write when that event never arrived. A copy
//! the webview handled normally therefore never reaches this code, so rich text
//! and file copies keep their data.

/// Written to the pasteboard by the fallback.
#[derive(serde::Serialize)]
pub struct ClipboardCopy {
    pub copied: bool,
}

/// Put `text` on the general pasteboard.
///
/// Callers pass the current selection and only call this after the webview
/// failed to raise its own `copy` event, so overwriting is not a concern here.
#[cfg(target_os = "macos")]
fn write_text(text: &str) -> bool {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;

    if text.is_empty() {
        return false;
    }
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    // SAFETY: `NSPasteboardTypeString` is a framework-provided pasteboard type
    // constant, valid for the process lifetime.
    pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString })
}

/// Copy the selection the page could not copy itself.
///
/// Used after the page's own `execCommand("copy")`: a failed WebKit copy still
/// clears the pasteboard and leaves it empty, so an empty pasteboard is the
/// reliable signal that the copy did not land. A non-empty one already holds
/// what the page copied — possibly rich text — and is left untouched.
#[tauri::command]
pub fn copy_selection(text: String) -> ClipboardCopy {
    if !should_copy(&text) {
        return ClipboardCopy { copied: false };
    }
    #[cfg(target_os = "macos")]
    {
        if pasteboard_has_text() {
            return ClipboardCopy { copied: false };
        }
        ClipboardCopy {
            copied: write_text(&text),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        ClipboardCopy { copied: false }
    }
}

/// Whether the general pasteboard already holds text.
#[cfg(target_os = "macos")]
fn pasteboard_has_text() -> bool {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    let pasteboard = NSPasteboard::generalPasteboard();
    // SAFETY: framework-provided pasteboard type constant.
    let value = unsafe { pasteboard.stringForType(NSPasteboardTypeString) };
    value.is_some_and(|text| !text.to_string().trim().is_empty())
}

/// Only a selection with actual characters is worth copying, and an empty one
/// must never clear a pasteboard the user filled themselves.
pub fn should_copy(text: &str) -> bool {
    !text.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::should_copy;

    #[test]
    fn empty_or_blank_text_is_never_copied() {
        for text in ["", " ", "   ", "\n", "\t \n"] {
            assert!(!should_copy(text), "copied {text:?}");
        }
    }

    #[test]
    fn real_text_is_copied() {
        for text in ["hello", " a ", "nhiều dòng\nvăn bản"] {
            assert!(should_copy(text), "refused {text:?}");
        }
    }
}
