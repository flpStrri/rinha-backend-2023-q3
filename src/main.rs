use std::net::{Ipv4Addr, SocketAddr, TcpListener};

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let static_config = rinha_backend_2023_q3::configuration::get_static_configuration()
        .expect("failed to load configs");
    let server_address = SocketAddr::from((Ipv4Addr::LOCALHOST, static_config.application_port));
    let server_listener = TcpListener::bind(server_address).expect("failed to bind random port");

    let mongodb_pool = rinha_backend_2023_q3::get_database_connection(static_config.database)
        .await
        .expect("failed to connect to mongodb");

    rinha_backend_2023_q3::run(server_listener, mongodb_pool)
        .await
        .unwrap()
        .await
}
