#[derive(serde::Deserialize)]
pub struct StaticConfiguration {
    pub database: DatabaseConfiguration,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseConfiguration {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseConfiguration {
    pub fn connection_string(&self) -> String {
        format!(
            "mongodb://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port)
    }
}

pub fn get_static_configuration() -> Result<StaticConfiguration, config::ConfigError> {
    let settings = config::Config::builder()
        .add_source(
            config::File::new("configuration.yaml", config::FileFormat::Yaml)
        )
        .build()?;

    settings.try_deserialize::<StaticConfiguration>()
}
