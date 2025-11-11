use crate::{
    Config, FlightData,
    types::{AirportCode, flight_type::FlightType, gate::Gate},
};

pub trait FlightUtils {
    fn score(&self, config: &mut Config, flight_data: &FlightData) -> anyhow::Result<i8>;
    fn get_flight_type(
        &self,
        config: &mut Config,
        flight_data: &FlightData,
    ) -> anyhow::Result<FlightType>;
}

impl FlightUtils for (&AirportCode, &AirportCode) {
    fn score(&self, config: &mut Config, flight_data: &FlightData) -> anyhow::Result<i8> {
        let mut s = 0i8;

        s -= flight_data.num_flights(self.0, self.1) as i8 - 1;
        if s == 1 {
            s += 1;
        }

        s += self.get_flight_type(config, flight_data)?.score();

        if config
            .preferred_between
            .iter()
            .any(|fs| fs.contains(self.0) && fs.contains(self.1))
        {
            s += 20;
        }
        if let Some(dests) = config.preferred_to.get(self.0)
            && dests.contains(self.1)
        {
            s += 20;
        }
        if let Some(dests) = config.preferred_to.get(self.1)
            && dests.contains(self.0)
        {
            s += 20;
        }

        if flight_data.old_world_airports.contains(self.0)
            != flight_data.old_world_airports.contains(self.1)
        {
            s += 3;
        }

        Ok(s)
    }

    //noinspection DuplicatedCode
    fn get_flight_type(
        &self,
        config: &mut Config,
        flight_data: &FlightData,
    ) -> anyhow::Result<FlightType> {
        Ok(if config.hubs()?.contains(self.0) {
            if config.hubs()?.contains(self.1) {
                if flight_data.num_flights(self.0, self.1) > 0 {
                    FlightType::ExistingH2H
                } else {
                    FlightType::NonExistingH2H
                }
            } else if flight_data.num_flights(self.0, self.1) > 0 {
                FlightType::ExistingH2N
            } else {
                FlightType::NonExistingH2N
            }
        } else if config.hubs()?.contains(self.1) {
            if flight_data.num_flights(self.0, self.1) > 0 {
                FlightType::ExistingH2N
            } else {
                FlightType::NonExistingH2N
            }
        } else if flight_data.num_flights(self.0, self.1) > 0 {
            FlightType::ExistingN2N
        } else {
            FlightType::NonExistingN2N
        })
    }
}

impl FlightUtils for (&Gate, &Gate) {
    fn score(&self, config: &mut Config, flight_data: &FlightData) -> anyhow::Result<i8> {
        let mut s = (&self.0.airport, &self.1.airport).score(config, flight_data)?;
        if &*self.0.size != "S" {
            s += 2;
        }
        if &*self.0.size == "XS" {
            s += 1;
        }

        Ok(s)
    }
    fn get_flight_type(
        &self,
        config: &mut Config,
        flight_data: &FlightData,
    ) -> anyhow::Result<FlightType> {
        (&self.0.airport, &self.1.airport).get_flight_type(config, flight_data)
    }
}
