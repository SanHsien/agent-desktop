use serde_json::json;

use crate::{
    AdapterError, Deadline, IdentityMatch, PlatformAdapter, RefEntry, WindowInfo,
    adapter::{NativeHandle, TreeOptions},
    locator::live_locator::{
        ObservationRequest, ObservationRoot, ObservedTree, materialize_ref_entry,
    },
    refs::recorded_window::{adapter_error, recorded_window},
    refs::ref_full_predicate::FullPredicate,
    refs::ref_identity::element_token,
    refs::ref_live_predicate::LivePredicate,
    refs::resolve_errors::{identity_not_unique, identity_unknown_error},
};

/// The depth a whole-window observation reaches: the limit a snapshot walk
/// supports, so a ref taken from the deepest snapshot node is inside the scope
/// it is captured from and resolved against.
pub(crate) const WHOLE_WINDOW_DEPTH: u8 = 50;

/// Resolves a saved ref under the reviewed identity rule.
///
/// A genuine retained element token resolves through the adapter, which owns
/// re-identification on that platform. Every other entry resolves only when it
/// was unique at capture and exactly one live element still matches its full
/// recorded predicate over complete evidence of the window it was captured
/// from; anything less is refused instead of guessed at.
pub fn resolve_saved_ref(
    adapter: &dyn PlatformAdapter,
    entry: &RefEntry,
    deadline: Deadline,
    lease: Option<&crate::InteractionLease>,
) -> Result<NativeHandle, AdapterError> {
    if element_token(entry).is_some() {
        return adapter.resolve_element_strict(entry, deadline);
    }
    let (_, observed, found) = observe_saved_ref(adapter, entry, deadline, lease, true)?;
    let anchor = positional_anchor(entry, &observed, found)?;
    let handle = adapter.resolve_locator_anchor(&anchor, deadline)?;
    let root = ObservationRoot::Element {
        handle: &handle,
        entry: &anchor,
        root_ref: None,
    };
    confirm_landed(adapter, entry, root, deadline, lease)?;
    Ok(handle)
}

/// A handle that landed a saved ref without the whole-window uniqueness count.
///
/// It converts to nothing: no `Deref`, no `From`, no way to take the
/// [`NativeHandle`] out. A dispatch is handed the handle of a full
/// [`resolve_saved_ref`], which is a plain `NativeHandle`, so the compiler
/// rejects a provisional landing in a dispatch position. The only door is
/// [`ProvisionalHandle::for_reads`], for actionability reads and presentation.
pub(crate) struct ProvisionalHandle(NativeHandle);

impl ProvisionalHandle {
    pub(crate) fn for_reads(&self) -> &NativeHandle {
        &self.0
    }
}

/// Lands a tokenless saved ref at its recorded path and confirms the landed
/// element against the full predicate, skipping the whole-window count.
///
/// The count is what proves the ref still unique, so the [`ProvisionalHandle`]
/// may only feed a wait that runs a full [`resolve_saved_ref`] under the
/// interaction lease before acting, or a presentation that never acts, such as
/// placing the cursor overlay. A landing that fails or does not confirm falls back to the full
/// resolution, so a moved, missing or twinned element gets the same verdict
/// either way and only the common case is cheaper.
pub(crate) fn land_saved_ref(
    adapter: &dyn PlatformAdapter,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<ProvisionalHandle, AdapterError> {
    if element_token(entry).is_none()
        && entry.identity.unique_at_capture
        && let Ok(handle) = adapter.resolve_locator_anchor(entry, deadline)
    {
        let root = ObservationRoot::Element {
            handle: &handle,
            entry,
            root_ref: None,
        };
        if confirm_landed(adapter, entry, root, deadline, None).is_ok() {
            return Ok(ProvisionalHandle(handle));
        }
    }
    resolve_saved_ref(adapter, entry, deadline, None).map(ProvisionalHandle)
}

/// The window a saved ref is captured from, one complete observation of it, and
/// the node in that observation it names.
pub(crate) type SavedRefMatch = (WindowInfo, ObservedTree, usize);

/// The evidence every tokenless ref is judged by, shared by resolution, by a
/// drill-down and by a rooted find, so a capture proof and a later resolve
/// measure one observation. A ref carrying an element token names its node by
/// that token; every other ref names it by the predicate it was capture-proven
/// with, and an entry that was not unique at capture has no such evidence to
/// gather. `identity_only` marks a read that only resolves and publishes nothing.
pub(crate) fn observe_saved_ref(
    adapter: &dyn PlatformAdapter,
    entry: &RefEntry,
    deadline: Deadline,
    lease: Option<&crate::InteractionLease>,
    identity_only: bool,
) -> Result<SavedRefMatch, AdapterError> {
    let token = element_token(entry);
    if token.is_none() && !entry.identity.unique_at_capture {
        return Err(identity_not_unique(entry));
    }
    let window = recorded_window(adapter, entry, deadline)?;
    let base = TreeOptions {
        surface: entry.source.source_surface,
        force_renderer_accessibility: entry.source.renderer_accessibility_forced,
        ..TreeOptions::default()
    };
    let mut request = whole_window_request(&base, deadline)?;
    request.observation_mode.identity_only = identity_only;
    let observed = observe_window(adapter, &request, &window, lease)?;
    let index = saved_ref_node(entry, token, &observed)?;
    let node = observed
        .nodes
        .get(index)
        .ok_or_else(|| AdapterError::internal("saved ref match is missing from the observation"))?;
    adapter.validate_saved_ref_context(entry, node.evidence.identifiers.retained_object())?;
    Ok((window, observed, index))
}

/// Confirms that the element an adapter landed is the one `entry` names, under
/// the same full predicate a saved ref resolves by. A caller that publishes a
/// landed handle - the C find entry point hands one to C callers - validates it
/// here instead of trusting the landing alone.
pub fn confirm_landed_element(
    adapter: &dyn PlatformAdapter,
    entry: &RefEntry,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let root = ObservationRoot::Element {
        handle,
        entry,
        root_ref: None,
    };
    confirm_landed(adapter, entry, root, deadline, None)
}

/// The anchor lands a stored path, which a different element can move into
/// between the live count and the descent, so the handle is re-identified
/// before it is handed back: only the recorded predicate proves the landing.
fn confirm_landed(
    adapter: &dyn PlatformAdapter,
    entry: &RefEntry,
    root: ObservationRoot<'_>,
    deadline: Deadline,
    lease: Option<&crate::InteractionLease>,
) -> Result<(), AdapterError> {
    let options = TreeOptions {
        max_depth: 0,
        surface: entry.source.source_surface,
        ..TreeOptions::default()
    };
    let request = ObservationRequest::snapshot(&options, deadline).validate()?;
    let observed =
        crate::tree::renderer_accessibility::observe_tree(adapter, root, &request, lease)
            .map_err(adapter_error)?;
    let landed = observed
        .roots
        .first()
        .and_then(|index| observed.nodes.get(*index as usize))
        .ok_or_else(|| AdapterError::internal("the landed element was not observed"))?;
    match FullPredicate::of_entry(entry).compare(&LivePredicate::observed(&landed.evidence)) {
        IdentityMatch::Match => Ok(()),
        IdentityMatch::Unknown => Err(identity_unknown_error(entry)),
        IdentityMatch::NoMatch => Err(absent(entry)),
    }
}

/// The one observation a tokenless ref is judged by, in the settings resolution
/// uses: the whole window at the deepest limit a walk supports, so a capture
/// proof and a later resolution count the same evidence. `base` supplies the
/// surface and the renderer-accessibility hint; its depth and presentation
/// settings are ignored.
pub(crate) fn observe_whole_window(
    adapter: &dyn PlatformAdapter,
    base: &TreeOptions,
    window: &WindowInfo,
    deadline: Deadline,
    lease: Option<&crate::InteractionLease>,
) -> Result<ObservedTree, AdapterError> {
    observe_window(
        adapter,
        &whole_window_request(base, deadline)?,
        window,
        lease,
    )
}

fn whole_window_request(
    base: &TreeOptions,
    deadline: Deadline,
) -> Result<ObservationRequest, AdapterError> {
    let options = TreeOptions {
        max_depth: WHOLE_WINDOW_DEPTH,
        surface: base.surface,
        force_renderer_accessibility: base.force_renderer_accessibility,
        ..TreeOptions::default()
    }
    .with_ref_identity_bounds();
    ObservationRequest::snapshot(&options, deadline).validate()
}

fn observe_window(
    adapter: &dyn PlatformAdapter,
    request: &ObservationRequest,
    window: &WindowInfo,
    lease: Option<&crate::InteractionLease>,
) -> Result<ObservedTree, AdapterError> {
    let mut observed = crate::tree::renderer_accessibility::observe_tree(
        adapter,
        ObservationRoot::Window(window),
        request,
        lease,
    )
    .map_err(adapter_error)?;
    if let crate::locator::live_locator::ObservationSource::Window {
        renderer_accessibility_forced,
        ..
    } = &mut observed.source
    {
        *renderer_accessibility_forced = request.observation_mode.force_renderer_accessibility;
    }
    Ok(observed)
}

fn saved_ref_node(
    entry: &RefEntry,
    token: Option<&str>,
    observed: &ObservedTree,
) -> Result<usize, AdapterError> {
    if let Some(token) = token {
        let found = observed
            .nodes
            .iter()
            .position(|node| node.evidence.identifiers.retained_object() == Some(token));
        return match found {
            Some(index) => Ok(index),
            None if observed.is_exhaustive() => Err(absent(entry)),
            None => Err(identity_unknown_error(entry)),
        };
    }
    unique_live_match(entry, observed)
}

fn unique_live_match(entry: &RefEntry, observed: &ObservedTree) -> Result<usize, AdapterError> {
    let predicate = FullPredicate::of_entry(entry);
    let mut found = None;
    let mut incomplete = !observed.is_exhaustive();
    for (index, node) in observed.nodes.iter().enumerate() {
        match predicate.compare(&LivePredicate::observed(&node.evidence)) {
            IdentityMatch::Match if found.is_some() => return Err(ambiguous_live_match(entry)),
            IdentityMatch::Match => found = Some(index),
            IdentityMatch::Unknown => incomplete = true,
            IdentityMatch::NoMatch => {}
        }
    }
    if incomplete {
        return Err(identity_unknown_error(entry));
    }
    found.ok_or_else(|| absent(entry))
}

/// A positional anchor at the matching node's path, carrying the saved
/// predicate: the adapter lands that path and its own anchor verification
/// re-checks the recorded identity, so a row that moved under the path is
/// refused rather than acted on.
fn positional_anchor(
    entry: &RefEntry,
    observed: &ObservedTree,
    index: usize,
) -> Result<RefEntry, AdapterError> {
    let node = observed
        .nodes
        .get(index)
        .ok_or_else(|| AdapterError::internal("saved ref match is missing from the observation"))?;
    let mut anchor = materialize_ref_entry(
        node,
        &observed.source,
        &crate::locator::LocatorQuery::default(),
    );
    anchor.identity = crate::RefEntryIdentity {
        retained_object: entry.identity.retained_object.clone(),
        unique_at_capture: false,
        ..entry.identity.clone()
    };
    Ok(anchor)
}

fn ambiguous_live_match(entry: &RefEntry) -> AdapterError {
    AdapterError::ambiguous_target(
        "Two or more live elements match the saved ref's recorded role and stable identity",
    )
    .with_suggestion(
        "An element identical to the captured one appeared, so neither can be picked by ref and \
         a new snapshot refuses both as twins. Take a new snapshot and act on a control that is \
         unique in the window, or ask the person to confirm which one is meant.",
    )
    .with_details(json!({
        "kind": "saved_ref_ambiguous",
        "retryable": false,
        "complete": true,
        "role": entry.identity.role,
    }))
}

fn absent(entry: &RefEntry) -> AdapterError {
    AdapterError::stale_ref_because("No live element matches the saved ref's recorded identity")
        .with_details(json!({
            "kind": "saved_ref_absent",
            "retryable": false,
            "complete": true,
            "role": entry.identity.role,
        }))
}

#[cfg(test)]
#[path = "resolve_saved_ref_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "resolve_saved_ref_window_tests.rs"]
mod window_tests;

#[cfg(test)]
#[path = "resolve_saved_ref_context_tests.rs"]
mod context_tests;

#[cfg(test)]
#[path = "resolve_saved_ref_test_support.rs"]
mod test_support;
