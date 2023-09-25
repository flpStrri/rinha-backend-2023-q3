use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use mongodb::Database;
use reqwest::header::LOCATION;
use reqwest::StatusCode;
use serde_json::json;

use rinha_backend_2023_q3::configuration::get_static_configuration;
use rinha_backend_2023_q3::get_database_connection;

async fn spawn_app() -> String {
    let test_address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let test_listener = TcpListener::bind(test_address).expect("failed to bind random port");
    let local_address = test_listener.local_addr().unwrap();
    let database = get_test_database().await.expect("failed to connect to mongodb");

    let test_server = rinha_backend_2023_q3::run(test_listener, database).await.expect("failed to run the server");

    tokio::spawn(test_server);
    format!("http://{}", local_address)
}

async fn get_test_database() -> Result<Database, mongodb::error::Error> {
    let mut test_config = get_static_configuration().expect("failed to load configs");
    let test_database_name = format!("test-{}", &ulid::Ulid::new().to_string());
    test_config.database.database_name = test_database_name;

    get_database_connection(test_config.database).await
}

#[tokio::test]
async fn health_check_works() {
    let test_address = spawn_app().await;
    let response = reqwest::Client::new()
        .get(format!("{}/health-check", test_address))
        .send()
        .await
        .expect("failed request");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn it_returns_a_dev_given_a_valid_body() {
    let test_address = spawn_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/pessoas", test_address))
        .json(
            &json!({
                "apelido": "foo",
                "nome": "bye",
                "nascimento": "1992-11-23",
                "stack": ["Rust", "Ruby"]
            })
        )
        .send()
        .await
        .expect("failed request");

    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(response
        .headers()
        .get(LOCATION)
        .expect("header not found")
        .to_str()
        .expect("not ASCII value")
        .starts_with("/pessoas/"));
    let response_body = response.json::<HashMap<String, serde_json::Value>>().await.unwrap();
    assert_eq!(response_body["apelido"], String::from("foo"));
    assert_eq!(response_body["nome"], String::from("bye"));
    assert_eq!(
        response_body["nascimento"],
        String::from("1992-11-23")
    );
    assert_eq!(
        response_body["stack"],
        json!([String::from("Rust"), String::from("Ruby")])
    );
}
