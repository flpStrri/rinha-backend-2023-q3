use std::net::TcpListener;

use axum::{http::header, http::StatusCode, Json, response::IntoResponse, Router};
use axum::extract::{Path, Query, State};
use axum::routing::{get, IntoMakeService, post};
use futures::stream::TryStreamExt;
use hyper::server::conn::AddrIncoming;
use mongodb::{bson::doc, Client, Collection, Database};
use mongodb::options::ClientOptions;
use uuid::Uuid;

use structs::{api, person};

use crate::configuration::DatabaseConfiguration;

mod structs;
pub mod configuration;

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

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

pub async fn run(listener: TcpListener, mongodb_pool: Database) -> Result<axum::Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    Ok(
        axum::Server::from_tcp(listener)?
            .serve(app(mongodb_pool).into_make_service())
    )
}

pub async fn get_database_connection(database_config: DatabaseConfiguration) -> Result<Database, mongodb::error::Error> {
    let client_options = ClientOptions::parse(database_config.connection_string()).await?;
    let client = Client::with_options(client_options)?;
    Ok(client.database(&database_config.database_name))
}

fn app(database: Database) -> Router {
    Router::new()
        .route("/health-check", get(health_check))
        .route("/pessoas/:id", get(get_person))
        .route("/pessoas", post(create_person))
        .route("/pessoas", get(search_persons))
        .route("/contagem-pessoas", get(count_persons))
        .with_state(database)
}


#[cfg(test)]
mod tests {
    use axum::http::{header::LOCATION, StatusCode};
    use axum_test_helper::TestClient;
    use chrono::NaiveDate;
    use mongodb::Client;
    use mongodb::options::ClientOptions;
    use serde_json::json;

    use crate::*;

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
        let client = TestClient::new(app(get_test_database("valid_post_request").await));

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
        let client = TestClient::new(app(get_test_database("other_valid_post_request").await));

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
        let client = TestClient::new(app(get_test_database("invalid_name_post_request").await));

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
        let client = TestClient::new(app(get_test_database("invalid_stacks_post_request").await));

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
        let client = TestClient::new(app(get_test_database("invalid_name_post_request").await));

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
        let user = person::Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<person::Person> = database.collection("devs");
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

    #[tokio::test]
    async fn count_persons_on_empty_db() {
        let database = get_test_database("count_persons_on_empty_db").await;
        let client = TestClient::new(app(database.clone()));

        let res = client.get("/contagem-pessoas").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "0");
    }

    #[tokio::test]
    async fn count_persons_populated_db() {
        let database = get_test_database("count_persons_on_populated_db").await;
        let client = TestClient::new(app(database.clone()));

        let user = person::Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<person::Person> = database.collection("devs");
        devs_store.insert_one(&user, None).await.unwrap();

        let res = client.get("/contagem-pessoas").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "1");
    }

    #[tokio::test]
    async fn search_person_by_name_with_exact_match() {
        let database = get_test_database("search_person_name").await;
        let client = TestClient::new(app(database.clone()));

        let user = person::Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<person::Person> = database.collection("devs");
        devs_store.insert_one(&user, None).await.unwrap();

        let res = client.get("/pessoas?t=foo").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let mut response = res.json::<Vec<api::PersonBody>>().await;
        let only_response: api::PersonBody = response.pop().expect("a person in response");
        assert_eq!(only_response.id, user.id)
    }

    #[tokio::test]
    async fn search_person_by_name_without_exact_match() {
        let database = get_test_database("search_person_name").await;
        let client = TestClient::new(app(database.clone()));

        let user = person::Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<person::Person> = database.collection("devs");
        devs_store.insert_one(&user, None).await.unwrap();

        let res = client.get("/pessoas?t=fo").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let mut response = res.json::<Vec<api::PersonBody>>().await;
        let only_response: api::PersonBody = response.pop().expect("a person in response");
        assert_eq!(only_response.id, user.id)
    }

    #[tokio::test]
    async fn search_person_by_nickname_without_exact_match() {
        let database = get_test_database("search_person_name").await;
        let client = TestClient::new(app(database.clone()));

        let user = person::Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<person::Person> = database.collection("devs");
        devs_store.insert_one(&user, None).await.unwrap();

        let res = client.get("/pessoas?t=ba").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let mut response = res.json::<Vec<api::PersonBody>>().await;
        let only_response: api::PersonBody = response.pop().expect("a person in response");
        assert_eq!(only_response.id, user.id)
    }

    #[tokio::test]
    async fn search_person_by_stack_without_exact_match() {
        let database = get_test_database("search_person_name").await;
        let client = TestClient::new(app(database.clone()));

        let user = person::Person {
            id: Uuid::new_v4(),
            name: String::from("foo"),
            nickname: String::from("bar"),
            birth_date: NaiveDate::from_ymd_opt(2020, 12, 3).unwrap(),
            stacks: Some(vec![String::from("Rust"), String::from("Ruby")]),
        };

        let devs_store: Collection<person::Person> = database.collection("devs");
        devs_store.insert_one(&user, None).await.unwrap();

        let res = client.get("/pessoas?t=rus").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let mut response = res.json::<Vec<api::PersonBody>>().await;
        let only_response: api::PersonBody = response.pop().expect("a person in response");
        assert_eq!(only_response.id, user.id)
    }
}
