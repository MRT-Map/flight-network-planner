use smol_str::SmolStr;

pub mod config;
pub mod flight;
pub mod flight_data;
pub mod flight_type;
pub mod fng;
pub mod gate;
pub mod score;

pub type AirlineName = SmolStr;
pub type AirportCode = SmolStr;
pub type GateCode = SmolStr;
pub type FlightNumber = u16;
pub type Size = SmolStr;
