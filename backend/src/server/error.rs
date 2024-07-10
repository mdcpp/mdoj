use opentelemetry::{metrics::MetricsError, trace::TraceError};

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("Fail to create initial connection: `{0}`")]
    InitConn(sea_orm::DbErr),
    #[error("Fail to optimize database: `{0}`")]
    OptimizeDB(sea_orm::DbErr),
    #[cfg(feature = "standalone")]
    #[error("Fail to run auto migration: `{0}`")]
    AutoMigrate(Box<dyn std::error::Error>),
    #[error("Fail to create initial user: `{0}`")]
    UserCreation(sea_orm::DbErr),
    #[error("Fail to create config dictionary: `{0}`")]
    ConfigDir(std::io::Error),
    #[error("Fail to parse config: `{0}`")]
    ConfigParse(toml::de::Error),
    #[error("Fail to read config: `{0}`")]
    ConfigRead(std::io::Error),
    #[error("Fail to write config: `{0}`")]
    ConfigWrite(std::io::Error),
    #[error("`{0}`")]
    Tracer(#[from] TraceError),
    #[error("`{0}`")]
    Metrics(#[from] MetricsError),
    #[error("Fail to read pem file: `{0}`")]
    ReadPem(std::io::Error),
}
