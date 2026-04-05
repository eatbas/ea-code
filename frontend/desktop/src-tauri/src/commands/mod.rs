pub(crate) mod api_health;
pub(crate) mod cli;
pub(crate) mod emitter;
pub(crate) mod notifications;
pub(crate) mod power;
pub(crate) mod settings;
pub(crate) mod workspace;

use crate::sidecar::SidecarManager;

/// Shared application state.
pub struct AppState {
    pub sidecar: SidecarManager,
}
