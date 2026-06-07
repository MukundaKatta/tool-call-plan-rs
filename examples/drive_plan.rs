//! A runnable example showing how to build a tool call plan and drive it to
//! completion, simulating an executor that invokes each tool in turn.
//!
//! Run with:
//!
//! ```text
//! cargo run --example drive_plan
//! ```

use serde_json::json;
use tool_call_plan::{StepStatus, ToolCallPlan};

fn main() {
    // Build an ordered plan of tool calls for a small research task.
    let mut plan = ToolCallPlan::new("research task");
    let search = plan.add_step("search", json!({ "q": "rust async runtimes" }));
    let _summarize = plan.add_step("summarize", json!({ "max_words": 120 }));
    let _publish = plan.add_step("publish", json!({ "channel": "#agents" }));

    println!("Plan '{}' has {} step(s).", plan.name, plan.len());

    // Drive the plan forward one pending step at a time.
    while let Some(step) = plan.next_pending() {
        let id = step.id;
        let tool = step.tool_name.clone();
        plan.mark_running(id);

        // In a real agent you would dispatch to the named tool here. We
        // simulate two normal results and one intentional failure to show the
        // retry path.
        if id == search {
            // Pretend the first attempt fails, then retry it.
            plan.mark_failed(id, "rate limited");
            println!("step #{id} ({tool}) failed: rate limited; retrying");
            plan.reset(id);
            plan.mark_running(id);
            plan.mark_done(id, json!({ "hits": 3 }));
        } else {
            plan.mark_done(id, json!({ "tool": tool, "ok": true }));
        }

        println!("step #{id} ({tool}) -> done");
    }

    // Inspect the final state of the plan.
    println!(
        "complete={} progress={:.0}% done={} failed={} skipped={}",
        plan.is_complete(),
        plan.progress() * 100.0,
        plan.count_done(),
        plan.count_failed(),
        plan.count_skipped(),
    );

    // Pull a specific result back out by id.
    if let Some(result) = plan.result(search) {
        println!("search result: {result}");
    }

    // Report any failures that remain.
    for (id, reason) in plan.failures() {
        println!("unresolved failure on step #{id}: {reason}");
    }

    assert!(plan.is_complete());
    assert!(!plan.has_failures());
    debug_assert_eq!(plan.find(search).map(|s| &s.status), {
        Some(&StepStatus::Done(json!({ "hits": 3 })))
    });
}
