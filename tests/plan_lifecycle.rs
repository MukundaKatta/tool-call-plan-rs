//! Integration tests that exercise `tool-call-plan` through its public API,
//! the same way a downstream crate would.

use serde_json::json;
use tool_call_plan::{StepStatus, ToolCallPlan};

/// Drive a full plan to completion through the normal executor loop and verify
/// the aggregate state reported afterwards.
#[test]
fn drives_a_plan_to_completion() {
    let mut plan = ToolCallPlan::new("pipeline");
    let a = plan.add_step("fetch", json!({ "url": "https://example.com" }));
    let b = plan.add_step("parse", json!({}));
    let c = plan.add_step("store", json!({}));

    let mut executed = Vec::new();
    while let Some(step) = plan.next_pending() {
        let id = step.id;
        executed.push(step.tool_name.clone());
        plan.mark_running(id);
        plan.mark_done(id, json!({ "step": id }));
    }

    assert_eq!(executed, vec!["fetch", "parse", "store"]);
    assert!(plan.is_complete());
    assert!(!plan.has_failures());
    assert_eq!(plan.count_done(), 3);
    assert_eq!(plan.progress(), 1.0);
    assert_eq!(plan.result(a).unwrap()["step"], 0);
    assert_eq!(plan.result(b).unwrap()["step"], 1);
    assert_eq!(plan.result(c).unwrap()["step"], 2);
}

/// A failed step can be reset and retried, after which the plan completes
/// cleanly with no lingering failures.
#[test]
fn failed_step_can_be_retried() {
    let mut plan = ToolCallPlan::new("retry");
    let id = plan.add_step("flaky", json!({}));

    plan.mark_running(id);
    plan.mark_failed(id, "timeout");
    assert!(plan.has_failures());
    assert_eq!(plan.failures().collect::<Vec<_>>(), vec![(id, "timeout")]);
    // A failed step is terminal, so the plan reads as "complete" but with a
    // failure recorded. `next_pending` therefore yields nothing until we retry.
    assert!(plan.is_complete());
    assert!(plan.next_pending().is_none());

    // Retry: reset back to pending and succeed this time.
    assert!(plan.reset(id));
    assert_eq!(plan.next_pending().map(|s| s.id), Some(id));
    plan.mark_done(id, json!({ "ok": true }));

    assert!(plan.is_complete());
    assert!(!plan.has_failures());
    assert!(matches!(
        plan.find(id).map(|s| &s.status),
        Some(StepStatus::Done(_))
    ));
}

/// Skipped steps count as terminal and contribute to completion.
#[test]
fn skipped_steps_complete_the_plan() {
    let mut plan = ToolCallPlan::new("skips");
    let a = plan.add_step("a", json!({}));
    let b = plan.add_step("b", json!({}));
    plan.mark_done(a, json!({}));
    plan.skip(b);

    assert!(plan.is_complete());
    assert_eq!(plan.count_skipped(), 1);
    assert_eq!(plan.count_done(), 1);
}

/// Mutating with unknown ids must be a no-op rather than a panic.
#[test]
fn unknown_ids_are_ignored() {
    let mut plan = ToolCallPlan::new("safety");
    plan.add_step("a", json!({}));

    // None of these should panic or alter existing state.
    plan.mark_running(999);
    plan.mark_done(999, json!({}));
    plan.mark_failed(999, "nope");
    plan.skip(999);
    assert!(!plan.reset(999));

    assert_eq!(plan.count_pending(), 1);
    assert!(!plan.is_complete());
}
