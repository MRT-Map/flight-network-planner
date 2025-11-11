use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use gatelogue_types::{GatelogueData, World};
use itertools::Itertools;
use log::{debug, info, warn};

use crate::types::{AirlineName, AirportCode, SmolStr, config::Config};

#[expect(dead_code)]
#[derive(Debug)]
pub struct FlightDataFlight {
    pub airline: AirlineName,
    pub flight_number: SmolStr,
    pub airports: Vec<AirportCode>,
}

#[expect(dead_code)]
#[derive(Debug)]
pub struct FlightData {
    pub flights: Vec<FlightDataFlight>,
    pub old_world_airports: Vec<AirportCode>,
    pub new_world_airports: Vec<AirportCode>,
    pub timestamp: u64,
}
impl FlightData {
    pub fn from_gatelogue() -> Result<Self> {
        info!("Downloading gatelogue data");
        let data = GatelogueData::ureq_get_no_sources()?;

        info!("Processing gatelogue data");
        let flights = data
            .nodes
            .values()
            .filter_map(|a| a.as_air_flight())
            .map(|a| {
                let airline_name = data.get_air_airline(*a.airline)?.name.clone().into();

                let flight_number = a.codes.first().ok_or_else(|| anyhow!("No codes"))?.into();

                let airport_codes = a
                    .gates
                    .iter()
                    .map(|a| {
                        let airport_id = *data.get_air_gate(**a)?.airport;
                        Ok(data.get_air_airport(airport_id)?.code.clone().into())
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(FlightDataFlight {
                    airline: airline_name,
                    flight_number,
                    airports: airport_codes,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let old_world_airports = data
            .nodes
            .values()
            .filter_map(|a| a.as_air_airport())
            .filter(|a| a.common.world.as_ref().is_some_and(|a| **a == World::Old))
            .map(|a| a.code.clone().into())
            .collect();

        let new_world_airports = data
            .nodes
            .values()
            .filter_map(|a| a.as_air_airport())
            .filter(|a| a.common.world.as_ref().is_none_or(|a| **a == World::New))
            .map(|a| a.code.clone().into())
            .collect();

        Ok(Self {
            flights,
            old_world_airports,
            new_world_airports,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        })
    }
    pub fn preprocess(&mut self, config: &mut Config) -> Result<()> {
        info!("Preprocessing flight data");
        debug!("Throwing out ignored airlines");
        self.flights
            .retain(|f| !config.ignored_airlines().contains(&f.airline));

        debug!("Checking airport codes");
        config
            .gates()?
            .iter()
            .map(|g| g.airport.clone())
            .sorted()
            .dedup()
            .filter(|a| {
                !self.new_world_airports.contains(a) && !self.old_world_airports.contains(a)
            })
            .for_each(|a| {
                warn!("Airport `{a}` doesn't exist");
            });

        let airports = config.airports()?;
        config
            .hubs()?
            .into_iter()
            .filter(|a| !airports.iter().contains(a))
            .for_each(|a| {
                warn!("Airport `{a}` has no gates but is stated as a hub");
            });

        debug!("Ensuring flight number allocations for hubs");
        let fnr_not_specified = config
            .hubs()?
            .into_iter()
            .filter(|a| !config.range_h2n.keys().contains(a))
            .collect::<Vec<_>>();
        if !fnr_not_specified.is_empty() {
            return Err(anyhow!(
                "Flight number range not specified for: {}",
                fnr_not_specified.join(", ")
            ));
        }
        Ok(())
    }
    pub fn num_flights(&self, airport1: &AirportCode, airport2: &AirportCode) -> usize {
        self.flights
            .iter()
            .filter(|f| f.airports.contains(airport1) && f.airports.contains(airport2))
            .count()
    }
}
