// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use structopt::StructOpt;

use ov_cup::calculate_ranking;
use ov_cup::cli;

#[derive(StructOpt, Debug)]
#[structopt(name = "ranking")]
struct Opt {
    #[structopt(long, default_value = "forest-cup", parse(try_from_str = cli::parse_cup))]
    cup: String,

    #[structopt(long, default_value = "2020")]
    season: String,

    #[structopt(long, default_value = "H35")]
    age_class: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let ranking = calculate_ranking(opt.cup, opt.season, opt.age_class)?;
    dbg!(ranking);
    Ok(())
}
