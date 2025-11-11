use anyhow::Result;
use itertools::Itertools;

use crate::{
    Config,
    types::{flight::Flight, flight_type::FlightType},
};

pub fn get_stats(res: &[Flight], config: &mut Config) -> Result<String> {
    let flights = res.len();
    let flight_pairs = res.len() / 2;
    let airports = config.airports()?.len();
    let gates = config.gates()?.len();
    let hubs = config.hubs()?;
    let hard_max_hub = config.hard_max_hub;
    let hard_max_nonhub = config.hard_max_nonhub;

    let full_gates = config.gates()?.into_iter().filter(|g| {
        res.iter()
            .filter(|f| f.airport1 == (g.airport.clone(), g.code.clone()))
            .count()
            >= if hubs.contains(&g.airport) {
                hard_max_hub
            } else {
                hard_max_nonhub
            } as usize
    });
    let empty_gates = config.gates()?.into_iter().filter(|g| {
        res.iter()
            .filter(|f| f.airport1 == (g.airport.clone(), g.code.clone()))
            .count()
            == 0
    });
    let duped_flights = res
        .iter()
        .filter(|f| {
            [
                FlightType::ExistingH2H,
                FlightType::ExistingH2N,
                FlightType::ExistingN2N,
            ]
            .contains(&f.ty)
        })
        .count();
    Ok(format!(
        "==Flight Stats==\n\
        Flights: {} ({} pairs)\n\
        Destinations: {}\n\
        Flight:Destination ratio: {:.2}\n\
        Gates: {}\n\
        Full gates: {}\n\
        Empty gates: {}\n\
        % duplicates: {:.2}\n\
        ",
        flights,
        flight_pairs,
        airports,
        flight_pairs as f64 / airports as f64,
        gates,
        full_gates
            .map(|f| f.to_string())
            .sorted()
            .collect::<Vec<_>>()
            .join(", "),
        empty_gates
            .map(|f| f.to_string())
            .sorted()
            .collect::<Vec<_>>()
            .join(", "),
        duped_flights as f64 / flights as f64 * 100.0
    ))
}
