//! `keinontolibrary-server` — the axum HTTP service.
//!
//! Routes:
//! - `GET  /healthz` — liveness.
//! - `GET  /about` — version, data metadata, attribution.
//! - `GET  /decline?word=&number=&case=[&hn=&tn=]` — one slot.
//! - `GET  /paradigm?word=[&hn=&tn=]` — full table.
//! - `POST /admin/add`, `POST /admin/override` — overlay mutation (bearer auth).
//!
//! The state is shared and stateless across requests; the overlay uses interior mutability
//! so admin writes are immediately visible to lookups.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use keinontolibrary_core::{Case, Engine, Error, Forms, Number, Paradigm, ParadigmRef};
use keinontolibrary_data::{Meta, Overlay};
use serde::Deserialize;
use serde_json::{json, Value};

/// Shared application state.
#[derive(Debug)]
pub struct AppState {
    /// The declension engine.
    pub engine: Engine,
    /// The overlay store (for admin mutation).
    pub overlay: Overlay,
    /// Artifact metadata.
    pub meta: Meta,
    /// Admin bearer token; when `None`, admin endpoints are disabled.
    pub admin_token: Option<String>,
}

/// Build the router for a given state.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/about", get(about))
        .route("/decline", get(decline))
        .route("/paradigm", get(paradigm))
        .route("/admin/add", post(admin_add))
        .route("/admin/override", post(admin_add))
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn about(State(st): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "service": "keinontolibrary",
        "version": st.meta.version,
        "lemmas": st.meta.n_lemmas,
        "forms": st.meta.n_forms,
        "attribution": {
            "kotus": st.meta.kotus_source,
            "voikko": st.meta.voikko_source,
            "license": "Source code MIT. Data: Kotus Nykysuomen sanalista 2024 (CC BY 4.0); \
                        see LICENSING.md for the Voikko-derived-data terms.",
        },
    }))
}

/// Query for `/decline`.
#[derive(Debug, Deserialize)]
struct DeclineQuery {
    word: String,
    number: String,
    case: String,
    hn: Option<u8>,
    tn: Option<u8>,
}

/// Query for `/paradigm`.
#[derive(Debug, Deserialize)]
struct ParadigmQuery {
    word: String,
    hn: Option<u8>,
    tn: Option<u8>,
}

async fn decline(
    State(st): State<Arc<AppState>>,
    Query(q): Query<DeclineQuery>,
) -> (StatusCode, Json<Value>) {
    let (number, case) = match parse_number_case(&q.number, &q.case) {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };
    let result = match q.tn {
        Some(tn) => st
            .engine
            .decline_with(&q.word, number, case, &ParadigmRef::new(q.hn, tn)),
        None => st.engine.decline(&q.word, number, case),
    };
    match result {
        Ok(forms) => (StatusCode::OK, Json(forms_json(&forms))),
        Err(e) => error_response(&e),
    }
}

async fn paradigm(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ParadigmQuery>,
) -> (StatusCode, Json<Value>) {
    let result = match q.tn {
        Some(tn) => st
            .engine
            .paradigm_with(&q.word, &ParadigmRef::new(q.hn, tn)),
        None => st.engine.paradigm(&q.word),
    };
    match result {
        Ok(p) => (StatusCode::OK, Json(paradigm_json(&p))),
        Err(e) => error_response(&e),
    }
}

async fn admin_add(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(entry): Json<keinontolibrary_data::OverlayEntry>,
) -> (StatusCode, Json<Value>) {
    if !authorized(&st, &headers) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "forbidden" })));
    }
    match st.overlay.append(&entry) {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

fn authorized(st: &AppState, headers: &HeaderMap) -> bool {
    let Some(expected) = &st.admin_token else {
        return false; // admin disabled when no token configured
    };
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| token == expected)
}

fn parse_number_case(
    number: &str,
    case: &str,
) -> Result<(Number, Case), (StatusCode, Json<Value>)> {
    let number = number.parse::<Number>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    let case = case.parse::<Case>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    Ok((number, case))
}

fn forms_json(forms: &Forms) -> Value {
    serde_json::to_value(forms).unwrap_or_else(|_| json!({}))
}

fn paradigm_json(p: &Paradigm) -> Value {
    serde_json::to_value(p).unwrap_or_else(|_| json!({}))
}

fn error_response(e: &Error) -> (StatusCode, Json<Value>) {
    match e {
        Error::UnknownWord(word) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "unknown_word", "word": word })),
        ),
        Error::Ambiguous { lemma, paradigms } => {
            let options: Vec<Value> = paradigms
                .iter()
                .map(|p| json!({ "tn": p.tn, "hn": p.hn, "av": p.av, "gloss": p.gloss }))
                .collect();
            (
                StatusCode::CONFLICT,
                Json(json!({ "error": "ambiguous", "lemma": lemma, "paradigms": options })),
            )
        }
        Error::DefectiveForm {
            lemma,
            number,
            case,
        } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "defective_form",
                "lemma": lemma,
                "number": number.name(),
                "case": case.name(),
            })),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt as _;
    use keinontolibrary_core::{MemoryStore, Source};
    use tower::ServiceExt as _;

    fn test_state() -> Arc<AppState> {
        let mut store = MemoryStore::new();
        store.insert(
            "talo",
            ParadigmRef::new(None, 1),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["talossa".into()], Source::Lookup),
        );
        // Mirror production wiring: the engine consults the SAME shared overlay.
        let overlay = Overlay::in_memory();
        let engine = Engine::builder()
            .lookup(Box::new(store))
            .overlay(Box::new(overlay.clone()))
            .build();
        Arc::new(AppState {
            engine,
            overlay,
            meta: Meta {
                version: "test".into(),
                ..Meta::default()
            },
            admin_token: Some("secret".into()),
        })
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    }

    #[tokio::test]
    async fn healthz_ok() {
        let resp = app(test_state())
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn decline_returns_form() {
        let resp = app(test_state())
            .oneshot(
                Request::get("/decline?word=talo&number=singular&case=inessive")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["variants"][0], "talossa");
    }

    #[tokio::test]
    async fn unknown_word_is_404() {
        let resp = app(test_state())
            .oneshot(
                Request::get("/decline?word=zzz&number=singular&case=inessive")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn bad_case_is_400() {
        let resp = app(test_state())
            .oneshot(
                Request::get("/decline?word=talo&number=singular&case=sisaolento")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn admin_requires_token() {
        let body = r#"{"lemma":"blorko","tn":1,"number":"singular","case":"inessive","variants":["blorkossa"]}"#;
        // Without auth -> 403.
        let resp = app(test_state())
            .oneshot(
                Request::post("/admin/add")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // With the right bearer token -> 200, and then the form is declinable.
        let state = test_state();
        let resp = app(state.clone())
            .oneshot(
                Request::post("/admin/add")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let f = state
            .engine
            .decline("blorko", Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("blorkossa"));
    }
}
