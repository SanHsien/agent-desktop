use serde_json::json;

use crate::{AdapterError, AppError, Deadline, PlatformAdapter, RefEntry, WindowInfo};

/// The window the ref was captured from, never another window of the process;
/// a process-owned ref's owner is its live process, whatever windows it opens.
/// The surface lookup resolves shell surfaces, the focused window and a recorded
/// window id. An entry with no window id falls back to the process's windows,
/// where the generation says which process it was; when more than one window of
/// that process still fits, nothing says which one the ref came from, so the
/// lookup refuses instead of taking the first.
pub(crate) fn recorded_window(
    adapter: &dyn PlatformAdapter,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    if entry.source.is_process_owned() {
        return crate::system::app_lookup::live_process_owner(adapter, entry, deadline)
            .map_err(adapter_error);
    }
    let resolved = crate::snapshot::resolve_window_for_surface(
        adapter,
        entry.source.source_app.as_deref(),
        entry.source.source_window_id.as_deref(),
        entry.source.source_surface,
        deadline,
    );
    let mut failure = None;
    match resolved {
        Ok(window) if belongs_to_entry(&window, entry) => return Ok(window),
        Ok(_) => {}
        Err(error) => failure = Some(adapter_error(error)),
    }
    if entry.source.source_window_id.is_some() {
        return Err(failure.unwrap_or_else(gone));
    }
    let listed = adapter.list_windows(
        &crate::adapter::WindowFilter {
            focused_only: false,
            app: entry.source.source_app.clone(),
        },
        deadline,
    )?;
    let mut candidates = listed
        .into_iter()
        .filter(|window| belongs_to_entry(window, entry));
    match (candidates.next(), candidates.next()) {
        (Some(window), None) => Ok(window),
        (Some(_), Some(_)) => Err(ambiguous_window(2 + candidates.count())),
        (None, _) => Err(failure.unwrap_or_else(gone)),
    }
}

/// An error from the app-level lookups, flattened to the adapter error the
/// resolvers return.
pub(crate) fn adapter_error(error: AppError) -> AdapterError {
    match error {
        AppError::Adapter(error) => error,
        other => AdapterError::internal(other.to_string()),
    }
}

fn gone() -> AdapterError {
    AdapterError::stale_ref_because("The window the ref was captured from is gone")
}

fn ambiguous_window(candidates: usize) -> AdapterError {
    AdapterError::ambiguous_target(
        "The saved ref records no window id and more than one window of its process could be the one it was captured from",
    )
    .with_suggestion(
        "Take a new snapshot of the window you mean, then use a ref from it; a new snapshot records its window id.",
    )
    .with_details(json!({
        "kind": "saved_ref_window_ambiguous",
        "retryable": false,
        "complete": true,
        "candidates": candidates,
    }))
}

/// Whether a live window is the one the entry was captured from: the owner kind,
/// pid and a recorded window id must agree, and process generations must agree
/// when both sides carry one, since some macOS window records omit it.
fn belongs_to_entry(window: &WindowInfo, entry: &RefEntry) -> bool {
    window.is_process_owner() == entry.source.is_process_owned()
        && window.pid == entry.process.pid
        && entry
            .source
            .source_window_id
            .as_deref()
            .is_none_or(|id| window.id == id)
        && window
            .process_instance
            .as_deref()
            .zip(entry.process.process_instance.as_deref())
            .is_none_or(|(live, saved)| live == saved)
}
