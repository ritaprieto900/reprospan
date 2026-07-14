use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path as AxumPath, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use reprospan_core::Bundle;
use reprospan_store::{Store, StoreError};
use serde::Serialize;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    api_version: &'static str,
    contract_version: &'static str,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

pub fn router(store: Store) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/bundles", get(list_bundles))
        .route("/v1/bundles/ingest", post(ingest))
        .route("/v1/bundles/{bundle_id}/timeline", get(timeline))
        .route("/v1/artifacts/{sha256}", put(put_artifact))
        .layer(CorsLayer::permissive())
        .with_state(AppState {
            store: Arc::new(Mutex::new(store)),
        })
}

/// Serves the local API on a loopback address.
///
/// # Errors
///
/// Returns [`ServeError`] when the address is not loopback, the store cannot open, binding
/// fails, or the HTTP server exits with an I/O error.
pub async fn serve(database: impl AsRef<Path>, listen: SocketAddr) -> Result<(), ServeError> {
    if !listen.ip().is_loopback() {
        return Err(ServeError::NonLoopback(listen.ip()));
    }
    let store = Store::open_and_migrate(database)?;
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, router(store)).await?;
    Ok(())
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        api_version: "v1",
        contract_version: "reprospan.bundle.v1",
    })
}

async fn list_bundles(
    State(state): State<AppState>,
) -> Response {
    let result = {
        let store = state.store.lock().unwrap();
        store.list_bundles()
    };
    match result {
        Ok(bundles) => Json(bundles).into_response(),
        Err(store_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            store_error.to_string(),
        ),
    }
}

async fn ingest(
    State(state): State<AppState>,
    payload: Result<Json<Bundle>, JsonRejection>,
) -> Response {
    let Json(bundle) = match payload {
        Ok(bundle) => bundle,
        Err(rejection) => {
            return error(
                StatusCode::BAD_REQUEST,
                "invalid_json",
                rejection.body_text(),
            );
        }
    };

    let result = {
        let Ok(mut store) = state.store.lock() else {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "internal storage error",
            );
        };
        store.import_bundle(&bundle)
    };

    match result {
        Ok(()) => (StatusCode::CREATED, Json(bundle)).into_response(),
        Err(StoreError::Conflict(_)) => error(
            StatusCode::CONFLICT,
            "bundle_exists",
            format!("bundle already exists: {}", bundle.bundle_id),
        ),
        Err(StoreError::InvalidBundle(source)) => error(
            StatusCode::BAD_REQUEST,
            "invalid_bundle",
            source.to_string(),
        ),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "internal storage error",
        ),
    }
}

async fn timeline(
    State(state): State<AppState>,
    AxumPath(bundle_id): AxumPath<String>,
) -> Response {
    let result = {
        let Ok(store) = state.store.lock() else {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "internal storage error",
            );
        };
        store.timeline(&bundle_id)
    };

    match result {
        Ok(bundle) => Json(bundle).into_response(),
        Err(StoreError::NotFound(_)) => error(
            StatusCode::NOT_FOUND,
            "bundle_not_found",
            format!("bundle not found: {bundle_id}"),
        ),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "internal storage error",
        ),
    }
}

async fn put_artifact(
    State(state): State<AppState>,
    AxumPath(sha256): AxumPath<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let media_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream");

    let result = {
        let store = state.store.lock().unwrap();
        store.store_artifact(&sha256, media_type, &body)
    };

    match result {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(store_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            store_error.to_string(),
        ),
    }
}

fn error(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorBody {
            code,
            message: message.into(),
        }),
    )
        .into_response()
}

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("listen address must be loopback, got {0}")]
    NonLoopback(IpAddr),
    #[error("store failed to open")]
    Store(#[from] StoreError),
    #[error("server I/O failed")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    const BUNDLE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/failed-tool-run.bundle.json"
    ));

    fn app() -> (tempfile::TempDir, Router) {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let store = Store::open_and_migrate(directory.path().join("server.sqlite"))
            .expect("store should open");
        (directory, router(store))
    }

    #[tokio::test]
    async fn health_ingest_and_timeline_flow() {
        let (_directory, app) = app();
        let health = app
            .clone()
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);

        let ingest = app
            .clone()
            .oneshot(
                Request::post("/v1/bundles/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(BUNDLE))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ingest.status(), StatusCode::CREATED);

        let timeline = app
            .oneshot(
                Request::get("/v1/bundles/bundle_support_refund_001/timeline")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(timeline.status(), StatusCode::OK);
        let body = to_bytes(timeline.into_body(), usize::MAX).await.unwrap();
        let bundle: Bundle = serde_json::from_slice(&body).unwrap();
        assert_eq!(bundle.events.len(), 4);
        assert_eq!(bundle.events[0].sequence, 0);
        assert_eq!(bundle.events[3].sequence, 3);
    }

    #[tokio::test]
    async fn invalid_duplicate_and_unknown_requests_have_stable_statuses() {
        let (_directory, app) = app();
        let invalid = app
            .clone()
            .oneshot(
                Request::post("/v1/bundles/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from("{"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

        for expected in [StatusCode::CREATED, StatusCode::CONFLICT] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/v1/bundles/ingest")
                        .header("content-type", "application/json")
                        .body(Body::from(BUNDLE))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }

        let missing = app
            .oneshot(
                Request::get("/v1/bundles/missing/timeline")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn public_listen_address_is_rejected() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let error = serve(
            directory.path().join("server.sqlite"),
            "0.0.0.0:8787".parse().unwrap(),
        )
        .await
        .expect_err("public bind should be rejected");
        assert!(matches!(error, ServeError::NonLoopback(_)));
    }
}
