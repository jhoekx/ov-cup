// SPDX-FileCopyrightText: 2023 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use anyhow::bail;
use itertools::Itertools;
use regex::Regex;
use rusqlite::params;

use crate::{db::Database, total_seconds, Performance, RankingEntry, RankingScore, COURSES};

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

    let (age_class, course) = get_course(&age_class)?;
    let performance_filter = PerformanceFilter::new(age_class);

    // Find all results in the courses of the requested category
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
        where Event.cup = ?
          and Event.season = ?
          and Result.category_name = ?
        order by Runner.name asc, Event.date asc
    ",
    )?;
    let results: Vec<Performance> = stmt
        .query_map(params![cup, season, course], |row| {
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
        .filter_map(|r| r.ok())
        .filter(|r| !performance_filter.should_ignore(&r.age_class))
        .collect();

    // Find the best results in all courses that someone of the given age class participated in
    let mut fastest_times = HashMap::new();
    for result in &results {
        let course = (result.event_id, result.category_name.to_owned());
        let result_seconds = total_seconds(result.time);
        match fastest_times.get(&course) {
            Some(fastest_time) => {
                if result_seconds < *fastest_time {
                    fastest_times.insert(course, result_seconds);
                }
            }
            None => {
                fastest_times.insert(course, result_seconds);
            }
        }
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

fn get_course(age_class: &str) -> anyhow::Result<(String, String)> {
    if age_class.contains('|') {
        let re = Regex::new(r"^(H|D)(.*)\|(\d)")?;
        if let Some(groups) = re.captures(age_class) {
            let effective_class = format!("{}{}", &groups[1], &groups[2]);
            let effective_course = format!("{}:0{}", &groups[1], &groups[3]);
            return Ok((effective_class, effective_course));
        }
    }

    match age_class.chars().next() {
        Some(gender) => match COURSES.get(age_class) {
            Some(course) => Ok((age_class.to_owned(), format!("{}:0{}", gender, course))),
            None => bail!("age class not in courses"),
        },
        None => bail!("unknown course prefix"),
    }
}

// Ignore results of "higher" age classes
// For H35, ignore H21 and H-20
// For H-20, ignore H-21 and H-35
struct PerformanceFilter {
    age_class: String,
    re: Regex,
}

impl PerformanceFilter {
    fn new(age_class: String) -> Self {
        let re = Regex::new(r"(\d{2})$").unwrap();
        PerformanceFilter { age_class, re }
    }

    fn should_ignore(&self, other_age_class: &str) -> bool {
        let age = self.get_age(&self.age_class);
        let other_age = self.get_age(other_age_class);

        if age <= 20 {
            if other_age > 20 {
                return true;
            }
            if other_age <= age {
                return false;
            }
        } else {
            if other_age <= 20 {
                return true;
            }
            if other_age >= age {
                return false;
            }
        }

        true
    }

    fn get_age(&self, age_class: &str) -> i16 {
        let captures = self.re.captures(age_class).unwrap();
        captures[1].parse::<i16>().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{get_course, PerformanceFilter};

    #[test]
    fn course() {
        assert_eq!(
            get_course("H-18").unwrap(),
            ("H-18".to_string(), "H:02".to_string())
        );
        assert_eq!(
            get_course("H-12|5").unwrap(),
            ("H-12".to_string(), "H:05".to_string())
        );
    }

    #[test]
    fn filter_d50() {
        let filter = PerformanceFilter::new("D50".to_owned());
        assert!(filter.should_ignore("D45"));
        assert!(filter.should_ignore("D-20"));

        assert!(!filter.should_ignore("D50"));
        assert!(!filter.should_ignore("D55"));
    }

    #[test]
    fn filter_h20() {
        let filter = PerformanceFilter::new("H-20".to_owned());
        assert!(filter.should_ignore("H21"));
        assert!(filter.should_ignore("H35"));
        assert!(filter.should_ignore("H40"));

        assert!(!filter.should_ignore("H-20"));
        assert!(!filter.should_ignore("H-18"));
    }
}
