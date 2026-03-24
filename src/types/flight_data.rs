use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use gatelogue_types::{AirAirport, AirFlight, GD, LocatedNode, World, getter};
use itertools::Itertools;
use log::{debug, info, warn};

use crate::types::{AirlineName, AirportCode, SmolStr, config::Config};

#[expect(dead_code)]
#[derive(Debug)]
pub struct FlightDataFlight {
    pub airline: AirlineName,
    pub flight_number: SmolStr,
    pub from_airport: AirportCode,
    pub to_airport: AirportCode,
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
        let gd = GD::get_no_sources(getter!(ureq))?;

        info!("Processing gatelogue data");
        let flights = gd
            .nodes_of_type::<AirFlight>()?
            .into_iter()
            .map(|af| {
                Ok(FlightDataFlight {
                    airline: af.airline(&gd)?.name(&gd)?.into(),
                    flight_number: af.code(&gd)?.into(),
                    from_airport: af.from(&gd)?.airport(&gd)?.code(&gd)?.into(),
                    to_airport: af.to(&gd)?.airport(&gd)?.code(&gd)?.into(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut old_world_airports = vec![];
        let mut new_world_airports = vec![];
        for airport in gd.nodes_of_type::<AirAirport>()? {
            if airport.world(&gd)?.is_some_and(|a| a == World::Old) {
                old_world_airports.push(airport.code(&gd)?.into());
            } else {
                new_world_airports.push(airport.code(&gd)?.into());
            }
        }

        Ok(Self {
            flights,
            old_world_airports,
            new_world_airports,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        })
    }
    pub fn preprocess(&mut self, config: &Config) -> Result<()> {
        info!("Preprocessing flight data");
        debug!("Throwing out ignored airlines");
        self.flights
            .retain(|f| !config.ignored_airlines().contains(&f.airline));

        debug!("Checking airport codes");
        config
            .airports()
            .filter(|a| {
                !self.new_world_airports.contains(a) && !self.old_world_airports.contains(a)
            })
            .for_each(|a| {
                warn!("Airport `{a}` doesn't exist");
            });

        let airports = config.airports().collect::<Vec<_>>();
        config
            .hubs()
            .filter(|a| !airports.iter().contains(a))
            .for_each(|a| {
                warn!("Airport `{a}` has no gates but is stated as a hub");
            });

        debug!("Ensuring flight number allocations for hubs");
        let fnr_not_specified = config
            .hubs()
            .filter(|a| !config.range_h2n.keys().contains(a))
            .collect::<Vec<_>>();
        if !fnr_not_specified.is_empty() {
            return Err(anyhow!(
                "Flight number range not specified for: {}",
                fnr_not_specified.into_iter().join(", ")
            ));
        }
        Ok(())
    }
    pub fn num_flights(&self, from: &AirportCode, to: &AirportCode) -> usize {
        self.flights
            .iter()
            .filter(|f| f.from_airport == *from && f.to_airport == *to)
            .count()
    }
}
