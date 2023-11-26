/// IOF CompetitorList XML
// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later
use std::{fs, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CompetitorList {
    #[serde(rename = "$value")]
    pub competitors: Vec<Competitor>,
}

#[derive(Debug, Deserialize)]
pub struct Competitor {
    #[serde(rename = "Person")]
    pub person: Person,
    #[serde(rename = "Class")]
    pub class: Class,
}

#[derive(Debug, Deserialize)]
pub struct Person {
    #[serde(rename = "Name")]
    pub name: Name,
}

#[derive(Debug, Deserialize)]
pub struct Name {
    #[serde(rename = "Family")]
    pub family: String,
    #[serde(rename = "Given")]
    pub given: String,
}

#[derive(Debug, Deserialize)]
pub struct Class {
    #[serde(rename = "Name")]
    pub name: String,
}

pub fn parse_competitor_list(path: &Path) -> anyhow::Result<CompetitorList> {
    let xml_data = fs::read_to_string(path)?;
    let list: CompetitorList = serde_xml_rs::from_str(&xml_data)?;
    Ok(list)
}
