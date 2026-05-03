//! Overlay system — modal view stack for command palette, dialogs, etc.
//!
//! Overlays are pushed onto a stack. The topmost overlay receives input first.
//! If it doesn't consume the input, it falls through to the next layer.

pub mod kind;
mod stack;

pub use kind::{
    AgentsPanelState, ApproveApiKeyState, BackgroundTasksState, CostThresholdState,
    DiffOverlayState, DoctorCheck, DoctorState, ExportFormat, ExportState, GlobalSearchState,
    HelpState, HistorySearchState, McpPanelState, MemoryPanelState, MessageSelectorState,
    ModelPickerState, OAuthFlowState, OAuthStatus, OnboardingState, OverlayKind,
    PermissionDialogState, PermissionRulesState, SearchResult, SessionEntry, SessionPickerState,
    ThemePickerState, TranscriptState,
};
pub use stack::{Overlay, OverlayAction, OverlayStack};
