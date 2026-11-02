use super::*;
use crate::convert::string::c_to_string;
use agent_desktop_core::tree::element_state::ElementState;

#[test]
fn test_action_result_to_c_with_state() {
    let core_result = CoreActionResult::delivered_unverified("click").with_state(ElementState {
        role: "button".to_owned(),
        states: vec!["focused".to_owned(), "enabled".to_owned()],
        value: Some("OK".to_owned()),
        enabled: Some(true),
        hidden: Some(false),
        offscreen: Some(false),
    });
    let c_result = action_result_to_c(&core_result);
    unsafe {
        assert_eq!(c_to_string(c_result.action).as_deref(), Some("click"));
        assert!(c_result.ref_id.is_null());
        assert!(!c_result.post_state.is_null());
        assert!(c_result.steps.is_null());
        assert_eq!(c_result.step_count, 0);
        let state = c_result
            .post_state
            .as_ref()
            .expect("post_state must not be null");
        assert_eq!(c_to_string(state.role).as_deref(), Some("button"));
        assert_eq!(c_to_string(state.value).as_deref(), Some("OK"));
        assert_eq!(state.state_count, 2);
    }
    let mut c_result = c_result;
    unsafe { ad_free_action_result(&mut c_result) };
}

#[test]
fn free_action_result_ignores_mutated_counts() {
    let post_state = Box::new(AdElementState {
        role: crate::convert::string::string_to_c_lossy("button"),
        states: state_array(&["focused"]),
        state_count: u32::MAX,
        value: ptr::null(),
    });
    let mut steps = vec![AdActionStep {
        label: crate::convert::string::string_to_c_lossy("AXPress"),
        outcome: crate::convert::string::string_to_c_lossy("succeeded"),
        mechanism: AdStepMechanism::SemanticApi as i32,
        has_mechanism: true,
        verified: false,
        has_verified: false,
        _reserved: 0,
    }]
    .into_boxed_slice();
    let post_state = Box::into_raw(post_state);
    crate::resource::register_allocation(
        crate::resource::AllocationKind::ActionPostState,
        post_state,
        1,
    );
    crate::resource::register_allocation(
        crate::resource::AllocationKind::ActionSteps,
        steps.as_mut_ptr(),
        steps.len(),
    );
    let mut c_result = AdActionResult {
        action: crate::convert::string::string_to_c_lossy("click"),
        ref_id: ptr::null(),
        post_state,
        steps: steps.as_mut_ptr(),
        step_count: u32::MAX,
        details_json: crate::convert::string::string_to_c_lossy("{\"uncertain\":true}"),
        disposition: crate::types::AdDeliverySemantics::unknown(),
    };
    std::mem::forget(steps);
    unsafe { ad_free_action_result(&mut c_result) };

    assert!(c_result.post_state.is_null());
    assert!(c_result.steps.is_null());
    assert_eq!(c_result.step_count, 0);
    assert!(c_result.details_json.is_null());
}

#[test]
fn action_result_to_c_preserves_steps() {
    let core_result = CoreActionResult::delivered_unverified("click").with_steps(vec![
        agent_desktop_core::actions::action_step::ActionStep::attempted("AXScrollToVisible")
            .with_mechanism(StepMechanism::SemanticApi),
        agent_desktop_core::actions::action_step::ActionStep::succeeded("AXPress")
            .with_mechanism(StepMechanism::SemanticApi)
            .with_verified(true),
    ]);

    let mut c_result = action_result_to_c(&core_result);

    unsafe {
        assert_eq!(c_result.step_count, 2);
        assert!(!c_result.steps.is_null());
        assert_eq!(
            c_to_string((*c_result.steps.add(0)).label).as_deref(),
            Some("AXScrollToVisible")
        );
        assert_eq!(
            c_to_string((*c_result.steps.add(0)).outcome).as_deref(),
            Some("attempted")
        );
        assert!((*c_result.steps.add(0)).has_mechanism);
        assert_eq!(
            (*c_result.steps.add(0)).mechanism,
            AdStepMechanism::SemanticApi as i32
        );
        assert!(!(*c_result.steps.add(0)).has_verified);
        assert_eq!(
            c_to_string((*c_result.steps.add(1)).label).as_deref(),
            Some("AXPress")
        );
        assert_eq!(
            c_to_string((*c_result.steps.add(1)).outcome).as_deref(),
            Some("succeeded")
        );
        assert!((*c_result.steps.add(1)).has_mechanism);
        assert_eq!(
            (*c_result.steps.add(1)).mechanism,
            AdStepMechanism::SemanticApi as i32
        );
        assert!((*c_result.steps.add(1)).has_verified);
        assert!((*c_result.steps.add(1)).verified);
    }

    unsafe { ad_free_action_result(&mut c_result) };
    assert!(c_result.steps.is_null());
    assert_eq!(c_result.step_count, 0);
}

#[test]
fn action_result_to_c_preserves_details_json() {
    let core_result =
        CoreActionResult::delivered_unverified("click").with_details(serde_json::json!({
            "transient_ambiguity": true
        }));

    let mut c_result = action_result_to_c(&core_result);

    unsafe {
        assert_eq!(
            c_to_string(c_result.details_json).as_deref(),
            Some("{\"transient_ambiguity\":true}")
        );
    }
    unsafe { ad_free_action_result(&mut c_result) };
    assert!(c_result.details_json.is_null());
}

#[test]
fn action_result_to_c_preserves_delivery_and_retry_semantics() {
    let core_result = CoreActionResult::delivered_unverified("click");
    let mut result = action_result_to_c(&core_result);

    assert_eq!(
        result.disposition.delivery,
        crate::types::AdDeliveryDisposition::DeliveredUnverified as i32
    );
    assert_eq!(
        result.disposition.retry,
        crate::types::AdRetryDisposition::Unsafe as i32
    );
    unsafe { ad_free_action_result(&mut result) };
}

#[test]
fn free_action_result_rejects_a_mutated_nested_string_pointer() {
    let core_result = CoreActionResult::delivered_unverified("click")
        .with_steps(vec![agent_desktop_core::ActionStep::succeeded("AXPress")]);
    let mut result = action_result_to_c(&core_result);
    let steps = unsafe {
        result
            .steps
            .as_mut()
            .expect("steps must not be null when initialized with non-empty steps")
    };
    let original = steps.label as *mut std::os::raw::c_char;
    let foreign = std::ffi::CString::new("foreign").unwrap().into_raw();
    steps.label = foreign;

    unsafe { ad_free_action_result(&mut result) };
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(foreign) }.to_bytes(),
        b"foreign"
    );
    unsafe {
        drop(std::ffi::CString::from_raw(foreign));
        free_c_string(original);
    }
}

fn state_array(states: &[&str]) -> *mut *mut std::os::raw::c_char {
    let ptrs: Vec<*mut std::os::raw::c_char> = states
        .iter()
        .map(|state| crate::convert::string::string_to_c_lossy(state))
        .collect();
    let len = ptrs.len();
    let mut boxed = ptrs.into_boxed_slice();
    let raw = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    crate::resource::register_allocation(
        crate::resource::AllocationKind::ActionStateStrings,
        raw,
        len,
    );
    raw
}

#[test]
fn test_free_null_action_result() {
    unsafe { ad_free_action_result(ptr::null_mut()) };
}
