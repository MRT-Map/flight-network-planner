mod cmd;
mod types;
mod utils;

use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete_command::Shell;
use itertools::Itertools;
use types::config::Config;

use crate::{
    cmd::{run, stats, update},
    types::{flight::Flight, flight_data::FlightData},
};

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}
#[derive(Parser)]
enum Command {
    /// Run the planner
    Run(Run),
    /// Gets the configuration for the planner
    GetConfig,
    /// Tool to format the output of `run` as a mapping of gates to destinations
    GateKeys(GateKeys),
    /// Generate a completion file for your shell
    Completion {
        /// The shell to generate for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Parser)]
struct Run {
    /// The configuration YML file to read from
    file: PathBuf,
    /// Whether to print statistics
    #[clap(short, long, action)]
    stats: bool,
    /// The old output file
    /// (will be used to preserve original flight routes so it won't duplicate so much)
    #[clap(short, long, value_parser)]
    old: Option<PathBuf>,
    /// Whether to replace the old file instead of printing to stdout
    #[clap(short, long, action)]
    replace: bool,
}

#[derive(Parser)]
struct GateKeys {
    /// The output file from `run`
    #[clap(default_value = "out.txt")]
    out_file: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::try_init()?;
    let args = Args::parse();
    match args.command {
        Command::Run(run) => {
            let mut config: Config = serde_yaml::from_reader(std::fs::File::open(&run.file)?)?;
            config._folder = run.file.parent().map(ToOwned::to_owned);
            let mut fd = FlightData::from_gatelogue()?;
            fd.preprocess(&mut config)?;
            let old_plan = if let Some(old) = &run.old {
                Some(Flight::vec_from_str(&std::fs::read_to_string(old)?)?)
            } else {
                None
            };
            let mut result = run::run(&mut config, &fd, old_plan.as_ref())?;
            if run.stats {
                eprintln!("\n{}", stats::get_stats(&result, &mut config)?);
            }
            if let Some(old_plan) = &old_plan {
                result = update::update(old_plan, result, &config);
            }
            let result_string = Flight::vec_to_string(&result);
            if run.replace {
                if let Some(old) = &run.old {
                    std::fs::write(old, result_string)?;
                    println!("Overwritten {}", old.display());
                } else {
                    println!("{result_string}");
                }
            } else {
                println!("{result_string}");
            }
        }
        Command::GetConfig => {
            println!("{}", include_str!("../data/default_config.yml"));
        }
        Command::GateKeys(gate_keys) => {
            let flights = Flight::vec_from_str(&std::fs::read_to_string(&gate_keys.out_file)?)?;
            let mut map: HashMap<_, Vec<_>> = HashMap::new();
            for flight in flights {
                map.entry(flight.airport1)
                    .or_default()
                    .push((flight.airport2, flight.number));
            }
            let res = map
                .iter()
                .map(|((ka, kg), vs)| {
                    format!(
                        "{} {}: {}",
                        ka,
                        kg,
                        vs.iter()
                            .map(|((va, vg), num)| format!("{num} {va} {vg}"))
                            .join(", ")
                    )
                })
                .sorted()
                .join("\n");
            println!("{res}");
        }
        Command::Completion { shell } => {
            shell.generate(&mut Args::command(), &mut std::io::stdout());
        }
    }
    Ok(())
}
