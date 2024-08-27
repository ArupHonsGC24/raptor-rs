use crate::network::{PathfindingCost, RouteIndex, StopIndex, Timestamp, TripIndex};
use crate::{utils, Network};
use std::fmt::Display;
use crate::multicriteria::{Bag, Label};

pub struct Connection {
    pub unique_trip_idx: TripIndex, // Unique across the network.
    pub trip_order: TripIndex, // Index of the trip in the route.
    pub route_idx: RouteIndex,
    pub departure_idx: StopIndex,
    pub departure_stop_order: StopIndex,
    pub departure_time: Timestamp,
    pub arrival_idx: StopIndex,
    pub arrival_time: Timestamp,
}

#[derive(Clone)]
pub(crate) struct Boarding {
    pub boarded_stop: StopIndex,
    pub boarded_stop_order: StopIndex,
    pub boarded_time: Timestamp,
    pub route_idx: RouteIndex,
    pub trip_order: TripIndex,
}

impl Boarding {
    pub fn from(connection: &Connection) -> Self {
        Self {
            boarded_stop: connection.departure_idx,
            boarded_stop_order: connection.departure_stop_order,
            boarded_time: connection.departure_time,
            route_idx: connection.route_idx,
            trip_order: connection.trip_order,
        }
    }
}

#[derive(Clone)]
pub(crate) struct TauEntry {
    pub time: Timestamp,
    pub boarding: Option<Boarding>,
}

impl Default for TauEntry {
    fn default() -> Self {
        Self {
            time: Timestamp::MAX,
            boarding: None,
        }
    }
}

pub struct Leg {
    pub boarded_stop: StopIndex,
    pub boarded_stop_order: StopIndex,
    pub boarded_time: Timestamp,
    pub arrival_stop: StopIndex,
    pub arrival_stop_order: StopIndex,
    pub arrival_time: Timestamp,
    pub route_idx: RouteIndex,
    pub trip_order: TripIndex,
}

// Journey preferences for a multi-criteria journey query.
pub struct JourneyPreferences {
    // Will choose the journey with the shortest travel time.
    pub utility_function: fn(&Label) -> PathfindingCost,
}

impl Default for JourneyPreferences {
    fn default() -> Self {
        // By default, ignore cost and only consider travel time.
        JourneyPreferences { utility_function: |label| label.arrival_time as PathfindingCost }
    }
}

impl JourneyPreferences {
    pub(crate) fn best_label<'a>(&self, labels: &'a [Label]) -> Option<&'a Label> {
        labels.iter().min_by(|a,b| f32::total_cmp(&(self.utility_function)(a), &(self.utility_function)(b)))
    }
}

pub struct Journey<'a> {
    pub legs: Vec<Leg>,
    pub network: &'a Network,
}

impl<'a> Journey<'a> {
    pub fn empty(network: &'a Network) -> Self {
        Self { legs: Vec::new(), network }
    }

    fn from(legs: Vec<Leg>, network: &'a Network) -> Self {
        Self { legs, network }
    }

    pub(crate) fn from_tau(tau: &[TauEntry], network: &'a Network, start: usize, end: usize) -> Self {
        // No journey found.
        if tau[end].boarding.is_none() {
            return Journey::from(Vec::new(), network);
        }

        // Reconstruct trip from parent pointers
        let mut legs = Vec::new();
        let mut current_stop_opt = Some(end);
        const MAX_LEGS: usize = 100; // Prevent infinite loop (TODO: which is a bug).
        let mut num_legs = 0;
        while let Some(current_stop) = current_stop_opt {
            if current_stop == start {
                break;
            }
            num_legs += 1;
            if num_legs > MAX_LEGS {
                eprintln!("Infinite loop in journey reconstruction.");
                return Journey::empty(network);
            }
            let current_tau = &tau[current_stop];

            if let Some(boarded_leg) = &current_tau.boarding {
                // Find arrival stop order.
                let route = &network.routes[boarded_leg.route_idx as usize];
                let arrival_stop_order = route.get_stops(&network.route_stops).iter().enumerate().skip(boarded_leg.boarded_stop_order as usize).find_map(|(i, &stop)| {
                    if stop as usize == current_stop {
                        Some(i as StopIndex)
                    } else {
                        None
                    }
                }).expect("Arrival stop not found in route.");

                legs.push(Leg {
                    boarded_stop: boarded_leg.boarded_stop,
                    boarded_stop_order: boarded_leg.boarded_stop_order,
                    boarded_time: boarded_leg.boarded_time,
                    arrival_stop: current_stop as StopIndex,
                    arrival_stop_order,
                    arrival_time: current_tau.time,
                    route_idx: boarded_leg.route_idx,
                    trip_order: boarded_leg.trip_order,
                });
            }
            current_stop_opt = current_tau.boarding.as_ref().map(|leg| leg.boarded_stop as usize);
        }

        legs.reverse();

        Journey::from(legs, network)
    }

    pub(crate) fn from_tau_bag(tau: &[Bag], network: &'a Network, start: usize, end: usize, path_preferences: &JourneyPreferences) -> Self {
        let mut legs = Vec::new();
        let mut current_stop_opt = Some(end);
        while let Some(current_stop) = current_stop_opt {
            if current_stop == start {
                break;
            }
            if let Some(current_tau) = path_preferences.best_label(&tau[current_stop].labels) {
                if let Some(boarded_leg) = &current_tau.boarding {
                    // Find arrival stop order.
                    let route = &network.routes[boarded_leg.route_idx as usize];
                    let arrival_stop_order = route.get_stops(&network.route_stops).iter().enumerate().skip(boarded_leg.boarded_stop_order as usize).find_map(|(i, &stop)| {
                        if stop as usize == current_stop {
                            Some(i as StopIndex)
                        } else {
                            None
                        }
                    }).expect("Arrival stop not found in route.");

                    legs.push(Leg {
                        boarded_stop: boarded_leg.boarded_stop,
                        boarded_stop_order: boarded_leg.boarded_stop_order,
                        boarded_time: boarded_leg.boarded_time,
                        arrival_stop: current_stop as StopIndex,
                        arrival_stop_order,
                        arrival_time: current_tau.arrival_time,
                        route_idx: boarded_leg.route_idx,
                        trip_order: boarded_leg.trip_order,
                    });
                }
                current_stop_opt = current_tau.boarding.as_ref().map(|leg| leg.boarded_stop as usize);
            }
        }

        legs.reverse();
        Journey::from(legs, network)
    }
}

impl Display for Journey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "-----------------------------------------------")?;
        if self.legs.len() > 0 {
            for leg in self.legs.iter() {
                writeln!(f)?;
                writeln!(f,
                         "Board at {} at {} ({} line).",
                         //leg.boarded_stop_name,
                         utils::get_short_stop_name(&self.network.get_stop(leg.boarded_stop as usize).name),
                         utils::get_time_str(leg.boarded_time),
                         self.network.routes[leg.route_idx as usize].line,
                )?;
                writeln!(f,
                         "Arrive at {} at {}.",
                         //leg.arrival_stop_name,
                         &self.network.get_stop(leg.arrival_stop as usize).name,
                         utils::get_time_str(leg.arrival_time)
                )?;
            }
            writeln!(f, )?;
            writeln!(f, "Total journey time: {} minutes.", (self.legs.last().unwrap().arrival_time - self.legs[0].boarded_time) / 60)?;
        } else {
            writeln!(f)?;
            writeln!(f, "No journey found.")?;
        }
        writeln!(f, "-----------------------------------------------")?;
        Ok(())
    }
}

