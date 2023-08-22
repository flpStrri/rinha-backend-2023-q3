use axum::extract::{Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use ulid::Ulid;

#[instrument]
async fn get_person(Path(id): Path<Ulid>) {
    info!("GET /pessoas/{id} happened")
}

#[instrument]
async fn create_person(Json(body): Json<CreatePersonBody>) {
    info!("POST /pessoas happened");
}

#[instrument]
async fn search_persons(Query(query): Query<SearchPersonQuery>) {
    info!("GET /pessoas?t={0} happened", query.search_term)
}

#[instrument]
async fn count_persons() {
    info!("GET contagem-pessoas happened");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/pessoas/:id", get(get_person))
        .route("/pessoas", post(create_person))
        .route("/pessoas", get(search_persons))
        .route("/contagem-pessoas", get(count_persons));

    info!("Starting server at port 3000...");

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap()
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct CreatePersonBody {
    #[serde(rename(deserialize = "apelido"))]
    pub nickname: String,
    #[serde(rename(deserialize = "nome"))]
    pub name: String,
    #[serde(rename(deserialize = "nascimento"))]
    pub birth_date: NaiveDate,
    #[serde(rename(deserialize = "stack"))]
    pub stacks: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct SearchPersonQuery {
    #[serde(rename(deserialize = "t"))]
    search_term: String,
}
