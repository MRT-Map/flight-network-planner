use std::collections::HashMap;


use crate::{
    Config,
    types::{flight::Flight, flight_type::FlightType, fng::FlightNumberGenerator},
};

pub fn update(
    old_plan: &[Flight],
    generated_plan: Vec<Flight>,
    config: &Config,
) -> Vec<Flight> {
    let mut new_plan = vec![];
    let mut used_flight_numbers = vec![];
    let mut flight_number_mapping = HashMap::new();
    let mut new_flights = vec![];

    let mut h2h_fng = FlightNumberGenerator::new(config.range_h2h.clone());
    let mut h2n_fng = HashMap::new();
    let mut n2n_fng = FlightNumberGenerator::new(config.range_n2n.clone());

    for flight in generated_plan {
        if let Some(old_flight) = old_plan
            .iter()
            .find(|f| f.airport1 == flight.airport1 && f.airport2 == flight.airport2)
        {
            used_flight_numbers.push(old_flight.number.to_owned());
            new_plan.push(Flight {
                number: old_flight.number,
                airport1: flight.airport1,
                airport2: flight.airport2,
                size: flight.size,
                score: flight.score,
                ty: flight.ty,
            });
        } else {
            new_flights.push(flight);
        }
    }

    for flight in new_flights {
        let flight_number = flight_number_mapping
            .entry(flight.number)
            .or_insert_with(|| {
                let fng = match flight.ty {
                    FlightType::ExistingH2H | FlightType::NonExistingH2H => &mut h2h_fng,
                    FlightType::ExistingH2N | FlightType::NonExistingH2N => h2n_fng
                        .entry(
                            (if config.range_h2n.contains_key(&*flight.airport1.0.clone()) {
                                &flight.airport1.1
                            } else {
                                &flight.airport2.1
                            })
                            .to_owned(),
                        )
                        .or_insert_with(|| {
                            FlightNumberGenerator::new(
                                config
                                    .range_h2n
                                    .get(&*flight.airport1.0.clone())
                                    .unwrap_or_else(|| {
                                        &config.range_h2n[&*flight.airport2.0.clone()]
                                    })
                                    .to_owned(),
                            )
                        }),
                    FlightType::ExistingN2N | FlightType::NonExistingN2N => &mut n2n_fng,
                };

                let mut fn_ = fng.next();
                while used_flight_numbers.contains(&fn_.unwrap()) {
                    fn_ = fng.next();
                }
                fn_.unwrap()
            })
            .to_owned();
        used_flight_numbers.push(flight_number.to_owned());
        new_plan.push(Flight {
            number: flight_number,
            airport1: flight.airport1,
            airport2: flight.airport2,
            size: flight.size,
            score: flight.score,
            ty: flight.ty,
        });
    }
    new_plan
}
