use std::collections::VecDeque;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Elapsed milliseconds since the rotator was started, used as a time axis for the UI.
pub type ElapsedMs = u64;

/// Which of the three queues an action belongs to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueKind {
    /// `priority_actions_queue`
    Priority,
    /// `priority_actions_side_queue` (one-shot injected actions)
    Side,
    /// Normal traversal (StartToEnd / Reverse / AutoMobbing / PingPong)
    Normal,
}

/// Lightweight identifier for an action shown in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionView {
    /// Priority/normal action id; `None` for side-queue actions without an id.
    pub id: Option<u32>,
    /// Human-readable label, e.g. `"Buff(Rune)"`, `"ErdaShower"`, `"SolveRune"`.
    pub label: String,
    /// Stringified condition kind, e.g. `"EveryMillis(30000)"`.
    pub condition_kind: String,
    /// Whether the action was queued to the front.
    pub queue_to_front: bool,
}

/// Result of evaluating an action's condition this tick.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CondOutcome {
    Queue,
    Skip,
    Ignore,
    /// Condition was not called because `ignoring == true`.
    IgnoredWhileInQueue,
}

/// Why the priority queue dispatch was blocked this tick.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum BlockReason {
    #[default]
    None,
    CannotOverrideCurrentState,
    NormalLinkedActionActive,
    PriorityLinkedActionExecuting,
    SideLoadedActionExecuting,
    QueueToFrontHeld,
}

/// A single debug event emitted by the rotator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotatorDebugEvent {
    /// An action's condition was evaluated (or skipped because `ignoring`).
    ConditionEvaluated {
        at: ElapsedMs,
        action: ActionView,
        outcome: CondOutcome,
        /// Milliseconds until the action can be queued again (>0 = on cooldown).
        /// `None` when the condition has no cooldown semantics.
        cooldown_remaining_ms: Option<i64>,
    },
    /// An action was pushed into a queue.
    Enqueued {
        at: ElapsedMs,
        kind: QueueKind,
        action: ActionView,
        /// Queue length after the push.
        queue_len: usize,
        to_front: bool,
    },
    /// An action was handed to the player for execution.
    Dispatched {
        at: ElapsedMs,
        kind: QueueKind,
        action: ActionView,
    },
    /// The priority dispatch was blocked this tick.
    Blocked {
        at: ElapsedMs,
        reason: BlockReason,
    },
    /// Normal traversal advanced to a new index.
    NormalAdvanced {
        at: ElapsedMs,
        index: usize,
        backward: bool,
        action: ActionView,
    },
    /// Full snapshot sent on first connection or refresh.
    Snapshot(RotatorSnapshot),
}

/// Full state snapshot for initial display or manual refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotatorSnapshot {
    pub at: ElapsedMs,
    pub priority_queue: Vec<ActionView>,
    pub side_queue_len: usize,
    pub normal_index: usize,
    pub normal_backward: bool,
    pub normal_actions: Vec<ActionView>,
    pub priority_actions: Vec<PriorityActionView>,
    pub player_block_reason: BlockReason,
}

/// Per-priority-action view for the condition/cooldown panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityActionView {
    pub action: ActionView,
    pub ignoring: bool,
    /// Milliseconds until the action can be queued again.
    pub cooldown_remaining_ms: Option<i64>,
    pub in_queue_or_executing: bool,
}

// ---------------------------------------------------------------------------
// Sink
// ---------------------------------------------------------------------------

/// Ring-buffer event collector owned by `DefaultRotator`.
#[derive(Debug)]
pub struct RotatorDebugSink {
    pub enabled: bool,
    history: VecDeque<RotatorDebugEvent>,
    capacity: usize,
    started: Instant,
    /// Whether new events have been pushed since the last `drain_pending` call.
    pub dirty: bool,
}

impl Default for RotatorDebugSink {
    fn default() -> Self {
        Self::new(512)
    }
}

impl RotatorDebugSink {
    pub fn new(capacity: usize) -> Self {
        Self {
            enabled: false,
            history: VecDeque::with_capacity(capacity),
            capacity,
            started: Instant::now(),
            dirty: false,
        }
    }

    #[inline]
    pub fn now_ms(&self) -> ElapsedMs {
        self.started.elapsed().as_millis() as ElapsedMs
    }

    #[inline]
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.dirty = false;
            self.history.clear();
        }
    }

    /// Emits an event. No-op when disabled.
    #[inline]
    pub fn emit(&mut self, ev: RotatorDebugEvent) {
        if !self.enabled {
            return;
        }
        if self.history.len() == self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(ev);
        self.dirty = true;
    }

    /// Returns all pending events and clears the dirty flag.
    /// The history itself is **not** cleared so a subsequent `Snapshot` can still be built.
    pub fn drain_pending(&mut self) -> Vec<RotatorDebugEvent> {
        self.dirty = false;
        self.history.iter().cloned().collect()
    }
}
