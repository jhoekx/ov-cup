// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    fs::File,
    io::BufReader,
    str::FromStr,
};

use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct CourseResult {
    pub name: String,
    pub club: String,
    #[serde(rename = "ageclass")]
    pub age_class: Option<String>,
    #[serde(deserialize_with = "from_str")]
    pub position: u32,
    pub time: Option<NaiveTime>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct Category {
    pub name: String,
    #[serde(deserialize_with = "from_str")]
    pub distance: u32,
    #[serde(deserialize_with = "from_str")]
    pub climb: u32,
    pub results: Vec<CourseResult>,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub date: DateTime<Utc>,
    pub name: String,
    pub location: String,
    pub categories: HashMap<String, Category>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DataType<T> {
    String(String),
    Number(T),
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr + Deserialize<'de> + Debug,
    T::Err: Display,
    D: Deserializer<'de>,
{
    match DataType::<T>::deserialize(deserializer)? {
        DataType::String(s) => T::from_str(&s).map_err(serde::de::Error::custom),
        DataType::Number(n) => Ok(n),
    }
}

#[derive(Error, Debug)]
pub enum WebresError {
    #[error("unable to read json file {path:?}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid json in {path:?}")]
    InvalidJSON {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

pub fn read_event_json(path: String) -> Result<Event, WebresError> {
    let file = File::open(&path).map_err(|source| WebresError::FileRead {
        path: path.to_owned(),
        source,
    })?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|source| WebresError::InvalidJSON { path, source })
}
