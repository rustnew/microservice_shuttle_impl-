use actix_web::{
    web, App, HttpRequest, HttpResponse, Responder, Result,
    http::{header, StatusCode},
};
use actix_web::web::ServiceConfig;
use shuttle_actix_web::ShuttleActixWeb;
use serde::{Deserialize, Serialize};
use shuttle_runtime::SecretStore;
use actix_web::middleware::Next;
use sqlx::{PgPool, FromRow};

#[derive(Serialize, Deserialize, FromRow)]
struct AnalyticsEvent {
    event_type: String,
    post_id: Option<i32>,
    data: serde_json::Value,
}

#[derive(Serialize, FromRow)]
struct EventStats {
    event_type: String,
    count: Option<i64>,
}

#[derive(Clone)]
struct AppState {
    db: PgPool,
    api_key: String,
}

#[actix_web::post("/events")]
async fn receive_event(
    event: web::Json<AnalyticsEvent>,
    data: web::Data<AppState>,
) -> Result<impl Responder> {
    let result = sqlx::query!(
        "INSERT INTO events (event_type, post_id, data) VALUES ($1, $2, $3)",
        event.event_type,
        event.post_id,
        event.data
    )
    .execute(&data.db)
    .await;

    match result {
        Ok(_) => Ok(HttpResponse::Created().finish()),
        Err(e) => {
            eprintln!("Failed to insert event: {}", e);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

#[actix_web::get("/stats")]
async fn get_stats(data: web::Data<AppState>) -> Result<impl Responder> {
    let stats = sqlx::query_as!(
        EventStats,
        "SELECT event_type, COUNT(*) as count FROM events GROUP BY event_type"
    )
    .fetch_all(&data.db)
    .await
    .unwrap_or_default();

    Ok(web::Json(stats))
}

async fn auth_middleware(
    req: HttpRequest,
    data: web::Data<AppState>,
    next: Next<impl Responder>,
) -> Result<impl Responder> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    match auth_header {
        Some(token) if token == data.api_key => Ok(next.call(req).await?),
        _ => Ok(HttpResponse::Unauthorized().finish()),
    }
}

#[actix_web::get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json("OK")
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:password@localhost:5432/analytics_db"
    )]
    pool: PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let api_key = secrets
        .get("ANALYTICS_API_KEY")
        .expect("ANALYTICS_API_KEY must be set");

    let state = AppState { db: pool, api_key };

    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(
            App::new()
                .app_data(web::Data::new(state.clone()))
                .service(health)
                .service(get_stats)
                .service(
                    web::resource("/events")
                        .wrap_fn(|req, srv| {
                            let state = req.app_data::<web::Data<AppState>>().unwrap().clone();
                            async move {
                                auth_middleware(req, state, srv).await
                            }
                        })
                        .route(web::post().to(receive_event))
                )
        );
    };

    Ok(config.into())
}