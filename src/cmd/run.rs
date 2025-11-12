use std::collections::HashMap;

use anyhow::{Result, anyhow};
use itertools::Itertools;
use log::{debug, info, trace};

use crate::{
    FlightData, fbp,
    types::{
        AirportCode, config::Config, flight::Flight, flight_type::FlightType,
        flight_utils::FlightUtils, fng::FlightNumberGenerator, gate::Gate,
    },
    utils::{AnyAllBool, for_both, for_both_permutations},
};

fn sort_gates(
    x: Vec<(Gate, Gate, i8, FlightType)>,
    config: &mut Config,
    fd: &FlightData,
    old_plan: Option<&Vec<Flight>>,
) -> Result<Vec<(Gate, Gate, i8, FlightType)>> {
    Ok(x.into_iter()
        .map(|(g1, g2, _, ty)| {
            let s = (&g1, &g2).score(config, fd)?;
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
            Ok((g1, g2, s, ty, existed))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .sorted_by(|&(_, _, s1, _, existed1), &(_, _, s2, _, existed2)| {
            let s1 = if existed1 { s1 + 1 } else { s1 };
            let s2 = if existed2 { s2 + 1 } else { s2 };
            s1.cmp(&s2)
        })
        .map(|(g1, g2, s, ty, _)| (g1, g2, s, ty))
        .collect::<Vec<_>>())
}

#[expect(clippy::too_many_lines)]
pub fn run(
    config: &mut Config,
    fd: &FlightData,
    old_plan: Option<&Vec<Flight>>,
) -> Result<Vec<Flight>> {
    let hubs = config.hubs()?;
    let restricted_between = config.restricted_between.clone();
    let restricted_to = config.restricted_to.clone();
    let gate_allowed_dests = config.gate_allowed_dests.clone();
    let gate_denied_dests = config.gate_denied_dests.clone();
    let preferred_between = config.preferred_between.clone();
    let preferred_to = config.preferred_to.clone();
    let no_dupes = config.no_dupes.clone();
    let mut possible_flights = config
        .gates()?
        .into_iter()
        .tuple_combinations::<(_, _)>()
        .filter(|(g1, g2)| {
            !restricted_between.iter().any(|re| {
                vec![g1.airport.clone(), g2.airport.clone()]
                    .into_iter()
                    .all(|a| re.contains(&a))
            })
        })
        .filter(|(g1, g2)| g1.airport != g2.airport && g1.size == g2.size)
        .filter(fbp!(
            filter | g1: &Gate,
            g2: &Gate | {
                !restricted_to
                    .get(&*g1.airport)
                    .unwrap_or(&vec![])
                    .contains(&g2.airport)
            }
        ))
        .filter(fbp!(
            filter | g1: &Gate,
            g2: &Gate | {
                gate_allowed_dests.get(&*g1.airport).is_none_or(|gates| gates.get(&*g1.code).is_none_or(|gate| gate.contains(&g2.airport)))
            }
        ))
        .filter(fbp!(
            filter | g1: &Gate,
            g2: &Gate | {
                gate_denied_dests.get(&*g1.airport).is_none_or(|gates| gates.get(&*g1.code).is_none_or(|gate| !gate.contains(&g2.airport)))
            }
        ))
        .map(|(g1, g2)| {
            let ty = (&g1, &g2).get_flight_type(config, fd)?;
            Ok((g1, g2, 0i8, ty))
        })
        .filter_ok(|(g1, g2, _, ty)| {
            if preferred_between
                .iter()
                .any(|a| a.contains(&g1.airport) && a.contains(&g2.airport))
                || preferred_to
                    .get(&g1.airport)
                    .is_some_and(|a| a.contains(&g2.airport))
                || preferred_to
                    .get(&g2.airport)
                    .is_some_and(|a| a.contains(&g1.airport))
            {
                true
            } else if no_dupes.contains(&g1.airport) || no_dupes.contains(&g2.airport) {
                ![
                    FlightType::ExistingH2H,
                    FlightType::ExistingH2N,
                    FlightType::ExistingN2N,
                ]
                .contains(ty)
            } else {
                true
            }
        })
        .collect::<Result<Vec<_>>>()?;

    let mut h2h_fng = FlightNumberGenerator::new(config.range_h2h.clone());
    let mut h2n_fng = HashMap::new();
    let mut n2n_fng = FlightNumberGenerator::new(config.range_n2n.clone());

    let mut destinations: HashMap<Gate, Vec<AirportCode>> = HashMap::new();
    let mut flights: Vec<Flight> = vec![];

    possible_flights = sort_gates(possible_flights, config, fd, old_plan)?;

    while let Some((mut g1, mut g2, mut s, ty)) = possible_flights.pop() {
        if hubs.contains(&g2.airport) && !hubs.contains(&g1.airport) {
            (g1, g2) = (g2.clone(), g1.clone());
        }
        if for_both(&g1, &g2, |g| {
            destinations.get(g).unwrap_or(&vec![]).len()
                >= *config
                    .max_dests_per_gate
                    .get(&g.airport)
                    .unwrap_or(&u8::MAX) as usize
        })
        .any()
        {
            continue;
        }
        s -= (destinations.get(&g1).unwrap_or(&vec![]).len() as i8)
            .min(destinations.get(&g2).unwrap_or(&vec![]).len() as i8);
        if s < 0 {
            continue;
        }
        let (max1, max2) = for_both(&g1, &g2, |g| match ty {
            FlightType::ExistingH2H | FlightType::NonExistingH2H => config.max_h2h,
            FlightType::ExistingH2N | FlightType::NonExistingH2N => {
                if hubs.contains(&g.airport) {
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

        let (g1_hardmax, g2_hardmax) = for_both(&g1, &g2, |g| {
            (if let Some(n) = config.max_dests_per_gate.get(&g.airport) {
                *n
            } else if hubs.contains(&g.airport) {
                config.hard_max_hub
            } else {
                config.hard_max_nonhub
            }) as usize
        });
        if for_both_permutations(
            &(&g1, &g1_hardmax),
            &(&g2, &g2_hardmax),
            |(g, hardmax), (og, _)| {
                if destinations.get(g).unwrap_or(&vec![]).len() >= **hardmax {
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
        if for_both_permutations(&(&g1, max1), &(&g2, max2), |(g, max), (og, _)| {
            if destinations
                .get(g)
                .unwrap_or(&vec![])
                .iter()
                .filter(|d| (&g.airport, *d).get_flight_type(config, fd).unwrap() == ty)
                .count()
                >= *max as usize
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
            destinations
                .entry(g1.to_owned())
                .or_default()
                .push(g2.airport.clone());
        });
        let fng = match ty {
            FlightType::ExistingH2H | FlightType::NonExistingH2H => &mut h2h_fng,
            FlightType::ExistingH2N | FlightType::NonExistingH2N => h2n_fng
                .entry(
                    (if config.range_h2n.contains_key(&*g1.airport.clone()) {
                        &g1
                    } else {
                        &g2
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
