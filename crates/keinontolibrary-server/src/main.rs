//! `keinontolibrary-server` binary — loads the artifact + overlay and serves HTTP.
//!
//! Configuration via environment:
//! - `KEINONTO_ARTIFACT` (default `data/artifact/keinontolibrary.bin`)
//! - `KEINONTO_OVERLAY`  (default `data/overlay.jsonl`)
//! - `KEINONTO_ADDR`     (default `0.0.0.0:8080`)
//! - `KEINONTO_ADMIN_TOKEN` (admin endpoints are disabled if unset)

use std::sync::Arc;

use keinontolibrary_data::build_engine;
use keinontolibrary_server::{app, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structured logging; level via RUST_LOG (default info), tower-http traces requests.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let artifact = std::env::var("KEINONTO_ARTIFACT")
        .unwrap_or_else(|_| "data/artifact/keinontolibrary.bin".to_owned());
    let overlay_path =
        std::env::var("KEINONTO_OVERLAY").unwrap_or_else(|_| "data/overlay.jsonl".to_owned());
    let addr = std::env::var("KEINONTO_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let admin_token = std::env::var("KEINONTO_ADMIN_TOKEN").ok();

    let bundle = build_engine(&artifact, &overlay_path)
        .map_err(|e| format!("loading artifact {artifact}: {e}"))?;
    tracing::info!(
        lemmas = bundle.meta.n_lemmas,
        forms = bundle.meta.n_forms,
        version = %bundle.meta.version,
        "artifact loaded"
    );
    if admin_token.is_none() {
        tracing::warn!("KEINONTO_ADMIN_TOKEN unset — admin endpoints disabled");
    }

    let state = Arc::new(AppState {
        engine: bundle.engine,
        overlay: bundle.overlay,
        meta: bundle.meta,
        admin_token,
    });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("shut down cleanly");
    Ok(())
}

/// Resolve when the process receives SIGINT (Ctrl-C) or SIGTERM (container stop), so
/// axum drains in-flight requests before exiting.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("shutdown signal received, draining");
}
