use std::fmt::Display;

use crate::types::{AirportCode, config::Config, flight_data::FlightData, gate::Gate};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub enum FlightType {
    NonExistingH2H,
    ExistingH2H,
    NonExistingH2N,
    NonExistingN2N,
    ExistingH2N,
    ExistingN2N,
}

impl Display for FlightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonExistingH2H => write!(f, "H2Hn"),
            Self::ExistingH2H => write!(f, "H2He"),
            Self::NonExistingH2N => write!(f, "H2Nn"),
            Self::NonExistingN2N => write!(f, "N2Nn"),
            Self::ExistingH2N => write!(f, "H2Ne"),
            Self::ExistingN2N => write!(f, "N2Ne"),
        }
    }
}

impl Config {
    pub fn airports_flight_type(
        &mut self,
        flight_data: &FlightData,
        a1: &AirportCode,
        a2: &AirportCode,
    ) -> anyhow::Result<FlightType> {
        let hubs = self.hubs()?;
        Ok(if hubs.contains(a1) {
            if hubs.contains(a2) {
                if flight_data.num_flights(a1, a2) > 0 {
                    FlightType::ExistingH2H
                } else {
                    FlightType::NonExistingH2H
                }
            } else if flight_data.num_flights(a1, a2) > 0 {
                FlightType::ExistingH2N
            } else {
                FlightType::NonExistingH2N
            }
        } else if hubs.contains(a2) {
            if flight_data.num_flights(a1, a2) > 0 {
                FlightType::ExistingH2N
            } else {
                FlightType::NonExistingH2N
            }
        } else if flight_data.num_flights(a1, a2) > 0 {
            FlightType::ExistingN2N
        } else {
            FlightType::NonExistingN2N
        })
    }
    pub fn gates_flight_type(
        &mut self,
        flight_data: &FlightData,
        g1: &Gate,
        g2: &Gate,
    ) -> anyhow::Result<FlightType> {
        self.airports_flight_type(flight_data, &g1.airport, &g2.airport)
    }
}
