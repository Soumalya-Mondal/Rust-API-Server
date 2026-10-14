// Imports Axum types for the route, JSON body, and status code.
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
// Imports Serialize so the response struct can be converted to JSON.
use serde::Serialize;

// JSON returned by the root endpoint.
#[derive(Serialize)]
struct HomeResponse {
    // HTTP status code included in the body.
    status: u16,
    // Welcome text returned to the caller.
    message: String,
}

// Builds the router containing the GET / route.
pub fn router() -> Router {
    // Maps the root path / to home_route_function.
    Router::new().route("/", get(home_route_function))
}

// Return a simple response confirming that the API is reachable.
async fn home_route_function() -> impl IntoResponse {
    // Returns a status code and JSON body tuple, which Axum converts into the HTTP response.
    (
        // HTTP 200 OK status for the response.
        StatusCode::OK,
        // Wraps the body struct so Axum serializes it as JSON.
        Json(HomeResponse {
            // Numeric value of the 200 OK status, placed in the body.
            status: StatusCode::OK.as_u16(),
            // Welcome text for the root endpoint.
            message: "This Is Rust And AUXM Based API-Server!".to_string(),
        }),
    )
}
