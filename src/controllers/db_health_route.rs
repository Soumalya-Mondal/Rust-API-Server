// Imports Axum types for the route, JSON body, and status code.
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
// Imports Serialize so the health response can be converted to JSON.
use serde::Serialize;
// Imports Row, which enables row.get() on query results, plus Postgres connection types.
use sqlx::{
    // Row trait that provides row.get() for reading column values.
    Row,
    // Connection options, pool builder, and SSL mode for PostgreSQL.
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
// Imports the standard environment variable functions.
use std::env;

// Keep the database health result in the JSON response without exposing credentials.
#[derive(Serialize)]
struct DatabaseHealthResponse {
    // HTTP status code included in the body.
    status: u16,
    // Health result text: "Ok" or "Error".
    incidentdb: &'static str,
}

// Holds the PostgreSQL connection settings read from the environment.
pub struct DatabaseConfig {
    // Database user name.
    pub user: String,
    // Database password.
    pub password: String,
    // Database server host name or address.
    pub host: String,
    // Database port as text, parsed later.
    pub port: String,
    // Name of the incident database.
    pub incident_database_name: String,
}

// Reads the database settings from environment variables, loading .env first if present.
pub fn load_database_config() -> Result<DatabaseConfig, env::VarError> {
    // Loads .env into the process environment; a missing file is ignored.
    dotenvy::dotenv().ok();

    // Reads each required variable and fails with VarError if any is missing.
    Ok(DatabaseConfig {
        // Reads the database user name.
        user: env::var("PROD_DATABASE_USER")?,
        // Reads the database password.
        password: env::var("PROD_DATABASE_PASSWORD")?,
        // Reads the database host.
        host: env::var("PROD_DATABASE_HOST")?,
        // Reads the database port.
        port: env::var("PROD_DATABASE_PORT")?,
        // Reads the incident database name.
        incident_database_name: env::var("PROD_INCIDENT_DATABASE_NAME")?,
    })
}

// Builds the router containing the GET /dbhealth route.
pub fn router() -> Router {
    // Maps GET /dbhealth to db_health_route_function.
    Router::new().route("/dbhealth", get(db_health_route_function))
}

// Checks the incident database and returns its status as JSON.
async fn db_health_route_function() -> impl IntoResponse {
    // Loads settings from the environment; returns 500 if any variable is missing.
    let database_config = match load_database_config() {
        // Uses the loaded settings when every variable is present.
        Ok(database_config) => database_config,
        // Stops early with 500 when a variable is missing.
        Err(_) => {
            // A missing environment variable prevents any database check.
            return (
                // HTTP 500 status for a configuration failure.
                StatusCode::INTERNAL_SERVER_ERROR,
                // Body reporting the failure without contacting the database.
                Json(DatabaseHealthResponse {
                    // Numeric 500 code placed in the body.
                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    // Marks the database check as failed.
                    incidentdb: "Error",
                }),
            );
        }
    };

    // Runs the connection check against the incident database.
    let is_healthy =
        check_database_connection(&database_config, &database_config.incident_database_name).await;
    // Converts the boolean result into the "Ok" or "Error" text used in the JSON body.
    let db = if is_healthy { "Ok" } else { "Error" };

    // Uses 200 OK when healthy and 503 Service Unavailable otherwise.
    let status = if is_healthy {
        // Healthy: 200 OK.
        StatusCode::OK
    } else {
        // Unhealthy: 503 Service Unavailable.
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        // Status chosen above, returned as the HTTP status line.
        status,
        // Body containing the numeric status and the health text.
        Json(DatabaseHealthResponse {
            // Numeric value of the chosen status.
            status: status.as_u16(),
            // "Ok" or "Error" from the check.
            incidentdb: db,
        }),
    )
}

// Open an encrypted Azure PostgreSQL connection and execute a lightweight query.
async fn check_database_connection(database_config: &DatabaseConfig, database_name: &str) -> bool {
    // Reject an invalid port before attempting to create a database connection.
    let port = match database_config.port.parse::<u16>() {
        // Uses the parsed port number when the string is valid.
        Ok(port) => port,
        // Reports unhealthy when the port is not a valid number.
        Err(_) => return false,
    };

    // Build connection options directly so special characters in the password
    // do not need URL encoding.
    let connection_config = PgConnectOptions::new()
        // Sets the database server host name or address.
        .host(&database_config.host)
        // Sets the TCP port parsed above.
        .port(port)
        // Sets the database user name.
        .username(&database_config.user)
        // Sets the password, passed as-is without URL encoding.
        .password(&database_config.password)
        // Selects the database to connect to.
        .database(database_name)
        // Requires an encrypted TLS connection.
        .ssl_mode(PgSslMode::Require);

    // Use a one-connection pool because this endpoint only needs a health probe.
    let connection_pool = match PgPoolOptions::new()
        // Limits the pool to a single connection.
        .max_connections(1)
        // Opens the connection using the options above.
        .connect_with(connection_config)
        .await
    {
        // Uses the pool when the connection succeeds.
        Ok(connection_pool) => connection_pool,
        // Reports unhealthy when the connection cannot be opened.
        Err(_) => return false,
    };

    // Runs SELECT 1 on the pool; any database error is treated as unhealthy.
    sqlx::query("SELECT 1")
        // Sends the query and expects exactly one row back.
        .fetch_one(&connection_pool)
        .await
        // Returns true only when the single returned value equals 1.
        .map(|row| row.get::<i32, _>(0) == 1)
        // Converts a query error into false.
        .unwrap_or(false)
}
