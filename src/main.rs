mod structs;

use crate::structs::person::Person;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{http::header, http::StatusCode, response::IntoResponse, Json, Router};
use mongodb::{bson::doc, options::ClientOptions, Client, Collection, Database};
use std::net::SocketAddr;
use structs::api;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[instrument]
async fn get_person(State(client): State<Database>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let devs_store: Collection<Person> = client.collection("devs");
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
            error!("get_by_id: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument]
async fn create_person(
    State(client): State<Database>,
    Json(body): Json<api::CreatePersonBody>,
) -> impl IntoResponse {
    let user = Person {
        id: Uuid::new_v4(),
        name: body.name,
        nickname: body.nickname,
        birth_date: body.birth_date,
        stacks: body.stacks,
    };

    let devs_store: Collection<Person> = client.collection("devs");

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
            error!("post: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("Starting server at {}", addr);
    axum::Server::bind(&addr)
        .serve(app(client.database("test")).into_make_service())
        .await
        .unwrap()
}

fn app(database: Database) -> Router {
    Router::new()
        .route("/pessoas/:id", get(get_person))
        .route("/pessoas", post(create_person))
        .route("/pessoas", get(search_persons))
        .route("/contagem-pessoas", get(count_persons))
        .with_state(database)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header::LOCATION, StatusCode};
    use axum_test_helper::TestClient;
    use chrono::NaiveDate;
    use serde_json::json;

    async fn get_test_database(function_name: &str) -> Database {
        let uri = "mongodb://root:example@localhost:27017/test?authSource=admin";
        let client_options = ClientOptions::parse(uri).await.unwrap();
        let client = Client::with_options(client_options).unwrap();
        let test_database_name =
            format!("test-{}-{}", function_name, &ulid::Ulid::new().to_string());
        client.database(&test_database_name)
    }

    #[tokio::test]
    async fn valid_post_request() {
        let client = TestClient::new(app(get_test_database("hello_world").await));

        let res = client
            .post("/pessoas")
            .json(&api::CreatePersonBody {
                nickname: String::from("foo"),
                name: String::from("bye"),
                birth_date: NaiveDate::from_ymd_opt(1992, 11, 23).unwrap(),
                stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
            })
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::CREATED);
        assert!(res
            .headers()
            .get(LOCATION)
            .expect("header found")
            .to_str()
            .expect("ASCII value")
            .starts_with("/pessoas/"));
        let response = res.json::<api::PersonBody>().await;
        assert_eq!(response.nickname, String::from("foo"));
        assert_eq!(response.name, String::from("bye"));
        assert_eq!(
            response.birth_date,
            NaiveDate::from_ymd_opt(1992, 11, 23).unwrap()
        );
        assert_eq!(
            response.stacks,
            Some(vec![String::from("Rust"), String::from("Ruby")])
        );
    }
    #[tokio::test]
    async fn other_valid_post_request() {
        let client = TestClient::new(app(get_test_database("hello_world").await));

        let res = client
            .post("/pessoas")
            .json(&api::CreatePersonBody {
                nickname: String::from("foo"),
                name: String::from("bye"),
                birth_date: NaiveDate::from_ymd_opt(1992, 11, 23).unwrap(),
                stacks: None,
            })
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::CREATED);
        assert!(res
            .headers()
            .get(LOCATION)
            .expect("header found")
            .to_str()
            .expect("ASCII value")
            .starts_with("/pessoas/"));
        let response = res.json::<api::PersonBody>().await;
        assert_eq!(response.nickname, String::from("foo"));
        assert_eq!(response.name, String::from("bye"));
        assert_eq!(
            response.birth_date,
            NaiveDate::from_ymd_opt(1992, 11, 23).unwrap()
        );
        assert_eq!(response.stacks, None);
    }
    #[tokio::test]
    async fn invalid_name_post_request() {
        let client = TestClient::new(app(get_test_database("hello_world").await));

        let res = client
            .post("/pessoas")
            .json(&json!({
                "apelido": "foo",
                "nascimento": "1992-11-23",
                "stack": ["Rust"]
            }))
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    #[tokio::test]
    async fn invalid_nickname_post_request() {
        let client = TestClient::new(app(get_test_database("hello_world").await));

        let res = client
            .post("/pessoas")
            .json(&json!({
                "nome": "foo",
                "nascimento": "1992-11-23",
                "stack": ["Rust"]
            }))
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    #[tokio::test]
    async fn invalid_stacks_content_post_request() {
        let client = TestClient::new(app(get_test_database("hello_world").await));

        let res = client
            .post("/pessoas")
            .json(&json!({
                "nome": "foo",
                "apelido": "bar",
                "nascimento": "1992-11-23",
                "stack": [1, "Rust"]
            }))
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    #[tokio::test]
    async fn invalid_name_content_post_request() {
        let client = TestClient::new(app(get_test_database("hello_world").await));

        let res = client
            .post("/pessoas")
            .json(&json!({
                "nome": 1,
                "apelido": "bar",
                "nascimento": "1992-11-23",
                "stack": ["Rust"]
            }))
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn found_person() {
        let database = get_test_database("found_person").await;
        let client = TestClient::new(app(database.clone()));

        let user = Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<Person> = database.collection("devs");
        devs_store.insert_one(&user, None).await.unwrap();

        let res = client.get(&format!("/pessoas/{}", &user.id)).send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let response = res.json::<api::PersonBody>().await;
        assert_eq!(response.name, String::from("foo"));
        assert_eq!(response.nickname, String::from("bar"));
        assert_eq!(
            response.birth_date,
            NaiveDate::from_ymd_opt(2020, 12, 3).unwrap()
        );
        assert_eq!(
            response.stacks,
            Some(vec![String::from("Rust"), String::from("Ruby")])
        );
    }

    #[tokio::test]
    async fn not_found_person() {
        let client = TestClient::new(app(get_test_database("not_found_person").await));

        let res = client
            .get("/pessoas/e50408fa-e368-4ccd-9ade-851fdb553e0f")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
