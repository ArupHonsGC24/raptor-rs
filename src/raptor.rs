use std::collections::HashMap;
use std::rc::Rc;

use crate::utils;
use chrono::NaiveDate;
use gtfs_structures::{DirectionType, Gtfs, Trip};

use crate::utils::OptionExt;

const K: usize = 8;

// Timestamp is seconds since midnight.
pub type Timestamp = u32;
pub type StopIndex = u8;

const STOP_BITFIELD_LENGTH: usize = (StopIndex::MAX as usize + 1) / 64;

pub type StopBitfield = bnum::BUint<STOP_BITFIELD_LENGTH>;
pub type RouteIndex = u32;
pub type TripIndex = u32;

pub struct Route {
    pub line: Rc<str>,
    pub num_stops: StopIndex,
    pub num_trips: TripIndex,
    pub route_stops_idx: usize,
    pub stop_times_idx: usize,
}

impl Route {
    pub fn get_stops<'a>(&self, route_stops: &'a [StopIndex]) -> &'a [StopIndex] {
        &route_stops[self.route_stops_idx..(self.route_stops_idx + self.num_stops as usize)]
    }
    pub fn get_trip<'a>(&self, trip: usize, stop_times: &'a [StopTime]) -> &'a [StopTime] {
        let start = self.stop_times_idx + trip * self.num_stops as usize;
        let end = start + self.num_stops as usize;
        &stop_times[start..end]
    }
}

pub struct StopTime {
    pub arrival_time: Timestamp,
    pub departure_time: Timestamp,
}

pub struct Stop {
    pub name: Box<str>,
    pub routes_idx: usize,
    pub num_routes: usize,
}

impl Stop {
    pub fn new(name: String) -> Self {
        Self {
            name: name.into_boxed_str(),
            routes_idx: 0,
            num_routes: 0,
        }
    }

    pub fn get_routes<'a>(&self, stop_routes: &'a [RouteIndex]) -> &'a [RouteIndex] {
        &stop_routes[self.routes_idx..(self.routes_idx + self.num_routes)]
    }
}

#[derive(Clone)]
struct Boarding {
    pub boarded_stop: StopIndex,
    pub boarded_time: Timestamp,
    pub line: Rc<str>,
}

pub struct Leg {
    pub boarded_stop: StopIndex,
    pub boarded_time: Timestamp,
    pub arrival_stop: StopIndex,
    pub arrival_time: Timestamp,
    pub line: Rc<str>,
}

pub struct Raptor<'a> {
    routes: Vec<Route>,
    stops: Vec<Stop>,
    stop_index: HashMap<&'a str, StopIndex>,
    stop_times: Vec<StopTime>,
    stop_routes: Vec<RouteIndex>,
    route_stops: Vec<StopIndex>,
    pub transfer_time: Vec<Timestamp>,
}

impl<'a> Raptor<'a> {
    pub fn new(gtfs: &'a Gtfs, journey_date: NaiveDate, default_transfer_time: Timestamp) -> Self {
        // We use one stop index as the direction of the trip when grouping as routes.
        assert!(
            gtfs.stops.len() < (StopIndex::MAX - 1) as usize,
            "Too many stops ({}) in GTFS (we currently use a {}-bitfield for stops).",
            gtfs.stops.len(),
            StopIndex::MAX
        );

        let mut stop_index = HashMap::with_capacity(gtfs.stops.capacity());
        let mut stops = Vec::with_capacity(gtfs.stops.len());
        for (i, (id, value)) in gtfs.stops.iter().enumerate() {
            stop_index.insert(id.as_str(), i as StopIndex);
            stops.push(Stop::new(value.name.as_ref().unwrap().to_string()));
        }

        // Construct our own routes as collections of trips, because the ones defined in the GTFS contain different amounts of stops.
        let mut routes_map = HashMap::new();
        for (_, trip) in gtfs.trips.iter() {
            // Only consider trips that run on the given date.
            if !utils::does_trip_run(&gtfs, &trip, journey_date) {
                continue;
            }

            // TODO: Group trips by route first so we can use a smaller integer for the bitfield, and handle more stops across the network.
            // Construct a 256 bit integer where the most significant bit is the direction of the trip, and the rest are stops.
            let mut stop_field = StopBitfield::from_digit(
                trip.direction_id.unwrap_or(DirectionType::Outbound) as u64,
            ) << StopIndex::MAX;
            for stop_time in trip.stop_times.iter() {
                let stop_idx = stop_index[stop_time.stop.id.as_str()];
                stop_field |= StopBitfield::ONE << stop_idx;
            }
            let route: &mut Vec<&Trip> = routes_map.entry(stop_field).or_default();
            route.push(trip);
        }

        assert!(
            routes_map.len() < RouteIndex::MAX as usize,
            "Too many routes in GTFS (we currently use a {}-bit index for routes).",
            std::mem::size_of::<RouteIndex>() * 8
        );
        assert!(
            gtfs.trips.len() < TripIndex::MAX as usize,
            "Too many trips in GTFS (we currently use a {}-bit index for trips).",
            std::mem::size_of::<TripIndex>() * 8
        );

        let mut routes = Vec::new();
        let mut route_stops = Vec::new();
        let mut stop_times = Vec::new();
        for route_trips in routes_map.values_mut() {
            // Sort trips in route based on earliest arrival time.
            route_trips.sort_unstable_by(|a, b| {
                a.stop_times[0]
                    .arrival_time
                    .cmp(&b.stop_times[0].arrival_time)
            });

            routes.push(Route {
                line: Rc::from(
                    gtfs.routes[route_trips[0].route_id.as_str()]
                        .short_name
                        .as_ref()
                        .unwrap()
                        .to_string(),
                ),
                num_stops: route_trips[0].stop_times.len() as StopIndex,
                num_trips: route_trips.len() as TripIndex,
                route_stops_idx: route_stops.len(),
                stop_times_idx: stop_times.len(),
            });

            for stop_time in route_trips[0].stop_times.iter() {
                route_stops.push(stop_index[stop_time.stop.id.as_str()]);
            }

            for trip in route_trips {
                for stop_time in trip.stop_times.iter() {
                    stop_times.push(StopTime {
                        arrival_time: stop_time.arrival_time.unwrap(),
                        departure_time: stop_time.departure_time.unwrap(),
                    });
                }
            }
        }

        // Index the routes for a given stop.
        let mut stop_routes = Vec::new();
        for (stop_idx, stop) in stops.iter_mut().enumerate() {
            stop.routes_idx = stop_routes.len();

            for (route_idx, route) in routes.iter().enumerate() {
                if route
                    .get_stops(&route_stops)
                    .contains(&(stop_idx as StopIndex))
                {
                    stop_routes.push(route_idx as RouteIndex);
                }
            }
            stop.num_routes = stop_routes.len() - stop.routes_idx;
        }

        let transfer_time = vec![default_transfer_time; stops.len()];

        Self {
            routes,
            stops,
            stop_index,
            stop_times,
            stop_routes,
            route_stops,
            transfer_time,
        }
    }

    pub fn get_stop(&self, stop: StopIndex) -> &Stop {
        &self.stops[stop as usize]
    }

    pub fn get_stop_idx(&self, stop_id: &str) -> StopIndex {
        self.stop_index[stop_id]
    }

    pub fn query(&self, start: StopIndex, start_time: Timestamp, end: StopIndex) -> Vec<Leg> {
        let start = start as usize;
        let end = end as usize;
        // τ[p][i] = earliest known arrival time at stop p with up to i trips.
        let mut tau = vec![[Timestamp::MAX; K]; self.stops.len()];
        // τ*[p] = earliest known arrival time at stop p.
        let mut tau_star = vec![(Timestamp::MAX, None); self.stops.len()];

        // Set initial departure time from start station.
        tau[start][0] = start_time;
        tau_star[start] = (start_time, None);

        // Array for recording which stops have been marked in the current round.
        let mut marked_stops = vec![false; self.stops.len()];
        marked_stops[start] = true;

        // The equivalent of the set Q in the paper.
        let mut earliest_stop_for_route = vec![None; self.routes.len()];

        // RAPTOR
        for k in 1..K {
            earliest_stop_for_route.fill(Some(0));
            for marked_stop in
                marked_stops
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &touched)| if touched { Some(i) } else { None })
            {
                for &route_idx in self.stops[marked_stop].get_routes(&self.stop_routes) {
                    let route_idx = route_idx as usize;
                    let route = &self.routes[route_idx];
                    let earliest_stop_in_route_order =
                        earliest_stop_for_route[route_idx].unwrap_or(route.num_stops as usize);

                    for (stop_order, &route_stop) in
                        route.get_stops(&self.route_stops).iter().enumerate()
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
                let route = &self.routes[route_idx];
                let mut trip_idx: Option<(usize, Boarding)> = None;
                for (stop_order, &stop_idx) in route
                    .get_stops(&self.route_stops)
                    .iter()
                    .enumerate()
                    .skip(earliest_stop_order)
                {
                    let stop_idx = stop_idx as usize;
                    let current_tau = tau[stop_idx][k - 1];
                    // Ignore transfer time for first round.
                    let current_transfer_time = if k > 1 {
                        self.transfer_time[stop_idx]
                    } else {
                        0
                    };

                    // Can the arrival time at this stop be improved in this round?
                    let mut current_departure_time = None;
                    if let Some((trip_idx, boarded_stop_idx)) = &trip_idx {
                        let trip = route.get_trip(*trip_idx, &self.stop_times);
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

                        // Because the trip index can only ever decrease, we start from the last trip and work our way back. Thus, all trips are accessed at most once each round.
                        let found_trip_idx = (0..current_trip_idx)
                            .rev()
                            .map(|trip_idx| {
                                // We want to save the departure time of the trip we select.
                                let stop_time_idx = trip_idx * num_stops + stop_order;
                                (
                                    trip_idx,
                                    self.stop_times[route.stop_times_idx + stop_time_idx]
                                        .departure_time,
                                )
                            })
                            .take_while(|&(_, departure_time)| {
                                departure_time >= current_tau.saturating_add(current_transfer_time)
                            })
                            .last();

                        // If no new trip was found, we continue with the current trip. If a new trip was found, we update the trip and the stop we boarded it.
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

        if false {
            println!();
            let mut sorted_stops = (0..self.stops.len()).collect::<Vec<_>>();
            sorted_stops.sort_unstable_by_key(|&stop| &self.stops[stop].name);
            for stop in sorted_stops {
                println!(
                    "Earliest arrival time at {}: {}",
                    utils::get_short_stop_name(&self.stops[stop].name),
                    utils::get_time_str(tau_star[stop].0)
                );
            }
            println!();
        }

        // Reconstruct trip from parent pointers
        let mut journey = Vec::new();
        let mut current_stop_opt = Some(end);
        while let Some(current_stop) = current_stop_opt {
            if current_stop == start {
                break;
            }
            let (arrival_time, boarded_leg) = &tau_star[current_stop];
            if let Some(boarded_leg) = boarded_leg {
                journey.push(Leg {
                    boarded_stop: boarded_leg.boarded_stop,
                    boarded_time: boarded_leg.boarded_time,
                    arrival_stop: current_stop as StopIndex,
                    arrival_time: *arrival_time,
                    line: boarded_leg.line.clone(),
                });
            }
            current_stop_opt = boarded_leg.as_ref().map(|leg| leg.boarded_stop as usize);
        }
        journey.reverse();

        journey
    }

    pub fn set_transfer_time_for_stop(&mut self, stop_id: &str, transfer_time: Timestamp) {
        self.transfer_time[self.stop_index[stop_id] as usize] = transfer_time;
    }
}
