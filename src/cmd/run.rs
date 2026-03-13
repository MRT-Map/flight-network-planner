use std::collections::HashMap;

use anyhow::{Result, anyhow};
use itertools::Itertools;
use log::{debug, info, trace};
use rayon::prelude::*;

use crate::{
    FlightData,
    types::{
        AirportCode, config::Config, flight::Flight, flight_type::FlightType,
        fng::FlightNumberGenerator, gate::Gate,
    },
    utils::{AnyAllBool, for_both, for_both_permutations},
};

#[expect(clippy::too_many_lines)]
pub fn run(
    config: &Config,
    fd: &FlightData,
    old_plan: Option<&Vec<Flight>>,
) -> Result<Vec<Flight>> {
    let hubs = config.hubs().collect::<Vec<_>>();
    let possible_flights = config
        .gates
        .iter()
        .tuple_combinations::<(_, _)>()
        .par_bridge()
        .filter(|(g1, g2)| config.is_valid_flight(fd, g1, g2));

    let mut h2h_fng = FlightNumberGenerator::new(config.range_h2h.clone());
    let mut h2n_fng = HashMap::new();
    let mut n2n_fng = FlightNumberGenerator::new(config.range_n2n.clone());

    let mut destinations: HashMap<&Gate, Vec<AirportCode>> = HashMap::new();
    let mut flights: Vec<Flight> = vec![];

    let sorted_flights = possible_flights
        .map(|(g1, g2)| {
            let s = config.gates_score(fd, g1, g2);
            let ty = config.gates_flight_type(fd, g1, g2);
            let existed = old_plan.is_some_and(|old_plan| {
                old_plan
                    .iter()
                    .filter(|f| {
                        (f.airport1 == (g1.airport.clone(), g1.code.clone())
                            && f.airport2 == (g2.airport.clone(), g2.code.clone()))
                            || (f.airport1 == (g2.airport.clone(), g2.code.clone())
                                && f.airport2 == (g1.airport.clone(), g1.code.clone()))
                    })
                    .count()
                    > 0
            });
            (g1, g2, s, ty, existed)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|&(_, _, s1, _, existed1), &(_, _, s2, _, existed2)| {
            let s1 = if existed1 { s1 + 1 } else { s1 };
            let s2 = if existed2 { s2 + 1 } else { s2 };
            s2.cmp(&s1)
        })
        .map(|(g1, g2, s, ty, _)| (g1, g2, s, ty))
        .collect::<Vec<_>>();

    for (mut g1, mut g2, mut s, ty) in sorted_flights {
        if hubs.contains(&&g2.airport) && !hubs.contains(&&g1.airport) {
            (g1, g2) = (g2, g1);
        }
        if for_both(&g1, &g2, |g| {
            destinations.get(g).map_or(0, Vec::len)
                >= *config
                    .max_dests_per_gate
                    .get(&g.airport)
                    .unwrap_or(&u8::MAX) as usize
        })
        .any()
        {
            continue;
        }
        s -= (destinations.get(&g1).map_or(0, Vec::len) as i8)
            .min(destinations.get(&g2).map_or(0, Vec::len) as i8);
        if s < 0 {
            continue;
        }
        let (max1, max2) = for_both(&g1, &g2, |g| match ty {
            FlightType::ExistingH2H | FlightType::NonExistingH2H => config.max_h2h,
            FlightType::ExistingH2N | FlightType::NonExistingH2N => {
                if hubs.contains(&&g.airport) {
                    config.max_h2n_hub
                } else {
                    config.max_h2n_nonhub
                }
            }
            FlightType::ExistingN2N | FlightType::NonExistingN2N => config.max_n2n,
        });

        if flights.iter().any(|f| {
            (f.airport1.0 == g1.airport && f.airport2.0 == g2.airport)
                || (f.airport1.0 == g2.airport && f.airport2.0 == g1.airport)
        }) {
            trace!(
                "Rejected ({} {}): {} {} <-> {} {} (already exists)",
                ty, g1.size, g1.airport, g1.code, g2.airport, g2.code
            );
            continue;
        }

        let (g1_hardmax, g2_hardmax) = for_both(g1, g2, |g| {
            config.max_dests_per_gate.get(&g.airport).map_or_else(
                || {
                    if hubs.contains(&&g.airport) {
                        config.hard_max_hub
                    } else {
                        config.hard_max_nonhub
                    }
                },
                |n| *n,
            ) as usize
        });
        if for_both_permutations(
            &(g1, &g1_hardmax),
            &(g2, &g2_hardmax),
            |(g, hardmax), (og, _)| {
                if destinations.get(g).map_or(0, Vec::len) >= **hardmax {
                    debug!(
                        "Rejected ({} {}): {} {} <-> {} {} ({2} hit max limit of {})",
                        ty, og.size, g.airport, g.code, og.airport, og.code, hardmax
                    );
                    true
                } else {
                    false
                }
            },
        )
        .any()
        {
            continue;
        }
        if for_both_permutations(&(g1, max1), &(g2, max2), |(g, max), (og, _)| {
            if destinations.get(g).map_or(0, |ds| {
                ds.iter()
                    .filter(|d| config.airports_flight_type(fd, &g.airport, d) == ty)
                    .count()
            }) >= *max as usize
            {
                debug!(
                    "Rejected ({} {}): {} {} <-> {} {} ({2} hit max type limit of {})",
                    ty, og.size, g.airport, g.code, og.airport, og.code, max
                );
                true
            } else {
                false
            }
        })
        .any()
        {
            continue;
        }

        for_both_permutations(&g1, &g2, |g1, g2| {
            destinations.entry(g1).or_default().push(g2.airport.clone());
        });
        let fng = match ty {
            FlightType::ExistingH2H | FlightType::NonExistingH2H => &mut h2h_fng,
            FlightType::ExistingH2N | FlightType::NonExistingH2N => h2n_fng
                .entry(
                    (if config.range_h2n.contains_key(&*g1.airport.clone()) {
                        g1
                    } else {
                        g2
                    })
                    .airport
                    .clone(),
                )
                .or_insert_with(|| {
                    FlightNumberGenerator::new(
                        config
                            .range_h2n
                            .get(&*g1.airport.clone())
                            .unwrap_or_else(|| &config.range_h2n[&*g2.airport.clone()])
                            .to_owned(),
                    )
                }),
            FlightType::ExistingN2N | FlightType::NonExistingN2N => &mut n2n_fng,
        };

        let fn1 = fng.find(|a| !flights.iter().map(|f| f.number).contains(a));
        let fn2 = if config.both_dir_same_num {
            fn1
        } else {
            fng.find(|a| !flights.iter().map(|f| f.number).contains(a))
        };

        let (flight1, flight2) =
            for_both_permutations(&(&g1, fn1), &(&g2, fn2), |(g1, fn1), (g2, _)| {
                let flight = Flight {
                    number: if let Some(fn_) = fn1 {
                        fn_.to_owned()
                    } else {
                        return Err(anyhow!(
                            "Could not generate flight number for {} -> {}",
                            g1.airport,
                            g2.airport
                        ));
                    },
                    airport1: (g1.airport.clone(), g1.code.clone()),
                    airport2: (g2.airport.clone(), g2.code.clone()),
                    size: g1.size.clone(),
                    score: s,
                    ty,
                };
                info!(
                    "{} ({} {}): {} {} -> {} {}, {}",
                    flight.number, ty, g1.size, g1.airport, g1.code, g2.airport, g2.code, s
                );
                flights.push(flight.clone());
                Ok(flight)
            });
        flight1?;
        flight2?;
        //possible_flights = sort_gates(possible_flights, config, fd)?;
    }

    Ok(flights)
}
