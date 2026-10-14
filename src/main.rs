// Declares the controllers module, loaded from src/controllers/mod.rs.
mod controllers;

// Imports the Axum type used to build the router.
use axum::Router;

// Starts the Tokio async runtime and makes main the async entry point.
#[tokio::main]
// Entry point; returns Ok(()) once the server stops.
async fn main() -> std::io::Result<()> {
    // Creates an empty router that the controller routes are merged into.
    let db_app = Router::new()
        // Registers the root (/) route.
        .merge(controllers::home_route::router())
        // Registers the /dbhealth route.
        .merge(controllers::db_health_route::router())
        // Registers the /serverhealth route.
        .merge(controllers::server_health_route::router())
        // Registers the /incidenttickets route.
        .merge(controllers::incident_tickets_route::router())
        // Registers the /servicedesktickets route.
        .merge(controllers::service_desk_ticket_route::router())
        // Registers the /servicerequesttickets route.
        .merge(controllers::service_request_ticket_route::router());

    // Creates a TCP listener bound to localhost port 3000.
    let db_app_listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        // Waits asynchronously until the socket is bound.
        .await
        // Panics and exits if the port is already in use or cannot be bound.
        .unwrap();

    // Prints the listen address to the console.
    println!("Server Listening On 127.0.0.1:3000");

    // Serves the router on the listener; keeps running until the process is stopped.
    axum::serve(db_app_listener, db_app).await.unwrap();

    // Reached only after serving ends; returns success to the caller.
    Ok(())
}
