use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracebound::{app::AppService, scenario::ScenarioRegistry, store::Store, web};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let data_dir = app_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    let database_path = data_dir.join("tracebound.sqlite3");
    let database_url = format!("sqlite://{}?mode=rwc", database_path.display());
    let store = Store::connect(&database_url).await?;
    let scenarios = ScenarioRegistry::load_embedded()?;
    let app = web::router(AppService { store, scenarios }).layer(TraceLayer::new_for_http());

    let address: SocketAddr = std::env::var("TRACEBOUND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    let listener = TcpListener::bind(address).await?;
    let url = format!("http://{}", listener.local_addr()?);
    tracing::info!(%url, database = %database_path.display(), "Tracebound ready");
    println!("Tracebound is running at {url}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn app_data_dir() -> PathBuf {
    std::env::var_os("TRACEBOUND_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".tracebound"))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
