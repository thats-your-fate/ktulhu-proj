use axum::Router;
use crate::routes::state::RouteState;

pub mod by_device;

pub fn router() -> Router<RouteState> {
    Router::new()
        .merge(by_device::router())
}
