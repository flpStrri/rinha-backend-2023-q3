use chrono::NaiveDate;
use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct CreatePersonBody {
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
pub struct SearchPersonQuery {
    #[serde(rename(deserialize = "t"))]
    pub search_term: String,
}

#[derive(Debug, Default, Serialize)]
pub struct PersonBody {
    pub id: Uuid,
    #[serde(rename(serialize = "apelido"))]
    pub nickname: String,
    #[serde(rename(serialize = "nome"))]
    pub name: String,
    #[serde(rename(serialize = "nascimento"))]
    pub birth_date: NaiveDate,
    #[serde(rename(serialize = "stack"))]
    pub stacks: Option<Vec<String>>,
}
