use std::fmt::Display;

use crate::types::{AirportCode, FlightNumber, GateCode, Size, flight_type::FlightType};

#[derive(Debug, Clone)]
pub struct Flight {
    pub number: FlightNumber,
    pub airport1: (AirportCode, GateCode),
    pub airport2: (AirportCode, GateCode),
    pub size: Size,
    pub score: i8,
    pub ty: FlightType,
}

impl Display for Flight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}): {} {} {} {} ({}, {})",
            self.number,
            self.size,
            self.airport1.0,
            self.airport1.1,
            self.airport2.0,
            self.airport2.1,
            self.score,
            self.ty
        )
    }
}
