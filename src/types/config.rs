use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow};
use counter::Counter;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::types::{
    AirlineName, AirportCode, FlightNumber, GateCode,
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
    pub gates: HashMap<AirportCode, Vec<PartialGate>>,
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
    _gates: Vec<Gate>,
    #[serde(skip)]
    pub _folder: Option<PathBuf>,
}
impl Config {
    pub fn airports(&mut self) -> Result<Vec<AirportCode>> {
        Ok(self
            .gates()?
            .into_iter()
            .map(|g| g.airport)
            .sorted()
            .dedup()
            .collect())
    }
    pub fn hubs(&mut self) -> Result<Vec<AirportCode>> {
        Ok(if self.hubs.is_empty() {
            self.gates()?
                .into_iter()
                .map(|g| g.airport)
                .collect::<Counter<_>>()
                .into_iter()
                .filter(|(_, c)| *c >= self.hub_threshold)
                .map(|(a, _)| a)
                .collect::<Vec<_>>()
        } else {
            self.hubs.clone()
        })
    }
    pub fn gates(&mut self) -> Result<Vec<Gate>> {
        if self._gates.is_empty() {
            let gates = if let Some(gate_file) = &self.gate_file {
                let gate_file = self
                    ._folder
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
                self.gates
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

            self._gates = gates;
        }
        Ok(self._gates.clone())
    }
    pub fn ignored_airlines(&self) -> Vec<AirlineName> {
        if self.ignored_airlines.is_empty() {
            vec![self.airline_name.clone()]
        } else {
            self.ignored_airlines.clone()
        }
    }
}
