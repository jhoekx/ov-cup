// SPDX-FileCopyrightText: 2023 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use chrono::NaiveTime;
use itertools::Itertools;
use rusqlite::params;

use crate::{db::Database, total_seconds, Performance, RankingEntry, RankingScore};

pub(crate) fn calculate_ranking(
    db: &dyn Database,
    cup: String,
    season: i16,
    age_class: String,
    events_count: usize,
) -> Result<Vec<RankingEntry>, anyhow::Error> {
    let conn = db.open()?;

    // Find all events
    let mut stmt =
        conn.prepare("select id from Event where cup = ? and season = ? order by date asc")?;
    let events: Vec<u64> = stmt
        .query_map(params![cup, season], |row| {
            let event_id = row.get(0)?;
            Ok(event_id)
        })?
        .filter_map(|event_id| event_id.ok())
        .collect();

    // Find all results of all runners with at least one ranking in the given category
    let mut stmt = conn.prepare(
        "
        select
            Runner.name,
            Runner.club,
            Event.id,
            Result.age_class,
            Result.category_name,
            Result.position,
            Result.time
        from Result join Runner on Result.runner_id = Runner.id
                    join Event on Result.event_id = Event.id
        where Event.cup = ? and Event.season = ?
          and Runner.id in (
              select Runner.id
              from Runner join Result on Runner.id = Result.Runner_id
              where Result.age_class = ?
          )
        order by Runner.name asc, Event.date asc
    ",
    )?;
    let all_results = stmt
        .query_map(params![cup, season, age_class], |row| {
            let event_id = row.get(2)?;
            Ok(Performance {
                name: row.get(0)?,
                club: row.get(1)?,
                event_id,
                age_class: row.get(3)?,
                category_name: row.get(4)?,
                position: row.get(5)?,
                time: row.get(6)?,
                score: 0,
            })
        })?
        .filter_map(|r| r.ok());

    // TODO: Keep only courses that are valid for the age class of the result

    // Keep only results of runners where the last age class equals the given age class
    // This filters out runners who moved to a different category,
    // while keeping the runners that moved into this category.
    let mut results = Vec::new();
    for (_, runner_results) in &all_results
        .into_iter()
        .chunk_by(|result| result.name.to_owned())
    {
        let mut runner_results: Vec<Performance> = runner_results.collect();
        if runner_results.last().unwrap().age_class == age_class {
            results.append(&mut runner_results);
        }
    }

    // Find the best results in all courses that someone of the given age class participated in
    let courses: Vec<(u64, String)> = results
        .iter()
        .map(|result| (result.event_id, result.category_name.to_owned()))
        .unique()
        .collect();
    let mut stmt = conn.prepare(
        "
        select Result.time
        from Result
        where Result.event_id = ? and Result.category_name = ?
        order by Result.time asc
        limit 1
    ",
    )?;
    let mut fastest_times = HashMap::new();
    for (event_id, category_name) in courses {
        let fastest_time: NaiveTime =
            stmt.query_row(params![event_id, category_name], |row| row.get(0))?;
        fastest_times.insert((event_id, category_name), total_seconds(fastest_time));
    }

    // Calculate score for each performance based on the fastest times
    let results = results.into_iter().map(|result| {
        let score = 1000
            * fastest_times
                .get(&(result.event_id, result.category_name.to_owned()))
                .unwrap()
            / total_seconds(result.time);
        Performance { score, ..result }
    });

    // Calculate the total scores per runner
    let mut ranking: Vec<RankingEntry> = Vec::new();
    for (name, runner_results) in &results
        .into_iter()
        .chunk_by(|result| result.name.to_owned())
    {
        let runner_results: Vec<Performance> = runner_results.collect();
        let mut scores: Vec<u32> = runner_results.iter().map(|result| result.score).collect();
        scores.sort_unstable();
        scores.reverse();
        let total_score: u32 = scores.iter().take(events_count).sum();

        let ranking_scores: Vec<RankingScore> = runner_results
            .iter()
            .map(|performance| RankingScore {
                event_id: performance.event_id,
                score: Some(performance.score),
                place: Some(performance.position),
            })
            .collect();

        ranking.push(RankingEntry {
            name,
            club: runner_results
                .last()
                .map_or("".to_owned(), |performance| performance.club.to_string()),
            total_score,
            scores: events
                .iter()
                .map(|&event_id| {
                    ranking_scores
                        .iter()
                        .find(|&score| score.event_id == event_id)
                        .copied()
                        .unwrap_or(RankingScore {
                            event_id,
                            score: None,
                            place: None,
                        })
                })
                .collect(),
        })
    }
    ranking.sort_by_key(|entry| entry.total_score);
    ranking.reverse();
    Ok(ranking)
}
