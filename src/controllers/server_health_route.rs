// Imports Axum types for the route, JSON body, and status code.
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
// Imports Local to read the server's current local time.
use chrono::Local;
// Imports Serialize so the response struct can be converted to JSON.
use serde::Serialize;

// JSON returned by the server health endpoint.
#[derive(Serialize)]
struct HealthResponse {
    // HTTP status code included in the body.
    status: u16,
    // Health message, "OK" when the server responds.
    message: String,
    // Current local time in RFC 3339 format.
    datetime: String,
}

// Builds the router containing the GET /serverhealth route.
pub fn router() -> Router {
    // Maps GET /serverhealth to server_health_function.
    Router::new().route("/serverhealth", get(server_health_function))
}

// Return the current server status and local timestamp.
async fn server_health_function() -> impl IntoResponse {
    // Returns a status code and JSON body tuple, which Axum converts into the HTTP response.
    (
        // HTTP 200 OK status for the response.
        StatusCode::OK,
        // Wraps the body struct so Axum serializes it as JSON.
        Json(HealthResponse {
            // Numeric value of the 200 OK status, placed in the body.
            status: StatusCode::OK.as_u16(),
            // Fixed health message.
            message: "OK".to_string(),
            // Current local time as an RFC 3339 string.
            datetime: Local::now().to_rfc3339(),
        }),
    )
}
