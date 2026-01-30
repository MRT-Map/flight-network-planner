mod cmd;
mod types;
mod utils;

use std::collections::HashMap;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete_command::Shell;
use itertools::Itertools;
use patharg::InputArg;
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
    #[clap(default_value_t)]
    file: InputArg,
    /// Whether to print statistics
    #[clap(short, long, action)]
    stats: bool,
    /// The old output file
    /// (will be used to preserve original flight routes so it won't duplicate so much)
    #[clap(short, long, value_parser)]
    old: Option<InputArg>,
    /// Whether to replace the old file instead of printing to stdout
    #[clap(short, long, action)]
    replace: bool,
}

#[derive(Parser)]
struct GateKeys {
    /// The flight-plan
    plan: InputArg,
}

fn main() -> Result<()> {
    pretty_env_logger::try_init()?;
    let args = Args::parse();
    match args.command {
        Command::Run(run) => {
            let config = Config::load(&run.file)?;
            let mut fd = FlightData::from_gatelogue()?;
            fd.preprocess(&config)?;
            let old_plan = if let Some(old) = &run.old {
                Some(Flight::vec_from_str(&old.read_to_string()?)?)
            } else {
                None
            };
            let mut result = run::run(&config, &fd, old_plan.as_ref())?;
            if run.stats {
                eprintln!("\n{}", stats::get_stats(&result, &config));
            }
            if let Some(old_plan) = &old_plan {
                result = update::update(old_plan, result, &config);
            }
            let result_string = Flight::vec_to_string(&result);
            if run.replace
                && let Some(old) = &run.old
                && let Some(path) = old.path_ref()
            {
                std::fs::write(path, result_string)?;
                println!("Overwritten {}", path.display());
            } else {
                println!("{result_string}");
            }
        }
        Command::GetConfig => {
            println!("{}", include_str!("../data/default_config.yml"));
        }
        Command::GateKeys(gate_keys) => {
            let flights = Flight::vec_from_str(&gate_keys.plan.read_to_string()?)?;
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
