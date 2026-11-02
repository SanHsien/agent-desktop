use crate::convert::string::{free_c_string, opt_string_to_c, string_to_c_lossy};
use crate::types::{
    AdActionResult, AdElementState, action_step::AdActionStep, step_mechanism::AdStepMechanism,
};
use agent_desktop_core::actions::action_result::ActionResult as CoreActionResult;
use agent_desktop_core::actions::action_step_outcome::ActionStepOutcome;
use agent_desktop_core::actions::step_mechanism::StepMechanism;
use std::ptr;

pub(crate) fn action_result_to_c(r: &CoreActionResult) -> AdActionResult {
    let action = string_to_c_lossy(&r.action);
    let post_state = match &r.post_state {
        None => ptr::null_mut(),
        Some(state) => {
            let role = string_to_c_lossy(&state.role);
            let value = opt_string_to_c(state.value.as_deref());
            let state_count = state.states.len() as u32;
            let states = if state.states.is_empty() {
                ptr::null_mut()
            } else {
                let ptrs: Vec<*mut std::os::raw::c_char> =
                    state.states.iter().map(|s| string_to_c_lossy(s)).collect();
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
            };
            let elem = Box::new(AdElementState {
                role,
                states,
                state_count,
                value,
            });
            let raw = Box::into_raw(elem);
            crate::resource::register_allocation(
                crate::resource::AllocationKind::ActionPostState,
                raw,
                1,
            );
            raw
        }
    };
    AdActionResult {
        action,
        ref_id: ptr::null(),
        post_state,
        steps: action_steps_to_c(r),
        step_count: r.steps.len() as u32,
        details_json: r
            .details
            .as_ref()
            .map(|details| string_to_c_lossy(&details.to_string()))
            .unwrap_or(ptr::null_mut()),
        disposition: crate::types::AdDeliverySemantics::from_core(r.disposition()),
    }
}

/// # Safety
///
/// `result` must be null or a pointer to an `AdActionResult` previously written
/// by `ad_execute_action`, `ad_execute_action_with_policy`,
/// `ad_execute_ref_action_exact_with_policy`, or `ad_notification_action`. This frees
/// `post_state`, `steps`, and all nested strings. After this call all pointers
/// inside the struct are invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_free_action_result(result: *mut AdActionResult) {
    crate::ffi_try::trap_panic_void(|| unsafe {
        let Some(r) = result.as_mut() else {
            return;
        };
        free_c_string(r.action as *mut _);
        free_c_string(r.ref_id as *mut _);
        free_c_string(r.details_json as *mut _);
        if !r.steps.is_null() {
            free_step_array(r.steps);
        }
        if crate::resource::take_allocation(
            crate::resource::AllocationKind::ActionPostState,
            r.post_state,
        ) == Some(1)
        {
            if let Some(state) = r.post_state.as_mut() {
                free_c_string(state.role as *mut _);
                free_c_string(state.value as *mut _);
                if !state.states.is_null() {
                    free_state_array(state.states);
                }
                drop(Box::from_raw(r.post_state));
                r.post_state = ptr::null_mut();
            }
        }
        r.action = ptr::null();
        r.ref_id = ptr::null();
        r.steps = ptr::null_mut();
        r.step_count = 0;
        r.details_json = ptr::null();
        r.disposition = crate::types::AdDeliverySemantics::unknown();
    })
}

fn action_steps_to_c(r: &CoreActionResult) -> *mut AdActionStep {
    if r.steps.is_empty() {
        return ptr::null_mut();
    }
    let steps = r
        .steps
        .iter()
        .map(|step| {
            let mechanism = step.mechanism().map(core_mechanism_to_c);
            let verified = step.verified();
            AdActionStep {
                label: string_to_c_lossy(step.label()),
                outcome: string_to_c_lossy(step_outcome_name(&step.outcome)),
                mechanism: mechanism.unwrap_or(AdStepMechanism::SemanticApi) as i32,
                has_mechanism: mechanism.is_some(),
                verified: verified.unwrap_or(false),
                has_verified: verified.is_some(),
                _reserved: 0,
            }
        })
        .collect::<Vec<_>>();
    let len = steps.len();
    let mut boxed = steps.into_boxed_slice();
    let raw = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    crate::resource::register_allocation(crate::resource::AllocationKind::ActionSteps, raw, len);
    raw
}

fn step_outcome_name(outcome: &ActionStepOutcome) -> &'static str {
    match outcome {
        ActionStepOutcome::Attempted => "attempted",
        ActionStepOutcome::Skipped => "skipped",
        ActionStepOutcome::Succeeded => "succeeded",
    }
}

fn core_mechanism_to_c(mechanism: StepMechanism) -> AdStepMechanism {
    match mechanism {
        StepMechanism::SemanticApi => AdStepMechanism::SemanticApi,
        StepMechanism::PhysicalSynthetic => AdStepMechanism::PhysicalSynthetic,
    }
}

unsafe fn free_state_array(states: *mut *mut std::os::raw::c_char) {
    unsafe {
        let Some(len) = crate::resource::take_allocation(
            crate::resource::AllocationKind::ActionStateStrings,
            states,
        ) else {
            return;
        };
        for index in 0..len {
            free_c_string(*states.add(index));
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            states, len,
        )));
    }
}

unsafe fn free_step_array(steps: *mut AdActionStep) {
    unsafe {
        let Some(len) =
            crate::resource::take_allocation(crate::resource::AllocationKind::ActionSteps, steps)
        else {
            return;
        };
        for index in 0..len {
            let step = &mut *steps.add(index);
            free_c_string(step.label as *mut _);
            free_c_string(step.outcome as *mut _);
        }
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            steps, len,
        )));
    }
}

#[cfg(test)]
#[path = "result_tests.rs"]
mod tests;
