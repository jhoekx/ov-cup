// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, path::Path};

#[macro_use]
extern crate lazy_static;

use chrono::{NaiveTime, Timelike};
use itertools::Itertools;
use rusqlite::{params, Connection};
use serde::Serialize;

pub mod cli;
pub mod webres;

const CLUBS: &[&str] = &[
    "Antwerp Orienteers",
    "Borasca",
    "hamok",
    "K.O.L.",
    "Omega",
    "Trol",
];

const CLASSES: &[&str] = &[
    "H. Pupilles",
    "D. Pupilles",
    "H. Espoirs - Beloften",
    "D. Espoirs - Beloften",
    "H. Junioren - Juniors",
    "D. Junioren - Juniores",
    "H. Open",
    "D. Open",
    "H. Masters A",
    "D. Masters A",
    "H. Masters B",
    "D. Masters B",
    "H. Masters C",
    "D. Masters C",
    "H. Masters D",
    "D. Masters D",
    "H. Masters E",
    "D. Masters E",
    "H. Masters F",
    "D. Masters F",
];

lazy_static! {
    static ref COURSES: HashMap<&'static str, i32> = {
        HashMap::<_, _>::from_iter(IntoIterator::into_iter([
            ("H-20", 1),
            ("H21", 1),
            ("H35", 1),
            ("H-18", 2),
            ("H40", 2),
            ("H45", 2),
            ("H50", 2),
            ("D-20", 2),
            ("D21", 2),
            ("H-16", 3),
            ("H55", 3),
            ("H60", 3),
            ("D-16", 3),
            ("D-18", 3),
            ("D35", 3),
            ("D40", 3),
            ("D45", 3),
            ("H-14", 4),
            ("H65", 4),
            ("D-14", 4),
            ("D50", 4),
            ("D55", 4),
            ("H10B", 8),
            ("H-10", 8),
            ("H-12", 8),
            ("H70", 5),
            ("H75", 5),
            ("H80", 5),
            ("H85", 5),
            ("H90", 5),
            ("D10B", 8),
            ("D-10", 8),
            ("D-12", 8),
            ("D60", 5),
            ("D65", 5),
            ("D70", 5),
            ("D75", 5),
            ("D80", 5),
            ("D85", 5),
            ("D90", 5),
        ]))
    };
}

pub struct ResultProcessingOptions {
    pub cup: String,
    pub season: String,
    pub results_by_class: Option<bool>,
}

impl ResultProcessingOptions {
    pub fn validate_club(&self) -> bool {
        self.cup == "kampioen"
    }
}

pub fn create_database(db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", &"on")?;
    conn.pragma_update(None, "journal_mode", &"WAL")?;
    conn.execute_batch(
        "
        create table if not exists Runner (
            id integer primary key autoincrement,
            name text not null,
            club text not null,

            unique(name)
        );

        create table if not exists Event (
            id integer primary key autoincrement,
            cup text not null,
            season text not null,
            name text not null,
            location text not null,
            date text not null,

            unique(cup, season, name, date)
        );

        create table if not exists Result (
            id integer primary key autoincrement,
            event_id integer not null,
            runner_id integer not null,
            category_name text not null,
            age_class text not null,
            position integer not null,
            time text not null,

            foreign key(event_id) references Event(id),
            foreign key(runner_id) references Runner(id)
        )
    ",
    )?;
    Ok(())
}

pub fn store_event(
    db_path: &Path,
    event: webres::Event,
    options: ResultProcessingOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;

    let event_db_id = prepare_event(&conn, &options.cup, &options.season, &event)?;
    if options.cup == "kampioen" || (options.results_by_class.unwrap_or(false)) {
        store_event_by_class(conn, event, options, event_db_id)?;
    } else {
        store_event_by_course(conn, event, event_db_id)?;
    }

    Ok(())
}

fn prepare_event(
    conn: &Connection,
    cup: &str,
    season: &str,
    event: &webres::Event,
) -> Result<i64, Box<dyn std::error::Error>> {
    conn.execute(
        "
        insert into Event (cup, season, name, location, date) values (?, ?, ?, ?, ?)
        on conflict (cup, season, name, date) do update set location = excluded.location;
    ",
        params![cup, season, event.name, event.location, event.date],
    )?;
    let event_db_id: i64 = conn.query_row(
        "
        select id from Event where name = ? and date = ?
    ",
        params![event.name, event.date],
        |row| row.get(0),
    )?;
    conn.execute(
        "
        delete from Result where event_id = ?
    ",
        params![event_db_id],
    )?;
    Ok(event_db_id)
}

fn store_event_by_class(
    conn: Connection,
    event: webres::Event,
    options: ResultProcessingOptions,
    event_db_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    for category in event.categories.values() {
        if !COURSES.contains_key(&category.name as &str)
            && !CLASSES.contains(&(&category.name as &str))
        {
            eprintln!("Skipping class {}", category.name);
            continue;
        }

        for result in &category.results {
            if result.status != "OK" || result.position == 0 {
                continue;
            }

            let club = result.club.to_string();
            if options.validate_club() && !is_ov_club(&club) {
                eprintln!("Skipping club {}", club);
                continue;
            }

            conn.execute(
                "
                insert into Runner (name, club) values (?, ?)
                on conflict (name) do update set club = excluded.club;
            ",
                params![result.name, club],
            )?;
            let runner_db_id: i64 = conn.query_row(
                "
                select id from Runner where name = ?
            ",
                params![result.name],
                |row| row.get(0),
            )?;

            let age_class = if CLASSES.contains(&(&category.name as &str)) {
                result.age_class.as_ref().unwrap()
            } else {
                &category.name
            };

            conn.execute(
                "
                insert into Result (event_id, runner_id, category_name, age_class, position, time)
                values (?, ?, ?, ?, ?, ?)
            ",
                params![
                    event_db_id,
                    runner_db_id,
                    &category.name,
                    age_class,
                    result.position,
                    result.time
                ],
            )?;
        }
    }

    Ok(())
}

fn is_ov_club(club: &str) -> bool {
    for existing_club in CLUBS {
        if club
            .to_lowercase()
            .starts_with(&existing_club.to_lowercase())
        {
            return true;
        }
    }
    false
}

fn store_event_by_course(
    conn: Connection,
    event: webres::Event,
    event_db_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let category_re = regex::Regex::new(r"[H|D]:\d*(\d)$").unwrap();
    for category in event.categories.values() {
        let course_number = match category_re
            .captures_iter(&category.name)
            .next()
            .map(|g| g.get(1).unwrap().as_str().parse().unwrap())
        {
            Some(course_number) => course_number,
            None => {
                eprintln!("Skipping course {}", category.name);
                continue;
            }
        };

        for result in &category.results {
            if result.status != "OK" || result.position == 0 {
                continue;
            }
            let age_class = result.age_class.as_ref().unwrap();

            if COURSES[age_class as &str] < course_number {
                eprintln!(
                    "{} {} is running in incorrect course {}, should run {}",
                    result.name, age_class, course_number, COURSES[age_class as &str]
                );
                continue;
            }

            let mut club = result.club.to_string();
            for existing_club in CLUBS {
                if club
                    .to_lowercase()
                    .starts_with(&existing_club.to_lowercase())
                {
                    club = existing_club.to_string();
                }
            }

            conn.execute(
                "
                insert into Runner (name, club) values (?, ?)
                on conflict (name) do update set club = excluded.club;
            ",
                params![result.name, club],
            )?;
            let runner_db_id: i64 = conn.query_row(
                "
                select id from Runner where name = ?
            ",
                params![result.name],
                |row| row.get(0),
            )?;

            conn.execute(
                "
                insert into Result (event_id, runner_id, category_name, age_class, position, time)
                values (?, ?, ?, ?, ?, ?)
            ",
                params![
                    event_db_id,
                    runner_db_id,
                    category.name,
                    age_class,
                    result.position,
                    result.time
                ],
            )?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Performance {
    name: String,
    club: String,
    event_id: u64,
    age_class: String,
    category_name: String,
    position: u32,
    time: NaiveTime,
    score: u32,
}

fn total_seconds(time: impl Timelike) -> u32 {
    time.second() + time.minute() * 60 + time.hour() * 60 * 60
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RankingScore {
    #[serde(rename = "eventId")]
    event_id: u64,
    score: Option<u32>,
    place: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RankingEntry {
    name: String,
    club: String,
    #[serde(rename = "totalScore")]
    total_score: u32,
    scores: Vec<RankingScore>,
}

pub fn calculate_ranking(
    db_path: &Path,
    cup: String,
    season: String,
    age_class: String,
) -> Result<Vec<RankingEntry>, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;

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
        .group_by(|result| result.name.to_owned())
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
        .group_by(|result| result.name.to_owned())
    {
        let runner_results: Vec<Performance> = runner_results.collect();
        let mut scores: Vec<u32> = runner_results.iter().map(|result| result.score).collect();
        scores.sort_unstable();
        scores.reverse();
        let total_score: u32 = scores.iter().take(3).sum();

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
