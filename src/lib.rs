/*!
tool-call-plan: build and execute ordered tool call plans for LLM agents.

```rust
use tool_call_plan::{ToolCallPlan, PlanStep};
use serde_json::json;

let mut plan = ToolCallPlan::new("research task");
plan.add_step("search", json!({"q": "rust async"}));
plan.add_step("summarize", json!({"max_words": 200}));
assert_eq!(plan.len(), 2);
assert!(!plan.is_complete());
```
*/

use serde_json::Value;

/// Status of a plan step.
#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Done(Value),
    Failed(String),
    Skipped,
}

impl StepStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, StepStatus::Done(_) | StepStatus::Failed(_) | StepStatus::Skipped)
    }
}

/// A single step in a tool call plan.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: usize,
    pub tool_name: String,
    pub args: Value,
    pub status: StepStatus,
}

/// An ordered list of tool calls to execute.
pub struct ToolCallPlan {
    pub name: String,
    steps: Vec<PlanStep>,
    next_id: usize,
}

impl ToolCallPlan {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), steps: Vec::new(), next_id: 0 }
    }

    /// Add a step to the end of the plan.
    pub fn add_step(&mut self, tool: &str, args: Value) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.steps.push(PlanStep { id, tool_name: tool.to_string(), args, status: StepStatus::Pending });
        id
    }

    pub fn len(&self) -> usize { self.steps.len() }
    pub fn is_empty(&self) -> bool { self.steps.is_empty() }

    /// True if all steps are terminal.
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| s.status.is_terminal())
    }

    /// True if any step has Failed status.
    pub fn has_failures(&self) -> bool {
        self.steps.iter().any(|s| matches!(s.status, StepStatus::Failed(_)))
    }

    /// Next pending step.
    pub fn next_pending(&self) -> Option<&PlanStep> {
        self.steps.iter().find(|s| s.status == StepStatus::Pending)
    }

    /// Mark step as running.
    pub fn mark_running(&mut self, id: usize) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == id) {
            s.status = StepStatus::Running;
        }
    }

    /// Mark step as done with result.
    pub fn mark_done(&mut self, id: usize, result: Value) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == id) {
            s.status = StepStatus::Done(result);
        }
    }

    /// Mark step as failed.
    pub fn mark_failed(&mut self, id: usize, reason: &str) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == id) {
            s.status = StepStatus::Failed(reason.to_string());
        }
    }

    /// Skip a step.
    pub fn skip(&mut self, id: usize) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == id) {
            s.status = StepStatus::Skipped;
        }
    }

    pub fn steps(&self) -> &[PlanStep] { &self.steps }

    /// Find step by id.
    pub fn find(&self, id: usize) -> Option<&PlanStep> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Count steps by status.
    pub fn count_pending(&self) -> usize { self.steps.iter().filter(|s| s.status == StepStatus::Pending).count() }
    pub fn count_done(&self) -> usize { self.steps.iter().filter(|s| matches!(s.status, StepStatus::Done(_))).count() }
    pub fn count_failed(&self) -> usize { self.steps.iter().filter(|s| matches!(s.status, StepStatus::Failed(_))).count() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn add_and_len() {
        let mut p = ToolCallPlan::new("test");
        p.add_step("search", json!({}));
        p.add_step("fetch", json!({}));
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn initially_not_complete() {
        let mut p = ToolCallPlan::new("test");
        p.add_step("a", json!({}));
        assert!(!p.is_complete());
    }

    #[test]
    fn complete_when_all_done() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("a", json!({}));
        p.mark_done(id, json!({"ok": true}));
        assert!(p.is_complete());
    }

    #[test]
    fn complete_with_skip() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("a", json!({}));
        p.skip(id);
        assert!(p.is_complete());
    }

    #[test]
    fn has_failures() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("a", json!({}));
        p.mark_failed(id, "error");
        assert!(p.has_failures());
    }

    #[test]
    fn next_pending_returns_first_pending() {
        let mut p = ToolCallPlan::new("test");
        let id1 = p.add_step("a", json!({}));
        let _id2 = p.add_step("b", json!({}));
        p.mark_done(id1, json!({}));
        let next = p.next_pending().unwrap();
        assert_eq!(next.tool_name, "b");
    }

    #[test]
    fn next_pending_none_when_all_terminal() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("a", json!({}));
        p.mark_done(id, json!({}));
        assert!(p.next_pending().is_none());
    }

    #[test]
    fn mark_running() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("a", json!({}));
        p.mark_running(id);
        assert_eq!(p.find(id).unwrap().status, StepStatus::Running);
    }

    #[test]
    fn find_by_id() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("search", json!({"q": "hello"}));
        let s = p.find(id).unwrap();
        assert_eq!(s.tool_name, "search");
        assert_eq!(s.args["q"], "hello");
    }

    #[test]
    fn count_pending_done_failed() {
        let mut p = ToolCallPlan::new("test");
        let a = p.add_step("a", json!({}));
        let b = p.add_step("b", json!({}));
        let c = p.add_step("c", json!({}));
        p.mark_done(a, json!({}));
        p.mark_failed(b, "err");
        assert_eq!(p.count_pending(), 1); // c
        assert_eq!(p.count_done(), 1);    // a
        assert_eq!(p.count_failed(), 1);  // b
    }

    #[test]
    fn ids_sequential() {
        let mut p = ToolCallPlan::new("test");
        let a = p.add_step("a", json!({}));
        let b = p.add_step("b", json!({}));
        assert_eq!(a, 0);
        assert_eq!(b, 1);
    }

    #[test]
    fn done_result_accessible() {
        let mut p = ToolCallPlan::new("test");
        let id = p.add_step("a", json!({}));
        p.mark_done(id, json!({"answer": 42}));
        if let StepStatus::Done(v) = &p.find(id).unwrap().status {
            assert_eq!(v["answer"], 42);
        } else {
            panic!("expected Done");
        }
    }

    #[test]
    fn is_empty_initially() {
        let p = ToolCallPlan::new("empty");
        assert!(p.is_empty());
        assert!(p.is_complete()); // vacuously complete
    }
}
