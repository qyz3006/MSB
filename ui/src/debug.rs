use backend::{
    ActionView, BlockReason, CondOutcome, DebugState, PriorityActionView, QueueKind,
    RotatorDebugEvent, RotatorSnapshot, TransparentShapeDifficulty, auto_record_lie_detector,
    auto_save_rune, debug_state_receiver, record_video, rotator_debug_event_receiver,
    set_rotator_debug_enabled, test_spin_rune, test_transparent_shape, test_violetta,
};
use dioxus::prelude::*;
use tokio::sync::broadcast::error::RecvError;

use crate::components::{
    button::{Button, ButtonStyle},
    section::Section,
};

#[component]
pub fn DebugScreen() -> Element {
    let mut state = use_signal(DebugState::default);

    use_future(move || async move {
        let mut rx = debug_state_receiver().await;
        loop {
            let current_state = match rx.recv().await {
                Ok(state) => state,
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(_)) => continue,
            };
            if current_state != *state.peek() {
                state.set(current_state);
            }
        }
    });

    rsx! {
        div { class: "flex flex-col h-full overflow-y-auto",
            Section { title: "Debug",
                div { class: "grid grid-cols-2 gap-3",
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async {
                            test_spin_rune().await;
                        },

                        "Test spin rune"
                    }
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async {
                            test_violetta().await;
                        },

                        "Test Violetta"
                    }
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async {
                            test_transparent_shape(TransparentShapeDifficulty::Normal).await;
                        },

                        "Test transparent shape normal"
                    }
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async {
                            test_transparent_shape(TransparentShapeDifficulty::Hard).await;
                        },

                        "Test transparent shape hard"
                    }
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async move {
                            record_video(!state.peek().is_recording).await;
                        },

                        if state().is_recording {
                            "Stop recording"
                        } else {
                            "Start recording"
                        }
                    }
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async move {
                            auto_save_rune(!state.peek().is_rune_auto_saving).await;
                        },

                        if state().is_rune_auto_saving {
                            "Stop auto saving rune"
                        } else {
                            "Start auto saving rune"
                        }
                    }
                    Button {
                        style: ButtonStyle::Secondary,
                        on_click: move |_| async move {
                            let recording = state.peek().is_lie_detector_auto_recording;
                            auto_record_lie_detector(!recording).await;
                        },

                        if state().is_lie_detector_auto_recording {
                            "Stop auto record lie detector"
                        } else {
                            "Start auto record lie detector"
                        }
                    }
                }
            }
            RotatorDebugPanel {}
        }
    }
}

// ---------------------------------------------------------------------------
// Rotator Debug Panel
// ---------------------------------------------------------------------------

/// Local UI state rebuilt from incoming events.
#[derive(Default, Clone)]
struct RotatorDebugState {
    priority_queue: Vec<ActionView>,
    side_queue_len: usize,
    normal_index: usize,
    normal_backward: bool,
    normal_actions: Vec<ActionView>,
    priority_actions: Vec<PriorityActionView>,
    player_block_reason: BlockReason,
    timeline: Vec<TimelineEntry>,
    enabled: bool,
    paused: bool,
}

#[derive(Clone)]
struct TimelineEntry {
    at_ms: u64,
    text: String,
    is_warn: bool,
}


impl RotatorDebugState {
    fn apply_event(&mut self, ev: &RotatorDebugEvent) {
        match ev {
            RotatorDebugEvent::Snapshot(snap) => {
                self.priority_queue = snap.priority_queue.clone();
                self.side_queue_len = snap.side_queue_len;
                self.normal_index = snap.normal_index;
                self.normal_backward = snap.normal_backward;
                self.normal_actions = snap.normal_actions.clone();
                self.priority_actions = snap.priority_actions.clone();
                self.player_block_reason = snap.player_block_reason.clone();
            }
            RotatorDebugEvent::Enqueued {
                at,
                kind,
                action,
                queue_len,
                to_front,
            } => {
                let text = format!(
                    "{:.2}s  Enqueued  {:?}  {}  (len={}) {}",
                    *at as f64 / 1000.0,
                    kind,
                    action.label,
                    queue_len,
                    if *to_front { "[front]" } else { "" }
                );
                self.push_timeline(TimelineEntry {
                    at_ms: *at,
                    text,
                    is_warn: false,
                });
                match kind {
                    QueueKind::Priority => {
                        if *to_front {
                            self.priority_queue.insert(0, action.clone());
                        } else {
                            self.priority_queue.push(action.clone());
                        }
                    }
                    QueueKind::Side => {
                        self.side_queue_len += 1;
                    }
                    QueueKind::Normal => {}
                }
            }
            RotatorDebugEvent::Dispatched { at, kind, action } => {
                let text = format!(
                    "{:.2}s  Dispatched  {:?}  {}",
                    *at as f64 / 1000.0,
                    kind,
                    action.label
                );
                self.push_timeline(TimelineEntry {
                    at_ms: *at,
                    text,
                    is_warn: false,
                });
                match kind {
                    QueueKind::Priority => {
                        self.priority_queue.retain(|a| a.label != action.label);
                    }
                    QueueKind::Side => {
                        self.side_queue_len = self.side_queue_len.saturating_sub(1);
                    }
                    QueueKind::Normal => {}
                }
            }
            RotatorDebugEvent::Blocked { at, reason } => {
                self.player_block_reason = reason.clone();
                let text = format!(
                    "{:.2}s  Blocked  {:?}",
                    *at as f64 / 1000.0,
                    reason
                );
                self.push_timeline(TimelineEntry {
                    at_ms: *at,
                    text,
                    is_warn: false,
                });
            }
            RotatorDebugEvent::ConditionEvaluated {
                at,
                action,
                outcome,
                cooldown_remaining_ms,
            } => {
                let is_warn = matches!(outcome, CondOutcome::IgnoredWhileInQueue);
                if is_warn {
                    let cooldown_str = cooldown_remaining_ms
                        .map(|ms| {
                            if ms <= 0 {
                                "ready".to_string()
                            } else {
                                format!("{:.1}s", ms as f64 / 1000.0)
                            }
                        })
                        .unwrap_or_default();
                    let text = format!(
                        "{:.2}s  IgnoredWhileInQueue  {}  cooldown={}",
                        *at as f64 / 1000.0,
                        action.label,
                        cooldown_str
                    );
                    self.push_timeline(TimelineEntry {
                        at_ms: *at,
                        text,
                        is_warn: true,
                    });
                }
                // Update the cooldown info in priority_actions view
                if let Some(pav) = self
                    .priority_actions
                    .iter_mut()
                    .find(|p| p.action.id == action.id)
                {
                    pav.ignoring = matches!(outcome, CondOutcome::IgnoredWhileInQueue);
                    pav.cooldown_remaining_ms = *cooldown_remaining_ms;
                }
            }
            RotatorDebugEvent::NormalAdvanced {
                index, backward, ..
            } => {
                self.normal_index = *index;
                self.normal_backward = *backward;
            }
        }
    }

    fn push_timeline(&mut self, entry: TimelineEntry) {
        const MAX_TIMELINE: usize = 200;
        if self.timeline.len() >= MAX_TIMELINE {
            self.timeline.remove(0);
        }
        self.timeline.push(entry);
    }
}

#[component]
pub fn RotatorDebugPanel() -> Element {
    let mut debug = use_signal(RotatorDebugState::default);

    use_future(move || async move {
        let mut rx = rotator_debug_event_receiver().await;
        loop {
            let events = match rx.recv().await {
                Ok(events) => events,
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(_)) => continue,
            };
            let is_paused = debug.peek().paused;
            debug.with_mut(|d| {
                for ev in &events {
                    // Always apply to state; timeline is gated on pause below
                    match ev {
                        RotatorDebugEvent::ConditionEvaluated { .. }
                        | RotatorDebugEvent::Enqueued { .. }
                        | RotatorDebugEvent::Dispatched { .. }
                        | RotatorDebugEvent::Blocked { .. }
                        | RotatorDebugEvent::NormalAdvanced { .. }
                        | RotatorDebugEvent::Snapshot(_) => {
                            if is_paused {
                                // Still update structural state but skip timeline push
                                match ev {
                                    RotatorDebugEvent::Snapshot(snap) => {
                                        d.priority_queue = snap.priority_queue.clone();
                                        d.side_queue_len = snap.side_queue_len;
                                        d.normal_index = snap.normal_index;
                                        d.normal_backward = snap.normal_backward;
                                        d.normal_actions = snap.normal_actions.clone();
                                        d.priority_actions = snap.priority_actions.clone();
                                        d.player_block_reason = snap.player_block_reason.clone();
                                    }
                                    RotatorDebugEvent::NormalAdvanced { index, backward, .. } => {
                                        d.normal_index = *index;
                                        d.normal_backward = *backward;
                                    }
                                    _ => {}
                                }
                            } else {
                                d.apply_event(ev);
                            }
                        }
                    }
                }
            });
        }
    });

    let enabled = debug.read().enabled;
    let paused = debug.read().paused;

    rsx! {
        Section { title: "Rotator Debug",
            // Controls row
            div { class: "flex gap-2 mb-2",
                Button {
                    style: if enabled { ButtonStyle::Primary } else { ButtonStyle::Secondary },
                    on_click: move |_| async move {
                        let new_enabled = !debug.peek().enabled;
                        set_rotator_debug_enabled(new_enabled).await;
                        debug.with_mut(|d| {
                            d.enabled = new_enabled;
                            if !new_enabled {
                                *d = RotatorDebugState::default();
                            }
                        });
                    },
                    if enabled { "Disable" } else { "Enable" }
                }
                Button {
                    style: if paused { ButtonStyle::Primary } else { ButtonStyle::Secondary },
                    on_click: move |_| {
                        debug.with_mut(|d| d.paused = !d.paused);
                    },
                    if paused { "Resume" } else { "Pause" }
                }
            }

            if enabled {
                // ① Queues
                div { class: "mb-3",
                    div { class: "text-xs text-primary-text font-medium mb-1", "① Queues" }
                    div { class: "flex gap-3 flex-wrap",
                        // Priority queue
                        div { class: "flex-1 min-w-32",
                            div { class: "text-xs text-secondary-text font-medium mb-1", "Priority" }
                            div { class: "bg-secondary-surface p-2 min-h-8 text-xs font-mono",
                                if debug.read().priority_queue.is_empty() {
                                    span { class: "text-tertiary-text", "(empty)" }
                                } else {
                                    for av in debug.read().priority_queue.clone() {
                                        div {
                                            key: "{av.label}",
                                            class: "truncate",
                                            if av.queue_to_front {
                                                span { class: "text-yellow-400 mr-1", "▶" }
                                            }
                                            {av.id.map(|id| format!("#{id} ")).unwrap_or_default()}
                                            {av.label.clone()}
                                        }
                                    }
                                }
                            }
                        }
                        // Side queue
                        div { class: "flex-1 min-w-24",
                            div { class: "text-xs text-secondary-text font-medium mb-1", "Side" }
                            div { class: "bg-secondary-surface p-2 min-h-8 text-xs font-mono",
                                if debug.read().side_queue_len == 0 {
                                    span { class: "text-tertiary-text", "(empty)" }
                                } else {
                                    span { "{debug.read().side_queue_len} pending" }
                                }
                            }
                        }
                        // Normal traversal
                        div { class: "flex-1 min-w-40",
                            div { class: "text-xs text-secondary-text font-medium mb-1",
                                "Normal (idx {debug.read().normal_index}"
                                if debug.read().normal_backward {
                                    " ←rev"
                                }
                                ")"
                            }
                            div { class: "bg-secondary-surface p-2 min-h-8 text-xs font-mono flex flex-wrap gap-1",
                                for (i, av) in debug.read().normal_actions.clone().into_iter().enumerate() {
                                    span {
                                        key: "{i}",
                                        class: if i == debug.read().normal_index {
                                            "bg-primary-surface text-primary-text px-1"
                                        } else {
                                            "text-secondary-text px-1"
                                        },
                                        "{i}:{av.label}"
                                    }
                                }
                                if debug.read().normal_actions.is_empty() {
                                    span { class: "text-tertiary-text", "(empty)" }
                                }
                            }
                        }
                    }
                }

                // ② Conditions & Cooldown
                div { class: "mb-3",
                    div { class: "text-xs text-primary-text font-medium mb-1", "② Conditions & Cooldown" }
                    div { class: "bg-secondary-surface p-2",
                        if debug.read().priority_actions.is_empty() {
                            div { class: "text-xs text-tertiary-text", "(no priority actions)" }
                        } else {
                            for pav in debug.read().priority_actions.clone() {
                                div {
                                    key: "{pav.action.label}",
                                    class: "flex items-center gap-2 mb-1 text-xs",
                                    // Label
                                    span { class: "w-40 truncate font-mono", "{pav.action.label}" }
                                    // Ignoring badge
                                    span {
                                        class: if pav.ignoring {
                                            "text-yellow-400 w-16 text-center"
                                        } else {
                                            "text-tertiary-text w-16 text-center"
                                        },
                                        if pav.ignoring { "ignoring:✓" } else { "ignoring:✗" }
                                    }
                                    // Cooldown bar + label
                                    {render_cooldown(pav.cooldown_remaining_ms, pav.action.condition_kind.as_str())}
                                }
                            }
                        }
                    }
                }

                // ③ Timeline
                div { class: "mb-3",
                    div { class: "text-xs text-primary-text font-medium mb-1", "③ Timeline" }
                    div {
                        class: "bg-secondary-surface p-2 h-40 overflow-y-auto font-mono text-xs",
                        id: "rotator-timeline",
                        for (i, entry) in debug.read().timeline.clone().into_iter().enumerate() {
                            div {
                                key: "{i}",
                                class: if entry.is_warn {
                                    "text-yellow-400"
                                } else {
                                    "text-secondary-text"
                                },
                                "{entry.text}"
                            }
                        }
                    }
                }

                // ④ Player State
                div {
                    div { class: "text-xs text-primary-text font-medium mb-1", "④ Player State" }
                    div { class: "bg-secondary-surface p-2 text-xs font-mono",
                        "block: {debug.read().player_block_reason:?}"
                    }
                }
            }
        }
    }
}

fn render_cooldown(remaining_ms: Option<i64>, condition_kind: &str) -> Element {
    let Some(remaining) = remaining_ms else {
        return rsx! {
            span { class: "text-tertiary-text text-xs", "{condition_kind}" }
        };
    };

    let total_ms: i64 = if condition_kind.starts_with("EveryMillis") {
        condition_kind
            .trim_start_matches("EveryMillis(")
            .trim_end_matches(')')
            .parse()
            .unwrap_or(30_000)
    } else if condition_kind == "ErdaShowerOffCooldown" {
        20_000
    } else {
        return rsx! {
            span { class: "text-tertiary-text text-xs", "{condition_kind}" }
        };
    };

    if remaining <= 0 {
        return rsx! {
            span { class: "text-green-400 text-xs font-medium", "可排入" }
        };
    }

    let pct = ((total_ms - remaining) as f64 / total_ms as f64 * 100.0)
        .clamp(0.0, 100.0) as u32;
    let secs = remaining as f64 / 1000.0;

    rsx! {
        div { class: "flex items-center gap-1 flex-1",
            div { class: "flex-1 h-2 bg-tertiary-surface rounded overflow-hidden",
                div {
                    class: "h-full bg-primary-text rounded",
                    style: "width: {pct}%",
                }
            }
            span { class: "text-secondary-text w-12 text-right", "{secs:.1}s" }
        }
    }
}
