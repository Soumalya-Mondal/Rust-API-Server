// Imports Axum routing, extractors, body, and response types.
use axum::{
    // Router builds the route table for this controller.
    Router,
    // Body is the response body type used to wrap the JSON text.
    body::Body,
    // Query extracts the limit and field parameters from the URL.
    extract::Query,
    // StatusCode for HTTP status values; CONTENT_TYPE for the response header name.
    http::{StatusCode, header::CONTENT_TYPE},
    // IntoResponse converts status codes; Response is the full HTTP response type.
    response::{IntoResponse, Response},
    // get builds a GET route handler.
    routing::get,
};
// Imports Deserialize so query strings can be parsed into a struct.
use serde::Deserialize;
// Imports the generic JSON value type used for row data.
use serde_json::Value;
// Imports Row for row.get(), Postgres connection types, and the Json wrapper for decoding.
use sqlx::{
    // Row trait that provides row.get() for reading column values.
    Row,
    // Connection options, pool builder, and SSL mode for PostgreSQL.
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    // Json wrapper that decodes a JSON column into a serde value.
    types::Json,
};
// Imports the standard environment variable functions.
use std::env;

// Holds the PostgreSQL connection settings shared by this route.
struct DatabaseConfig {
    // Database user name.
    user: String,
    // Database password.
    password: String,
    // Database server host name or address.
    host: String,
    // Database port as text, parsed later.
    port: String,
}

// Query parameters: an optional row limit and an optional comma-separated column list.
#[derive(Deserialize)]
struct ServiceRequestTicketsQuery {
    // Maximum number of rows to return; absent means no limit.
    limit: Option<u32>,
    // Comma-separated column names; absent means all allowed columns.
    field: Option<String>,
}

// Allowlist of column names clients may request; only these names are ever placed in the SQL.
const SERVICE_REQUEST_COLUMNS: &[&str] = &[
    "ticket_number",
    "account_name",
    "ticket_type",
    "ticket_created_day",
    "ticket_created_month",
    "ticket_created_year",
    "ticket_created_week",
    "ticket_created_hours",
    "ticket_created_minutes",
    "ticket_created_seconds",
    "created_at",
    "ticket_opened_day",
    "ticket_opened_month",
    "ticket_opened_year",
    "ticket_opened_week",
    "ticket_opened_hours",
    "ticket_opened_minutes",
    "ticket_opened_seconds",
    "opened_at",
    "cmdb_ci",
    "cmdb_ci_validity_status",
    "cmdb_ci_type",
    "assigned_to",
    "catalogue_item",
    "parent",
    "state",
    "priority",
    "category",
    "sub_category",
    "channel",
    "assignment_group",
    "assignment_group_category",
    "request_item",
    "reassignment_count",
    "reassignment_bucket",
    "sla",
    "sla_breached",
    "kb_present",
    "kb_attached",
    "kb_referred",
    "attached_kb_number",
    "referred_kb_number",
    "kb_number",
    "ticket_closed_day",
    "ticket_closed_month",
    "ticket_closed_year",
    "ticket_closed_week",
    "ticket_closed_hours",
    "ticket_closed_minutes",
    "ticket_closed_seconds",
    "closed_at",
    "ticket_resolved_day",
    "ticket_resolved_month",
    "ticket_resolved_year",
    "ticket_resolved_week",
    "ticket_resolved_hours",
    "ticket_resolved_minutes",
    "ticket_resolved_seconds",
    "resolved_at",
    "resolved_by",
    "resolved_type",
    "short_description",
    "translated_short_description",
    "description",
    "translated_description",
    "work_notes",
    "translated_work_notes",
    "resolution_notes",
    "translated_resolution_notes",
    "request_language",
    "keywords_1",
    "keywords_2",
    "normalized_keywords_1",
    "normalized_keywords_2",
    "ticket_summary",
    "automation_probability",
    "automation_availability",
    "automation_id",
    "ticket_mttr_days",
    "ticket_mttr_hours",
    "ticket_mttr_minutes",
    "ticket_mttr_seconds",
    "ticket_mttr_bucket",
    "ticket_aging_days",
    "ticket_aging_hours",
    "ticket_aging_minutes",
    "ticket_aging_seconds",
    "ticket_aging_bucket",
    "cancelled_request",
    "short_duration_request",
    "backup_check_pattern",
    "url_allowlist_pattern",
    "port_opening_pattern",
    "firewall_rule_pattern",
    "vpn_access_pattern",
    "proxy_allowlist_pattern",
    "folder_permission_pattern",
    "parent_child_request",
    "duplicate_request",
    "deduplicate_request",
    "correlated_request",
    "eso_analysis",
    "eso_category",
    "final_category",
];

// Error kinds that the handler maps to HTTP 400 or 500.
enum ServiceRequestTicketsError {
    // A requested column is empty or not in the allowlist.
    InvalidField,
    // Configuration, connection, or query failed.
    Database,
}

// Builds the comma-separated SELECT column list from the optional field parameter.
fn selected_columns(field: Option<&str>) -> Result<String, ServiceRequestTicketsError> {
    // Splits the field parameter into trimmed names; stays empty when no field is given.
    let requested = match field {
        // Splits the comma-separated list and trims spaces around each name.
        Some(field) => field.split(',').map(str::trim).collect::<Vec<_>>(),
        // No field parameter given, so the requested list is empty.
        None => Vec::new(),
    };

    // Rejects the request if any name is empty or not in the allowlist.
    if field.is_some()
        // Fails when the list is empty or contains an unknown name.
        && (requested.is_empty()
            || requested
                .iter()
                .any(|column| column.is_empty() || !SERVICE_REQUEST_COLUMNS.contains(column)))
    {
        // Signals an invalid field list, mapped to HTTP 400.
        return Err(ServiceRequestTicketsError::InvalidField);
    }

    // Keeps the requested columns in allowlist order, not in the order the client sent them.
    let columns = if field.is_some() {
        // Walks the allowlist so the output order is fixed.
        SERVICE_REQUEST_COLUMNS
            .iter()
            // Keeps only the allowlist entries the client asked for.
            .filter(|column| requested.contains(column))
            // Copies each &&str into a &str.
            .copied()
            // Gathers the kept names into a Vec.
            .collect::<Vec<_>>()
    } else {
        // Uses every allowed column when no field parameter is given.
        SERVICE_REQUEST_COLUMNS.to_vec()
    };

    // Joins the column names with commas for the SQL text.
    Ok(columns.join(", "))
}

// Reads the database settings from the environment, loading .env first.
fn load_database_config() -> Result<DatabaseConfig, env::VarError> {
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
    })
}

// Builds the router containing the GET /servicerequesttickets route.
pub fn router() -> Router {
    // Maps GET /servicerequesttickets to the handler below.
    Router::new().route(
        "/servicerequesttickets",
        get(service_request_route_function),
    )
}

// Axum handler that runs the query and maps the result to a response.
async fn service_request_route_function(
    Query(query): Query<ServiceRequestTicketsQuery>,
) -> Response {
    // Converts the u32 limit to i64, defaulting to 1000 rows when omitted.
    let limit = Some(query.limit.map(i64::from).unwrap_or(1000));

    // Runs the query and maps its result to a status code.
    match load_service_request_tickets(limit, query.field.as_deref()).await {
        // Returns the JSON array with 200 OK and a JSON content type.
        Ok(ticket_json) => Response::builder()
            // Sets the HTTP status to 200 OK.
            .status(StatusCode::OK)
            // Declares the body as JSON.
            .header(CONTENT_TYPE, "application/json")
            // Uses the JSON text as the response body.
            .body(Body::from(ticket_json))
            // Safe to unwrap: the builder inputs above are always valid.
            .unwrap(),
        // Returns 400 when the requested field list is invalid.
        Err(ServiceRequestTicketsError::InvalidField) => StatusCode::BAD_REQUEST.into_response(),
        // Returns 500 when configuration, connection, or the query fails.
        Err(ServiceRequestTicketsError::Database) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Runs the service request query and returns the matching ticket records as JSON text.
async fn load_service_request_tickets(
    // Optional maximum number of rows to return.
    limit: Option<i64>,
    // Optional comma-separated list of column names.
    field: Option<&str>,
) -> Result<String, ServiceRequestTicketsError> {
    // Validates the field list first, then loads the database settings.
    let selected_columns = selected_columns(field)?;
    // Loads database settings from the environment; any failure maps to a Database error.
    let database_config =
        load_database_config().map_err(|_| ServiceRequestTicketsError::Database)?;
    // Builds an encrypted PostgreSQL connection; the password is passed directly, not in a URL.
    let connection_config = PgConnectOptions::new()
        // Sets the database server host name or address.
        .host(&database_config.host)
        // Parses the port string from the environment; a non-numeric value is a Database error.
        .port(
            database_config
                .port
                .parse::<u16>()
                .map_err(|_| ServiceRequestTicketsError::Database)?,
        )
        // Sets the database user name.
        .username(&database_config.user)
        // Sets the password, passed as-is without URL encoding.
        .password(&database_config.password)
        // Unlike the incident route, this database name is hardcoded rather than read from the environment.
        .database("prism_analytics_sr_db")
        // Requires an encrypted TLS connection.
        .ssl_mode(PgSslMode::Require);

    // Uses a single connection because this endpoint performs one read-only request.
    let connection_pool = PgPoolOptions::new()
        // Limits the pool to a single connection.
        .max_connections(1)
        // Opens the connection using the options above.
        .connect_with(connection_config)
        .await
        // Maps a connection failure to a Database error.
        .map_err(|_| ServiceRequestTicketsError::Database)?;

    // Starts the SQL: each row is wrapped with row_to_json so it comes back as one JSON object.
    let mut query_builder = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        // Opening SQL text: selects each row as JSON from a subquery.
        "SELECT row_to_json(ticket) AS ticket FROM (SELECT ",
    );
    // Appends the validated column list and the table name, ordered by id.
    query_builder
        // Adds the validated column list.
        .push(selected_columns)
        // Adds the source table and ordering clause.
        .push(" FROM processed_sr_data ORDER BY id");
    // Adds LIMIT with a bound parameter only when a limit was supplied.
    if let Some(limit) = limit {
        // Appends LIMIT and binds the value as a parameter, not as text.
        query_builder.push(" LIMIT ").push_bind(limit);
    }
    // Closes the subquery and gives it the alias ticket.
    query_builder.push(") AS ticket");

    // Executes the built query and loads every returned row into memory.
    let rows = query_builder
        // Converts the builder into an executable query.
        .build()
        // Runs the query and collects all rows.
        .fetch_all(&connection_pool)
        .await
        // Maps a query failure to a Database error.
        .map_err(|_| ServiceRequestTicketsError::Database)?;

    // Extracts each row's JSON object into a Vec of values.
    let tickets: Vec<Value> = rows
        // Consumes the rows one at a time.
        .into_iter()
        // Reads the ticket column as JSON and keeps the inner value.
        .map(|row| row.get::<Json<Value>, _>("ticket").0)
        // Gathers the values into the Vec.
        .collect();

    // Serializes the array of ticket objects into the JSON text returned to the client.
    serde_json::to_string(&tickets).map_err(|_| ServiceRequestTicketsError::Database)
}
