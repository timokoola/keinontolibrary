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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = std::env::var("KEINONTO_ARTIFACT")
        .unwrap_or_else(|_| "data/artifact/keinontolibrary.bin".to_owned());
    let overlay_path =
        std::env::var("KEINONTO_OVERLAY").unwrap_or_else(|_| "data/overlay.jsonl".to_owned());
    let addr = std::env::var("KEINONTO_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let admin_token = std::env::var("KEINONTO_ADMIN_TOKEN").ok();

    let bundle = build_engine(&artifact, &overlay_path)
        .map_err(|e| format!("loading artifact {artifact}: {e}"))?;
    eprintln!(
        "loaded {} lemmas / {} forms (v{})",
        bundle.meta.n_lemmas, bundle.meta.n_forms, bundle.meta.version
    );
    if admin_token.is_none() {
        eprintln!("note: KEINONTO_ADMIN_TOKEN unset — admin endpoints disabled");
    }

    let state = Arc::new(AppState {
        engine: bundle.engine,
        overlay: bundle.overlay,
        meta: bundle.meta,
        admin_token,
    });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("listening on http://{addr}");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
