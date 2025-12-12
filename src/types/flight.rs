use std::fmt::Display;

use anyhow::{Result, anyhow};
use itertools::Itertools;
use regex::Regex;

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

impl Flight {
    pub fn vec_to_string(vec: &[Self]) -> String {
        vec.iter()
            .sorted_by_key(|f| f.number)
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn vec_from_str(s: &str) -> Result<Vec<Self>> {
        let regex = Regex::new(r"(\d+) \((.*)\): (...) (.+) (...) (.+) \((\d+), (.2..)\)")?;
        s.split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| {
                Some({
                    let re = regex.captures(l)?;

                    Flight {
                        number: re.get(1)?.as_str().parse::<u16>().unwrap(),
                        airport1: (re.get(3)?.as_str().into(), re.get(4)?.as_str().into()),
                        airport2: (re.get(5)?.as_str().into(), re.get(6)?.as_str().into()),
                        size: re.get(2)?.as_str().into(),
                        score: re.get(7)?.as_str().parse::<i8>().unwrap(),
                        ty: match re.get(8)?.as_str() {
                            "H2Hn" => FlightType::NonExistingH2H,
                            "H2Nn" => FlightType::NonExistingH2N,
                            "N2Nn" => FlightType::NonExistingN2N,
                            "H2He" => FlightType::ExistingH2H,
                            "H2Ne" => FlightType::ExistingH2N,
                            "N2Ne" => FlightType::ExistingN2N,
                            _ => unreachable!(),
                        },
                    }
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow!("Invalid out file"))
    }
}
