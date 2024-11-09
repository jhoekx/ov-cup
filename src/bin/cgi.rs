// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::PathBuf;

use ov_cup::calculate_ranking;
use ov_cup::db::LocalDatabase;

pub fn main() {
    rust_cgi::handle(|request| {
        let query = request.uri().query().unwrap();
        let params: HashMap<_, _> = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();

        let cup = if let Some(cup) = params.get("cup") {
            cup.to_string()
        } else {
            return rust_cgi::text_response(400, "missing parameter 'cup'");
        };
        let season = if let Some(season) = params.get("season") {
            match season.parse::<i16>() {
                Ok(season) => season,
                Err(_) => return rust_cgi::text_response(400, "invalid parameter 'season'"),
            }
        } else {
            return rust_cgi::text_response(400, "missing parameter 'season'");
        };
        let age_class = if let Some(age_class) = params.get("ageClass") {
            age_class.to_string()
        } else {
            return rust_cgi::text_response(400, "missing parameter 'ageClass'");
        };
        let events_count = if let Some(events_count) = params.get("events") {
            if let Ok(events_count) = events_count.parse::<usize>() {
                events_count
            } else {
                return rust_cgi::text_response(400, "parameter 'events' should be a number");
            }
        } else {
            return rust_cgi::text_response(400, "missing parameter 'events'");
        };

        let script_path = match std::env::var("SCRIPT_FILENAME") {
            Ok(script_path) => PathBuf::from(script_path),
            Err(_) => {
                return rust_cgi::text_response(
                    500,
                    "SCRIPT_FILENAME environment variable is missing",
                )
            }
        };
        let db_path = script_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ov.sqlite");
        let db = LocalDatabase::new(db_path);

        match calculate_ranking(&db, cup, season, age_class, events_count) {
            Ok(ranking) => {
                let body = serde_json::to_vec(&ranking).unwrap();
                rust_cgi::binary_response(200, "application/json", body)
            }
            Err(err) => rust_cgi::text_response(500, err.to_string()),
        }
    })
}
