use std::net::TcpListener;

use axum::extract::{Path, Query, State};
use axum::http::Request;
use axum::routing::{get, post, IntoMakeService};
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    Json, Router,
};
use futures::stream::TryStreamExt;
use hyper::server::conn::AddrIncoming;
use mongodb::options::ClientOptions;
use mongodb::{bson::doc, Client, Collection, Database};
use tower::ServiceBuilder;
use tower_http::request_id::{MakeRequestId, RequestId};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use uuid::Uuid;

use structs::{api, person};

use crate::configuration::DatabaseConfiguration;

pub mod configuration;
mod structs;
pub mod telemetry;

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
#[tracing::instrument(name = "Looking for a developer", skip(client))]
async fn get_person(State(client): State<Database>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let devs_store: Collection<person::Person> = client.collection("devs");
    let found_dev = devs_store.find_one(doc! {"_id": id}, None).await;

    match found_dev {
        Ok(Some(dev)) => Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(api::PersonBody {
                id: dev.id,
                name: dev.name,
                nickname: dev.nickname,
                birth_date: dev.birth_date,
                stacks: dev.stacks,
            }),
        )),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(error) => {
            println!("get_by_id: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tracing::instrument(name = "Adding a new developer", skip(client, body))]
async fn create_person(
    State(client): State<Database>,
    Json(body): Json<api::CreatePersonBody>,
) -> impl IntoResponse {
    let user = person::Person {
        id: Uuid::new_v4(),
        name: body.name,
        nickname: body.nickname,
        birth_date: body.birth_date,
        stacks: body.stacks,
    };
    let devs_store: Collection<person::Person> = client.collection("devs");
    let inserted_result = devs_store.insert_one(&user, None).await;
    match inserted_result {
        Ok(_) => Ok((
            StatusCode::CREATED,
            [
                (header::LOCATION, format!("/pessoas/{}", &user.id)),
                (header::CONTENT_TYPE, String::from("application/json")),
            ],
            Json(api::PersonBody {
                id: user.id,
                name: user.name,
                nickname: user.nickname,
                birth_date: user.birth_date,
                stacks: user.stacks,
            }),
        )),
        Err(error) => {
            println!("post: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
#[tracing::instrument(name = "Searching for a developer", skip(client))]
async fn search_persons(
    State(client): State<Database>,
    Query(query): Query<api::SearchPersonQuery>,
) -> impl IntoResponse {
    let devs_store: Collection<person::Person> = client.collection("devs");

    let search_cursor = devs_store
        .find(
            doc! {
                "$or": [
                    {
                        "name": mongodb::bson::Regex{
                            pattern: query.search_term.clone(),
                            options: String::from("i"),
                        }
                    },
                    {
                        "stacks": {
                            "$in": [
                                mongodb::bson::Regex{
                                    pattern: query.search_term.clone(),
                                    options: String::from("i"),
                                }
                            ]
                        }
                    },
                    {
                        "nickname": mongodb::bson::Regex{
                            pattern: query.search_term.clone(),
                            options: String::from("i"),
                        }
                    }
                ]
            },
            None,
        )
        .await;
    match search_cursor {
        Ok(cursor) => {
            let found_devs = cursor.try_collect().await.unwrap_or_else(|_| vec![]);

            Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                axum::Json(
                    found_devs
                        .iter()
                        .map(|dev| api::PersonBody {
                            id: dev.id,
                            name: dev.name.clone(),
                            nickname: dev.nickname.clone(),
                            birth_date: dev.birth_date,
                            stacks: dev.stacks.clone(),
                        })
                        .collect::<Vec<api::PersonBody>>(),
                ),
            ))
        }
        Err(error) => {
            println!("persons?t=QUERY: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn count_persons(State(client): State<Database>) -> impl IntoResponse {
    let devs_store: Collection<person::Person> = client.collection("devs");
    let found_dev = devs_store.count_documents(None, None).await;

    match found_dev {
        Ok(count) => Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, String::from("text/plain"))],
            format!("{}", count),
        )),
        Err(error) => {
            println!("count: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn run(
    listener: TcpListener,
    mongodb_pool: Database,
) -> Result<axum::Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    Ok(axum::Server::from_tcp(listener)?.serve(app(mongodb_pool).into_make_service()))
}

pub async fn get_database_connection(
    database_config: DatabaseConfiguration,
) -> Result<Database, mongodb::error::Error> {
    let client_options = ClientOptions::parse(database_config.connection_string()).await?;
    let client = Client::with_options(client_options)?;
    Ok(client.database(&database_config.database_name))
}

#[derive(Clone, Copy)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let request_id = Uuid::new_v4().to_string().parse().unwrap();
        Some(RequestId::new(request_id))
    }
}

fn app(database: Database) -> Router {
    let sensitive_headers: std::sync::Arc<[_]> = vec![header::AUTHORIZATION, header::COOKIE].into();

    let tracing_middleware = ServiceBuilder::new()
        .sensitive_request_headers(sensitive_headers.clone())
        .set_x_request_id(MakeRequestUuid)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .include_headers(true)
                        .level(tracing::Level::INFO),
                )
                .on_response(DefaultOnResponse::new().include_headers(true)),
        )
        .propagate_x_request_id()
        .sensitive_response_headers(sensitive_headers);

    Router::new()
        .route("/pessoas/:id", get(get_person))
        .route("/pessoas", post(create_person))
        .route("/pessoas", get(search_persons))
        .route("/contagem-pessoas", get(count_persons))
        .layer(tracing_middleware)
        .route("/health-check", get(health_check))
        .with_state(database)
}
