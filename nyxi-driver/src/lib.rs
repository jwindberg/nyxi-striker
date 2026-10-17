pub mod config;
pub mod db;
pub mod device_manager;
pub mod grpc_service;
pub mod proto;
pub mod system;

use config::SupportConfig;
use db::DbManager;
use device_manager::DeviceManager;
use grpc_service::DriverGrpcService;
use proto::driver::driver_grpc_server::DriverGrpcServer;
use std::net::SocketAddr;
use std::sync::Arc;
use system::SystemMonitor;
use tonic::transport::Server;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("/tmp/nyxi_driver.log")?;
    let file_layer = tracing_subscriber::fmt::layer().with_writer(file).with_ansi(false);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer)
        .init();

    tracing::info!("Starting NYXI cross-platform driver (iot_driver)...");

    // Load device config
    let config = SupportConfig::load_or_default();

    // Initialize subsystems
    let db = DbManager::new();
    let device_mgr = Arc::new(DeviceManager::new(config)?);
    let system_monitor = Arc::new(SystemMonitor::new());

    // Start background hardware polling loop
    device_mgr.clone().start_monitor_loop();

    // Build gRPC-Web service
    let driver_service = DriverGrpcService::new(db, device_mgr, system_monitor);
    let grpc_server = DriverGrpcServer::new(driver_service);

    // Enable gRPC-Web (allows grpc-web clients from Electron / Browser to connect)
    let grpc_web_layer = tonic_web::enable(grpc_server);

    // Permissive CORS for Electron renderer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    let addr: SocketAddr = "127.0.0.1:3814".parse()?;
    tracing::info!("NYXI driver listening on http://{} (gRPC & gRPC-Web)", addr);

    Server::builder()
        .accept_http1(true)
        .layer(cors)
        .add_service(grpc_web_layer)
        .serve(addr)
        .await?;

    Ok(())
}
