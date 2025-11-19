use axum::Router;
use crate::routes::state::RouteState;

pub mod state;
pub mod chat_thread;
pub mod chat_summary;
pub mod state_delta;
pub mod messages;

pub fn router() -> Router<RouteState> {
    Router::new()
        .merge(chat_thread::router())
        .merge(chat_summary::router())
        .merge(state_delta::router())
        .merge(messages::router())          // ← NEW CLEAN MESSAGE ROUTER
}
