use crate::{
    Config, FlightData,
    types::{AirportCode, flight_type::FlightType, gate::Gate},
};

impl Config {
    pub fn airports_score(
        &mut self,
        flight_data: &FlightData,
        a1: &AirportCode,
        a2: &AirportCode,
    ) -> anyhow::Result<i8> {
        let mut s = 0i8;

        s -= flight_data.num_flights(a1, a2) as i8 - 1;
        if s == 1 {
            s += 1;
        }

        s += self.airports_flight_type(flight_data, a1, a2)?.score();

        if self
            .preferred_between
            .iter()
            .any(|fs| fs.contains(a1) && fs.contains(a2))
        {
            s += 20;
        }
        if let Some(dests) = self.preferred_to.get(a1)
            && dests.contains(a2)
        {
            s += 20;
        }
        if let Some(dests) = self.preferred_to.get(a2)
            && dests.contains(a1)
        {
            s += 20;
        }

        if flight_data.old_world_airports.contains(a1)
            != flight_data.old_world_airports.contains(a2)
        {
            s += 3;
        }

        Ok(s)
    }
    pub fn gates_score(
        &mut self,
        flight_data: &FlightData,
        g1: &Gate,
        g2: &Gate,
    ) -> anyhow::Result<i8> {
        let mut s = self.airports_score(flight_data, &g1.airport, &g2.airport)?;
        if &*g1.size != "S" {
            s += 2;
        }
        if &*g2.size == "XS" {
            s += 1;
        }

        Ok(s)
    }
}

impl FlightType {
    pub const fn score(self) -> i8 {
        match self {
            Self::NonExistingH2H => 6,
            Self::ExistingH2H => 5,
            Self::NonExistingH2N => 3,
            Self::NonExistingN2N => 2,
            Self::ExistingH2N => 1,
            Self::ExistingN2N => -1,
        }
    }
}
