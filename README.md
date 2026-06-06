# tool-call-plan

Build and execute ordered tool call plans for LLM agents, in Rust.

`tool-call-plan` provides a small, dependency-light data structure for
representing a sequence of tool invocations that an LLM agent intends to make,
along with the lifecycle state of each call. It gives you an ordered plan of
steps, each carrying a tool name and JSON arguments, and tracks every step
through a simple status machine (`Pending` → `Running` → `Done` / `Failed` /
`Skipped`) so an executor can drive the plan forward and inspect progress.

## What it does

- **Model a plan** as an ordered list of steps, each with a stable `id`, a
  `tool_name`, and `serde_json` arguments.
- **Track per-step status** via `StepStatus`: `Pending`, `Running`,
  `Done(Value)`, `Failed(String)`, or `Skipped`.
- **Drive execution** by pulling the next pending step, marking it running, and
  recording its result, failure, or a skip.
- **Inspect progress** with helpers such as `is_complete`, `has_failures`, and
  `count_pending` / `count_done` / `count_failed`.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
tool-call-plan = { git = "https://github.com/MukundaKatta/tool-call-plan-rs" }
serde_json = "1"
```

## Usage

```rust
use tool_call_plan::{ToolCallPlan, StepStatus};
use serde_json::json;

// Build an ordered plan of tool calls.
let mut plan = ToolCallPlan::new("research task");
let search = plan.add_step("search", json!({ "q": "rust async" }));
let summarize = plan.add_step("summarize", json!({ "max_words": 200 }));

assert_eq!(plan.len(), 2);
assert!(!plan.is_complete());

// Drive the plan forward one step at a time.
while let Some(step) = plan.next_pending() {
    let id = step.id;
    plan.mark_running(id);

    // ... actually invoke the tool here, producing a JSON result ...
    plan.mark_done(id, json!({ "ok": true }));
}

assert!(plan.is_complete());
assert!(!plan.has_failures());
assert_eq!(plan.count_done(), 2);
```

### Step lifecycle

Each step starts as `Pending`. An executor typically:

1. Reads the `next_pending()` step.
2. Calls `mark_running(id)`.
3. Records the outcome with `mark_done(id, result)`, `mark_failed(id, reason)`,
   or `skip(id)`.

`Done`, `Failed`, and `Skipped` are terminal states. A plan `is_complete()`
once every step has reached a terminal state (an empty plan is vacuously
complete).

## API overview

| Method | Description |
| --- | --- |
| `ToolCallPlan::new(name)` | Create an empty plan. |
| `add_step(tool, args) -> usize` | Append a step; returns its id. |
| `next_pending() -> Option<&PlanStep>` | First step still `Pending`. |
| `mark_running(id)` / `mark_done(id, v)` / `mark_failed(id, reason)` / `skip(id)` | Transition a step's status. |
| `find(id) -> Option<&PlanStep>` | Look up a step by id. |
| `steps() -> &[PlanStep]` | Borrow all steps in order. |
| `len` / `is_empty` / `is_complete` / `has_failures` | Plan-level queries. |
| `count_pending` / `count_done` / `count_failed` | Status counts. |

## Tech stack

- **Language:** Rust (edition 2021)
- **Dependencies:** [`serde_json`](https://crates.io/crates/serde_json) for
  tool arguments and results.

## Development

```bash
cargo build
cargo test
```

## License

Licensed under the MIT License.
