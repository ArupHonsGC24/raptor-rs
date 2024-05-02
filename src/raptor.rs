use std::fmt::Display;
use std::rc::Rc;

use crate::utils;
use crate::network::{Network, StopIndex, Timestamp};
use crate::utils::OptionExt;

const K: usize = 8;

// Timestamp is seconds since midnight.

#[derive(Clone)]
struct Boarding {
    pub boarded_stop: StopIndex,
    pub boarded_time: Timestamp,
    pub line: Rc<str>,
}

pub struct Leg {
    pub boarded_stop: StopIndex,
    pub boarded_stop_name: Box<str>,
    pub boarded_time: Timestamp,
    pub arrival_stop: StopIndex,
    pub arrival_stop_name: Box<str>,
    pub arrival_time: Timestamp,
    pub line: Rc<str>,
}

pub struct Journey {
    pub legs: Vec<Leg>,
}

impl Journey {
    pub fn new() -> Self {
        Self { legs: Vec::new() }
    }
    pub fn from(legs: Vec<Leg>) -> Self {
        Self { legs }
    }
}

impl Display for Journey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "-----------------------------------------------")?;
        if self.legs.len() > 0 {
            for leg in self.legs.iter() {
                writeln!(f)?;
                writeln!(f,
                         "Board at {} at {} ({} line).",
                         leg.boarded_stop_name,
                         // utils::get_short_stop_name(&self.network.get_stop(leg.boarded_stop).name),
                         utils::get_time_str(leg.boarded_time),
                         leg.line,
                )?;
                writeln!(f,
                         "Arrive at {} at {}.",
                         leg.arrival_stop_name,
                         // utils::get_short_stop_name(&self.network.get_stop(leg.arrival_stop).name),
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

pub fn raptor_query(network: &Network, start: StopIndex, start_time: Timestamp, end: StopIndex) -> Journey {
    let start = start as usize;
    let end = end as usize;
    let num_stops = network.stops.len();

    // τ[p][i] = earliest known arrival time at stop p with up to i trips.
    let mut tau = vec![[Timestamp::MAX; K]; num_stops];
    // τ*[p] = earliest known arrival time at stop p.
    let mut tau_star = vec![(Timestamp::MAX, None); num_stops];

    // Set initial departure time from start station.
    tau[start][0] = start_time;
    tau_star[start] = (start_time, None);

    // Array for recording which stops have been marked in the current round.
    let mut marked_stops = vec![false; num_stops];
    marked_stops[start] = true;

    // The equivalent of the set Q in the paper.
    let mut earliest_stop_for_route = vec![None; network.routes.len()];

    // RAPTOR
    for k in 1..K {
        earliest_stop_for_route.fill(Some(0));
        for marked_stop in
        marked_stops
            .iter()
            .enumerate()
            .filter_map(|(i, &touched)| if touched { Some(i) } else { None })
        {
            for &route_idx in network.stops[marked_stop].get_routes(&network.stop_routes) {
                let route_idx = route_idx as usize;
                let route = &network.routes[route_idx];
                let earliest_stop_in_route_order =
                    earliest_stop_for_route[route_idx].unwrap_or(route.num_stops as usize);

                for (stop_order, &route_stop) in
                route.get_stops(&network.route_stops).iter().enumerate()
                {
                    if stop_order >= earliest_stop_in_route_order {
                        break;
                    }
                    if route_stop == (marked_stop as StopIndex) {
                        // Update the earliest touched stop for route.
                        earliest_stop_for_route[route_idx] = Some(stop_order);
                        break;
                    }
                }
                // Should always have an earliest stop for route.
                assert!(earliest_stop_for_route[route_idx].is_some());
            }
        }
        marked_stops.fill(false);

        // Traverse each marked route.
        for (route_idx, earliest_stop_order) in earliest_stop_for_route
            .iter()
            .enumerate()
            .filter_map(|(i, stop)| stop.map(|s| (i, s)))
        {
            let route = &network.routes[route_idx];
            let mut trip_idx: Option<(usize, Boarding)> = None;
            for (stop_order, &stop_idx) in route
                .get_stops(&network.route_stops)
                .iter()
                .enumerate()
                .skip(earliest_stop_order)
            {
                let stop_idx = stop_idx as usize;
                let current_tau = tau[stop_idx][k - 1];
                // Ignore transfer time for first round.
                let current_transfer_time = if k > 1 {
                    network.transfer_times[stop_idx]
                } else {
                    0
                };

                // Can the arrival time at this stop be improved in this round?
                let mut current_departure_time = None;
                if let Some((trip_idx, boarded_stop_idx)) = &trip_idx {
                    let trip = route.get_trip(*trip_idx, &network.stop_times);
                    let arrival_time = trip[stop_order].arrival_time;
                    current_departure_time = Some(trip[stop_order].departure_time);
                    if arrival_time < tau_star[stop_idx].0.min(tau_star[end].0) {
                        tau[stop_idx][k] = arrival_time;
                        tau_star[stop_idx] = (arrival_time, Some(boarded_stop_idx.clone()));
                        marked_stops[stop_idx] = true;
                    }
                }

                // NOTE: Why is this after the code to update this stop?
                // Two cases where we update the current trip:
                // 1. This is the first stop in the trip. It was therefore set by the previous round.
                // 2. This is a subsequent stop in the trip, where another route has reached it faster. Similarly, it has already been updated to the fastest time.

                // Can we catch an earlier trip at this stop?
                if current_departure_time
                    .is_none_or(|dep| current_tau.saturating_add(current_transfer_time) <= dep)
                {
                    // Compute et(r, p)
                    let num_stops = route.num_stops as usize;
                    let current_trip_idx = match trip_idx {
                        Some((trip_idx, _)) => trip_idx,
                        None => route.num_trips as usize,
                    };

                    // Because the trip index can only ever decrease, we start from the last trip and work our way back.
                    // Thus, all trips are accessed at most once each round.
                    let found_trip_idx = (0..current_trip_idx)
                        .rev()
                        .map(|trip_idx| {
                            // We want to save the departure time of the trip we select.
                            let stop_time_idx = trip_idx * num_stops + stop_order;
                            (
                                trip_idx,
                                network.stop_times[route.stop_times_idx + stop_time_idx]
                                    .departure_time,
                            )
                        })
                        .take_while(|&(_, departure_time)| {
                            departure_time >= current_tau.saturating_add(current_transfer_time)
                        })
                        .last();

                    // If no new trip was found, we continue with the current trip.
                    // If a new trip was found, we update the trip and the stop we boarded it.
                    if let Some((found_trip_idx, departure_time)) = found_trip_idx {
                        trip_idx = Some((
                            found_trip_idx,
                            Boarding {
                                boarded_stop: stop_idx as StopIndex,
                                boarded_time: departure_time,
                                line: route.line.clone(),
                            },
                        ));
                    }
                }
            }
        }

        // TODO: Footpaths between stops

        if utils::is_zero(&marked_stops) {
            break;
        }
    }

    // Reconstruct trip from parent pointers
    let mut legs = Vec::new();
    let mut current_stop_opt = Some(end);
    while let Some(current_stop) = current_stop_opt {
        if current_stop == start {
            break;
        }
        let (arrival_time, boarded_leg) = &tau_star[current_stop];
        if let Some(boarded_leg) = boarded_leg {
            let boarded_stop_name = utils::get_short_stop_name(&network.get_stop(boarded_leg.boarded_stop as usize).name);
            let arrival_stop_name = utils::get_short_stop_name(&network.get_stop(current_stop).name);
            legs.push(Leg {
                boarded_stop: boarded_leg.boarded_stop,
                boarded_stop_name: boarded_stop_name.to_owned().into_boxed_str(),
                boarded_time: boarded_leg.boarded_time,
                arrival_stop: current_stop as StopIndex,
                arrival_stop_name: arrival_stop_name.to_owned().into_boxed_str(),
                arrival_time: *arrival_time,
                line: boarded_leg.line.clone(),
            });
        }
        current_stop_opt = boarded_leg.as_ref().map(|leg| leg.boarded_stop as usize);
    }
    legs.reverse();

    Journey::from(legs)
}
