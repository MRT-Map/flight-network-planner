use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow};
use counter::Counter;
use itertools::Itertools;
use patharg::InputArg;
use serde::{Deserialize, Serialize};

use crate::types::{
    AirlineName, AirportCode, FlightNumber, GateCode,
    flight_data::FlightData,
    flight_type::FlightType,
    gate::{Gate, PartialGate},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub airline_name: AirlineName,
    ignored_airlines: Vec<AirlineName>,
    hubs: Vec<AirportCode>,
    hub_threshold: usize,
    pub range_h2h: Vec<(FlightNumber, FlightNumber)>,
    pub range_n2n: Vec<(FlightNumber, FlightNumber)>,
    pub range_h2n: HashMap<AirportCode, Vec<(FlightNumber, FlightNumber)>>,
    pub both_dir_same_num: bool,
    pub gate_file: Option<PathBuf>,
    #[serde(rename = "gates")]
    _gates: HashMap<AirportCode, Vec<PartialGate>>,
    pub hard_max_hub: u8,
    pub hard_max_nonhub: u8,
    pub max_h2h: u8,
    pub max_h2n_hub: u8,
    pub max_h2n_nonhub: u8,
    pub max_n2n: u8,
    pub restricted_between: Vec<Vec<AirportCode>>,
    pub restricted_to: HashMap<AirportCode, Vec<AirportCode>>,
    pub no_dupes: Vec<AirportCode>,
    pub preferred_between: Vec<Vec<AirportCode>>,
    pub preferred_to: HashMap<AirportCode, Vec<AirportCode>>,
    pub gate_allowed_dests: HashMap<AirportCode, HashMap<GateCode, Vec<AirportCode>>>,
    pub gate_denied_dests: HashMap<AirportCode, HashMap<GateCode, Vec<AirportCode>>>,
    pub max_dests_per_gate: HashMap<AirportCode, u8>,
    #[serde(skip)]
    pub gates: Vec<Gate>,
}
impl Config {
    pub fn load(file: &InputArg) -> Result<Self> {
        let mut config: Self = serde_yaml::from_slice(&file.read()?)?;
        config.gates = if let Some(gate_file) = &config.gate_file {
            let folder = file
                .path_ref()
                .cloned()
                .or_else(|| std::env::current_exe().ok())
                .and_then(|a| a.parent().map(ToOwned::to_owned));
            let gate_file = folder
                .as_ref()
                .map_or_else(|| gate_file.to_owned(), |folder| folder.join(gate_file));
            std::fs::read_to_string(gate_file)?
                .split('\n')
                .filter(|l| !l.trim().is_empty())
                .map(|l| {
                    Some({
                        let params = l.split(' ').collect::<Vec<_>>();
                        Gate {
                            airport: params.first()?.trim().into(),
                            code: params.get(1)?.trim().into(),
                            size: params.get(2)?.trim().into(),
                        }
                    })
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| anyhow!("Invalid gate file"))?
        } else {
            config
                ._gates
                .iter()
                .flat_map(|(a, pgs)| {
                    pgs.iter().map(|pg| Gate {
                        airport: a.to_owned(),
                        code: pg.code.clone(),
                        size: pg.size.clone(),
                    })
                })
                .collect()
        };
        Ok(config)
    }
    pub fn airports(&self) -> impl Iterator<Item = &AirportCode> {
        self.gates.iter().map(|g| &g.airport).unique()
    }
    pub fn hubs(&self) -> Box<dyn Iterator<Item = &AirportCode> + '_> {
        if self.hubs.is_empty() {
            Box::new(
                self.gates
                    .iter()
                    .map(|g| &g.airport)
                    .collect::<Counter<_>>()
                    .into_iter()
                    .filter(|(_, c)| *c >= self.hub_threshold)
                    .map(|(a, _)| a),
            )
        } else {
            Box::new(self.hubs.iter())
        }
    }
    pub fn ignored_airlines(&self) -> Box<dyn Iterator<Item = &AirlineName> + '_> {
        if self.ignored_airlines.is_empty() {
            Box::new(std::iter::once(&self.airline_name))
        } else {
            Box::new(self.ignored_airlines.iter())
        }
    }

    pub fn is_valid_flight(&self, fd: &FlightData, g1: &Gate, g2: &Gate) -> bool {
        if g1.airport == g2.airport || g1.size != g2.size {
            return false;
        }

        if self
            .restricted_between
            .iter()
            .any(|re| re.contains(&g1.airport) && re.contains(&g2.airport))
        {
            return false;
        }

        if self
            .restricted_to
            .get(&*g1.airport)
            .is_some_and(|a| a.contains(&g2.airport))
            || self
                .restricted_to
                .get(&*g2.airport)
                .is_some_and(|a| a.contains(&g1.airport))
        {
            return false;
        }

        if self
            .gate_allowed_dests
            .get(&*g1.airport)
            .is_some_and(|gates| {
                gates
                    .get(&*g1.code)
                    .is_some_and(|gate| !gate.contains(&g2.airport))
            })
            || self
                .gate_allowed_dests
                .get(&*g2.airport)
                .is_some_and(|gates| {
                    gates
                        .get(&*g2.code)
                        .is_some_and(|gate| !gate.contains(&g1.airport))
                })
        {
            return false;
        }

        if self
            .gate_denied_dests
            .get(&*g1.airport)
            .is_some_and(|gates| {
                gates
                    .get(&*g1.code)
                    .is_some_and(|gate| gate.contains(&g2.airport))
            })
            || self
                .gate_denied_dests
                .get(&*g2.airport)
                .is_some_and(|gates| {
                    gates
                        .get(&*g2.code)
                        .is_some_and(|gate| gate.contains(&g1.airport))
                })
        {
            return false;
        }

        if self
            .preferred_between
            .iter()
            .any(|a| a.contains(&g1.airport) && a.contains(&g2.airport))
        {
            return true;
        }

        if self
            .preferred_to
            .get(&g1.airport)
            .is_some_and(|a| a.contains(&g2.airport))
            || self
                .preferred_to
                .get(&g2.airport)
                .is_some_and(|a| a.contains(&g1.airport))
        {
            return true;
        }

        if self.no_dupes.contains(&g1.airport) || self.no_dupes.contains(&g2.airport) {
            return ![
                FlightType::ExistingH2H,
                FlightType::ExistingH2N,
                FlightType::ExistingN2N,
            ]
            .contains(&self.gates_flight_type(fd, g1, g2));
        }

        true
    }
}
