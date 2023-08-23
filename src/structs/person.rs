use chrono::NaiveDate;
use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Person {
    #[serde(rename(serialize = "_id", deserialize = "_id"))]
    pub id: Uuid,
    pub nickname: String,
    pub name: String,
    pub birth_date: NaiveDate,
    pub stacks: Option<Vec<String>>,
}
