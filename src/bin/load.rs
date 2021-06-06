// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use structopt::StructOpt;

use ov_cup::cli;
use ov_cup::webres;

#[derive(StructOpt, Debug)]
#[structopt(name = "load")]
struct Opt {
    #[structopt(long, default_value = "forest-cup", parse(try_from_str = cli::parse_cup))]
    cup: String,

    #[structopt(long)]
    season: String,

    #[structopt(name = "FILE")]
    paths: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    ov_cup::create_database()?;
    for path in opt.paths {
        let event = webres::read_event_json(path)?;
        ov_cup::store_event(opt.cup.to_owned(), opt.season.to_owned(), event)?;
    }

    Ok(())
}
