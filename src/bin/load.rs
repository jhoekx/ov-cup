// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use ov_cup::db::LocalDatabase;
use ov_cup::iof;
use ov_cup::AgeClassOverride;
use ov_cup::Competitor;

use ov_cup::cli;
use ov_cup::webres;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "forest-cup", value_parser = cli::parse_cup)]
    cup: String,

    #[arg(long)]
    season: String,

    #[arg(name = "FILE")]
    paths: Vec<String>,

    #[arg(long)]
    by_class: Option<bool>,

    #[arg(long, default_value = "overrides.json")]
    overrides: String,

    #[arg(long)]
    competitor_list: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let cup = args.cup.to_owned();
    let season = args.season.to_owned();
    let overrides = read_overrides_json(args.overrides)?
        .into_iter()
        .filter(|age_class_override| {
            age_class_override.cup == cup && age_class_override.season == season
        })
        .collect();
    let competitors = read_competitor_lists(&args.competitor_list)?;
    let options = ov_cup::ResultProcessingOptions {
        cup,
        season,
        results_by_class: args.by_class,
        overrides,
        competitors,
    };

    let db_path = PathBuf::from("ov.sqlite");
    let db = LocalDatabase::new(db_path);
    ov_cup::create_database(&db)?;

    for path in args.paths {
        let event = webres::read_event_json(path)?;
        ov_cup::store_event(&db, event, &options)?;
    }

    Ok(())
}

fn read_overrides_json(path: String) -> Result<Vec<AgeClassOverride>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let overrides = serde_json::from_reader(reader)?;
    Ok(overrides)
}

fn read_competitor_lists(paths: &[String]) -> anyhow::Result<Vec<Competitor>> {
    let mut competitors = vec![];
    for path in paths {
        let competitor_list = iof::parse_competitor_list(Path::new(path))?;
        for competitor in competitor_list.competitors {
            competitors.push(Competitor::new(
                format!(
                    "{} {}",
                    competitor.person.name.given, competitor.person.name.family
                ),
                competitor.class.name,
            ))
        }
    }
    Ok(competitors)
}
