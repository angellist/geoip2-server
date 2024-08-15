use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use maxminddb::geoip2;
use std::{net::IpAddr, path::PathBuf, str::FromStr, sync::Arc};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{info, Level};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone, Copy, Debug, PartialEq)]
enum LookupError {
    IpAddressInvalid,
    IpAddressRequired,
    IpAddressNotFound,
    IpAddressReserved,
}

impl IntoResponse for LookupError {
    fn into_response(self) -> Response {
        let (status, code, msg) = match self {
            LookupError::IpAddressInvalid => (StatusCode::BAD_REQUEST, "IP_ADDRESS_INVALID", "You have not supplied a valid IPv4 or IPv6 address."),
            LookupError::IpAddressRequired => (StatusCode::BAD_REQUEST, "IP_ADDRESS_REQUIRED", "You have not supplied an IP address, which is a required field."),
            LookupError::IpAddressNotFound => (StatusCode::NOT_FOUND, "IP_ADDRESS_NOT_FOUND", "The supplied IP address is not in the database."),
            LookupError::IpAddressReserved => (StatusCode::BAD_REQUEST, "IP_ADDRESS_RESERVED", "You have supplied an IP address which belongs to a reserved or private range."),
        };

        (status, Json(serde_json::json!({ "code": code, "error": msg }))).into_response()
    }
}

async fn city(State(maxmind): State<Arc<maxminddb::Reader<maxminddb::Mmap>>>, Path(ip): Path<String>) -> Result<(StatusCode, Json<serde_json::Value>), LookupError> {
    let ip = IpAddr::from_str(&ip).map_err(|_| LookupError::IpAddressInvalid)?;
    let city: geoip2::City = maxmind.lookup(ip).map_err(|_| LookupError::IpAddressNotFound)?;
    let city = serde_json::to_value(city).unwrap();

    Ok((StatusCode::OK, Json(city)))
}

async fn country(State(maxmind): State<Arc<maxminddb::Reader<maxminddb::Mmap>>>, Path(ip): Path<String>) -> Result<(StatusCode, Json<serde_json::Value>), LookupError> {
    let ip = IpAddr::from_str(&ip).map_err(|_| LookupError::IpAddressInvalid)?;
    let country: geoip2::Country = maxmind.lookup(ip).map_err(|_| LookupError::IpAddressNotFound)?;
    let country = serde_json::to_value(country).unwrap();

    Ok((StatusCode::OK, Json(country)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = clap::Command::new("geoip2-server")
        .bin_name("geoip2-server")
        .version(env!("CARGO_PKG_VERSION"))
        .propagate_version(true)
        .arg(clap::Arg::new("bind").value_name("BIND").env("BIND").long("bind").short('b').global(true).default_value("0.0.0.0"))
        .arg(clap::Arg::new("port").value_name("PORT").env("PORT").long("port").short('p').global(true).default_value("3000").value_parser(clap::value_parser!(u16)))
        .arg(clap::Arg::new("db").value_name("DB").env("DB").long("database").short('d').global(true).required(true))
        .get_matches();

    let bind = args.get_one::<String>("bind").expect("No valid bind address set!");
    let port = args.get_one::<u16>("port").expect("No valid port set!");
    let db = args.get_one::<String>("db").expect("No valid database set!");
    let db = PathBuf::from_str(db).expect("Invalid database path!");
    db.try_exists().expect("Database file does not exist!");

    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().json()).with(filter::Targets::new().with_default(Level::INFO)).init();

    let reader = maxminddb::Reader::open_mmap(db)?;

    let app = Router::new()
        .route("/geoip/v2.1/city/:ip", get(city))
        .route("/geoip/v2.1/country/:ip", get(country))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO).latency_unit(LatencyUnit::Micros)),
        )
        .route("/status", get(|| async { "ok" }))
        .with_state(Arc::new(reader));

    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}")).await?;
    info!("listening on {bind}:{port}...");

    Ok(axum::serve(listener, app).await?)
}
