use chrono::{DateTime, NaiveTime, Timelike, Utc};
use itertools::Itertools;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug)]
struct Performance {
    name: String,
    club: String,
    event_id: u64,
    event_name: String,
    event_date: DateTime<Utc>,
    age_class: String,
    category_name: String,
    time: NaiveTime,
    score: u32,
}

fn total_seconds(time: impl Timelike) -> u32 {
    time.second() + time.minute() * 60 + time.hour() * 60 * 60
}

#[derive(Error, Debug)]
enum ArgumentsError {
    #[error("Invalid cup, valid cups are: city-cup, forest-cup")]
    UnknownCup,
}

fn parse_cup(flag: &str) -> Result<String, ArgumentsError> {
    if flag == "city-cup" || flag == "forest-cup" {
        Ok(flag.to_owned())
    } else {
        Err(ArgumentsError::UnknownCup)
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "ranking")]
struct Opt {
    #[structopt(long, default_value = "forest-cup", parse(try_from_str = parse_cup))]
    cup: String,

    #[structopt(long, default_value = "2020")]
    season: String,

    #[structopt(long, default_value = "H35")]
    age_class: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let conn = Connection::open("ov.sqlite")?;

    // Find all results of all runners with at least one ranking in the given category
    let mut stmt = conn.prepare(
        "
        select
            Runner.name,
            Runner.club,
            Event.id,
            Event.name,
            Event.date,
            Result.age_class,
            Result.category_name,
            Result.time
        from Result join Runner on Result.runner_id = Runner.id
                    join Event on Result.event_id = Event.id
        where Event.cup = ? and Event.season = ?
          and Runner.id in (
              select Runner.id
              from Runner join Result on Runner.id = Result.Runner_id
              where Result.age_class = ?
          )
    ",
    )?;
    let results: Vec<Performance> = stmt
        .query_map(params![opt.cup, opt.season, opt.age_class], |row| {
            Ok(Performance {
                name: row.get(0)?,
                club: row.get(1)?,
                event_id: row.get(2)?,
                event_name: row.get(3)?,
                event_date: row.get(4)?,
                age_class: row.get(5)?,
                category_name: row.get(6)?,
                time: row.get(7)?,
                score: 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // TODO: Keep only courses that are valid for the age class of the result

    // TODO: Keep only results of runners where the last age class equals the given age class
    //       This filters out runners who moved to a different category,
    //       while keeping the runners that moved into this category.

    // Find the best results in all courses that someone of the given age class participated in
    let courses: Vec<(u64, String)> = results
        .iter()
        .map(|result| (result.event_id, result.category_name.to_string()))
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
            stmt.query_row(params![event_id, category_name], |row| Ok(row.get(0)?))?;
        fastest_times.insert((event_id, category_name), total_seconds(fastest_time));
    }

    for mut result in results {
        result.score = 1000
            * fastest_times
                .get(&(result.event_id, result.category_name.to_string()))
                .unwrap()
            / total_seconds(result.time);
        dbg!(result);
    }

    // create intermediate rankings to show result evolution

    Ok(())
}
