mod structs;

use crate::structs::person::Person;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{http::StatusCode, response::IntoResponse, Json, Router};
use mongodb::bson::Uuid;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection, Database};
use std::net::SocketAddr;
use structs::api;
use tracing::{info, instrument};

#[instrument]
async fn get_person(State(client): State<Database>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let devs_store: Collection<Person> = client.collection("devs");
    let aah: Person = devs_store
        .find_one(doc! {"_id": id}, None)
        .await
        .unwrap()
        .unwrap();

    (
        StatusCode::OK,
        Json(api::PersonBody {
            id: aah.id,
            name: aah.name,
            nickname: aah.nickname,
            birth_date: aah.birth_date,
            stacks: aah.stacks,
        }),
    )
}

#[instrument]
async fn create_person(
    State(client): State<Database>,
    Json(body): Json<api::CreatePersonBody>,
) -> impl IntoResponse {
    let user = Person {
        id: Uuid::new(),
        name: body.name,
        nickname: body.nickname,
        birth_date: body.birth_date,
        stacks: body.stacks,
    };

    let devs_store: Collection<Person> = client.collection("devs");
    devs_store.insert_one(&user, None).await.unwrap();

    (
        StatusCode::CREATED,
        [("Location", format!("/pessoas/{}", &user.id))],
        Json(api::PersonBody {
            id: user.id,
            name: user.name,
            nickname: user.nickname,
            birth_date: user.birth_date,
            stacks: user.stacks,
        }),
    )
}

#[instrument]
async fn search_persons(Query(query): Query<api::SearchPersonQuery>) {
    info!("GET /pessoas?t={0} happened", query.search_term)
}

#[instrument]
async fn count_persons() {
    info!("GET contagem-pessoas happened");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let uri = "mongodb://root:example@localhost:27017/test?authSource=admin";
    let client_options = ClientOptions::parse(uri).await.unwrap();
    let client = Client::with_options(client_options).unwrap();

    client
        .database("admin")
        .run_command(doc! {"ping": 1}, None)
        .await
        .unwrap();
    info!("Successfully connected to MongoDB!");

    let app = Router::new()
        .route("/pessoas/:id", get(get_person))
        .route("/pessoas", post(create_person))
        .route("/pessoas", get(search_persons))
        .route("/contagem-pessoas", get(count_persons))
        .with_state(client.database("test"));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("Starting server at {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap()
}
