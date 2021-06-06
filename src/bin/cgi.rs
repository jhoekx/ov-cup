// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use ov_cup::calculate_ranking;

pub fn main() {
    cgi::handle(|request| {
        let query = request.uri().query().unwrap();
        let params: HashMap<_, _> = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();

        let cup = if let Some(cup) = params.get("cup") {
            cup.to_string()
        } else {
            return cgi::text_response(400, "missing parameter 'cup'");
        };
        let season = if let Some(season) = params.get("season") {
            season.to_string()
        } else {
            return cgi::text_response(400, "missing parameter 'season'");
        };
        let age_class = if let Some(age_class) = params.get("ageClass") {
            age_class.to_string()
        } else {
            return cgi::text_response(400, "missing parameter 'ageClass'");
        };

        cgi::err_to_500(calculate_ranking(cup, season, age_class).map(|ranking| {
            let body = serde_json::to_vec(&ranking).unwrap();
            cgi::binary_response(200, "application/json", body)
        }))
    })
}
