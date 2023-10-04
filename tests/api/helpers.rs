use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use mongodb::Database;
use rinha_backend_2023_q3::configuration::get_static_configuration;
use rinha_backend_2023_q3::get_database_connection;

pub async fn spawn_app() -> String {
    let test_address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let test_listener = TcpListener::bind(test_address).expect("failed to bind random port");
    let local_address = test_listener.local_addr().unwrap();
    let database = get_test_database()
        .await
        .expect("failed to connect to mongodb");

    let test_server = rinha_backend_2023_q3::run(test_listener, database)
        .await
        .expect("failed to run the server");

    tokio::spawn(test_server);
    format!("http://{}", local_address)
}

pub async fn get_test_database() -> Result<Database, mongodb::error::Error> {
    let mut test_config = get_static_configuration().expect("failed to load configs");
    let test_database_name = format!("test-{}", &ulid::Ulid::new().to_string());
    test_config.database.database_name = test_database_name;

    get_database_connection(test_config.database).await
}
