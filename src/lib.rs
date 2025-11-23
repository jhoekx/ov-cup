// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use anyhow::bail;
use chrono::{NaiveTime, Timelike};
use db::Database;
use indexmap::IndexSet;
use once_cell::sync::Lazy;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub mod cli;
pub mod db;
pub mod iof;
mod rules_2022;
mod rules_2023;
mod rules_2024;
mod rules_2026;
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

/// Courses using number codes as used before 2025.
static COURSES_NUMBERED: Lazy<HashMap<&'static str, i32>> = Lazy::new(|| {
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
        ("H-12", 5),
        ("H70", 5),
        ("H75", 5),
        ("H80", 6),
        ("H85", 6),
        ("H90", 6),
        ("D10B", 8),
        ("D-10", 8),
        ("D-12", 5),
        ("D60", 5),
        ("D65", 5),
        ("D70", 6),
        ("D75", 6),
        ("D80", 6),
        ("D85", 6),
        ("D90", 6),
    ]))
});

/// Courses using color codes as used from 2025.
static COURSES_COLORS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::<_, _>::from_iter(IntoIterator::into_iter([
        ("H-20", "H:Zwart Extra Lang"),
        ("H21", "H:Zwart Extra Lang"),
        ("H35", "H:Zwart Extra Lang"),
        ("H-18", "H:Zwart Lang"),
        ("H40", "H:Zwart Lang"),
        ("H45", "H:Zwart Lang"),
        ("H50", "H:Zwart Lang"),
        ("D-20", "D:Zwart Lang"),
        ("D21", "D:Zwart Lang"),
        ("H-16", "H:Zwart Midden"),
        ("H55", "H:Zwart Midden"),
        ("H60", "H:Zwart Midden"),
        ("D-16", "D:Zwart Midden"),
        ("D-18", "D:Zwart Midden"),
        ("D35", "D:Zwart Midden"),
        ("D40", "D:Zwart Midden"),
        ("D45", "D:Zwart Midden"),
        ("H-14", "H:Rood Midden"),
        ("H65", "H:Zwart Midden"),
        ("D-14", "D:Rood Midden"),
        ("D50", "D:Zwart Midden"),
        ("D55", "D:Zwart Midden"),
        ("H10B", "H:Groen Kort"),
        ("H-10", "H:Groen Kort"),
        ("H-12", "H:Blauw Kort"),
        ("H70", "H:Zwart Kort"),
        ("H75", "H:Zwart Kort"),
        ("H80", "H:Blauw Kort"),
        ("H85", "H:Blauw Kort"),
        ("H90", "H:Blauw Kort"),
        ("D10B", "D:Groen Kort"),
        ("D-10", "D:Groen Kort"),
        ("D-12", "D:Blauw Kort"),
        ("D60", "D:Zwart Kort"),
        ("D65", "D:Zwart Kort"),
        ("D70", "D:Blauw Kort"),
        ("D75", "D:Blauw Kort"),
        ("D80", "D:Blauw Kort"),
        ("D85", "D:Blauw Kort"),
        ("D90", "D:Blauw Kort"),
    ]))
});

#[derive(Debug, Deserialize)]
pub struct AgeClassOverride {
    pub cup: String,
    pub season: String,
    pub name: String,
    #[serde(rename = "ageclass")]
    pub age_class: String,
}

#[derive(Debug)]
pub struct Competitor {
    name: String,
    age_class: String,
}

impl Competitor {
    pub fn new(name: String, age_class: String) -> Self {
        Self { name, age_class }
    }
}

pub struct ResultProcessingOptions {
    pub cup: String,
    pub season: String,
    pub results_by_class: Option<bool>,
    pub overrides: Vec<AgeClassOverride>,
    pub competitors: Vec<Competitor>,
}

impl ResultProcessingOptions {
    pub fn validate_club(&self) -> bool {
        self.cup == "kampioen"
    }
}

pub fn create_database(db: &dyn Database) -> Result<(), anyhow::Error> {
    let conn = db.open()?;
    conn.pragma_update(None, "foreign_keys", "on")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
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
    db: &dyn Database,
    event: webres::Event,
    options: &ResultProcessingOptions,
) -> Result<(), anyhow::Error> {
    let conn = db.open()?;

    let event_db_id = prepare_event(&conn, &options.cup, &options.season, &event)?;
    if options.cup == "kampioen" || (options.results_by_class.unwrap_or(false)) {
        store_event_by_class(conn, event, options, event_db_id)?;
    } else {
        store_event_by_colored_course(conn, event, options, event_db_id)?;
    }

    Ok(())
}

fn prepare_event(
    conn: &Connection,
    cup: &str,
    season: &str,
    event: &webres::Event,
) -> Result<i64, anyhow::Error> {
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
    options: &ResultProcessingOptions,
    event_db_id: i64,
) -> anyhow::Result<()> {
    for category in event.categories.values() {
        if !COURSES_NUMBERED.contains_key(&category.name as &str)
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
                match result.age_class.as_ref() {
                    Some(age_class) => age_class,
                    None => {
                        let age_class = options
                            .competitors
                            .iter()
                            .find(|c| c.name == result.name)
                            .map(|c| &c.age_class);
                        match age_class {
                            Some(age_class) => age_class,
                            None => bail!("No age class for {}", result.name),
                        }
                    }
                }
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

fn store_event_by_colored_course(
    conn: Connection,
    event: webres::Event,
    options: &ResultProcessingOptions,
    event_db_id: i64,
) -> Result<(), anyhow::Error> {
    let all_courses: IndexSet<String> = [
        "Zwart Extra Lang",
        "Zwart Lang",
        "Zwart Midden",
        "Zwart Kort",
        "Rood Midden",
        "Blauw Kort",
        "Groen Kort",
    ]
    .iter()
    .map(|c| c.to_string())
    .collect();

    let course_re = regex::Regex::new(r"[H|D]:(.+)$").unwrap();
    let get_course_name = |name| {
        course_re
            .captures_iter(name)
            .next()
            .map(|g| g.get(1).unwrap().as_str())
    };

    for category in event.categories.values() {
        let course_name: &str = match get_course_name(&category.name) {
            Some(course_name) => course_name,
            None => {
                eprintln!("Skipping course {}", category.name);
                continue;
            }
        };

        let course_index = match all_courses.get_index_of(course_name) {
            Some(course_index) => course_index,
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
            let overridden_age_class =
                override_age_class(&options.overrides, &result.name, age_class);
            let age_class = overridden_age_class.as_ref();

            let result_course_name = match get_course_name(COURSES_COLORS[age_class]) {
                Some(result_course_name) => result_course_name,
                None => {
                    eprintln!(
                        "unable to find course name {} for result {}",
                        COURSES_COLORS[age_class], result.name
                    );
                    continue;
                }
            };
            let result_index = all_courses
                .get_index_of(result_course_name)
                .unwrap_or_else(|| panic!("unknown course: {}", result_course_name));

            if result_index < course_index {
                eprintln!(
                    "{} {} is running in incorrect course {}, should run {}",
                    result.name, age_class, course_name, COURSES_NUMBERED[age_class as &str]
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

fn override_age_class(overrides: &[AgeClassOverride], name: &str, age_class: &str) -> String {
    for age_class_override in overrides {
        if age_class_override.name == name {
            eprintln!(
                "Overriding age class of {} to {}",
                name, age_class_override.age_class
            );
            return age_class_override.age_class.to_owned();
        }
    }

    age_class.to_string()
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
    db: &dyn Database,
    cup: String,
    season: i16,
    age_class: String,
    events_count: usize,
) -> Result<Vec<RankingEntry>, anyhow::Error> {
    if cup == "kampioen" || season < 2023 || (cup == "forest-cup" && season == 2023) {
        rules_2022::calculate_ranking(db, cup, season, age_class, events_count)
    } else if season < 2024 || (cup == "forest-cup" && season == 2024) {
        rules_2023::calculate_ranking(db, cup, season, age_class, events_count)
    } else if season < 2026 {
        rules_2024::calculate_ranking(db, cup, season, age_class, events_count)
    } else {
        rules_2026::calculate_ranking(db, cup, season, age_class, events_count)
    }
}
