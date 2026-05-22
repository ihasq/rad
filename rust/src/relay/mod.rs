pub mod state;
pub mod participants;
pub mod operations;
pub mod accept;
pub mod read;

use axum::{Router, routing::{get, post}};
use state::SharedState;

pub fn create_relay_router(state: SharedState) -> Router {
    Router::new()
        .route("/rad/participants", post(participants::join).get(participants::list))
        .route("/rad/operations", post(operations::submit))
        .route("/rad/operations/:id/status", get(operations::status))
        .route("/rad/operations/:id", get(operations::detail))
        .route("/rad/accept", post(accept::accept))
        .route("/rad/visible/*path", get(read::visible))
        .route("/rad/files/*path", get(read::file))
        .route("/rad/files", get(read::file_list))
        .route("/rad/regions/*path", get(read::regions))
        .route("/rad/log", get(read::log))
        .route("/rad/compact", post(read::compact))
        .route("/rad/sync/git", post(read::sync_git))
        .with_state(state)
}
