use std::collections::HashMap;
use std::rc::Rc;
use chrono::NaiveDate;
use gtfs_structures::{DirectionType, Gtfs, Trip};
use crate::utils;

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

pub struct Network {
    pub routes: Vec<Route>,
    pub stops: Vec<Stop>,
    pub stop_index: HashMap<String, StopIndex>,
    pub stop_times: Vec<StopTime>,
    pub stop_routes: Vec<RouteIndex>,
    pub route_stops: Vec<StopIndex>,
    pub transfer_times: Vec<Timestamp>,
}

impl Network {
    pub fn new(gtfs: &Gtfs, journey_date: NaiveDate, default_transfer_time: Timestamp) -> Self {
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
            stop_index.insert(id.clone(), i as StopIndex);
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

        // Construct routes, which point to a series of stops and stop times.
        let mut routes = Vec::new();
        let mut route_stops = Vec::new();
        let mut stop_times = Vec::new();
        for route_trips in routes_map.values_mut() {
            let first_trip = match route_trips.get(0) {
                Some(&first_trip) => first_trip,
                None => continue,
            };

            // Sort trips in route based on earliest arrival time.
            route_trips.sort_unstable_by(|a, b| {
                a.stop_times[0]
                    .arrival_time
                    .cmp(&b.stop_times[0].arrival_time)
            });

            let first_route = &gtfs.routes[first_trip.route_id.as_str()];
            let line_name = first_route.short_name.as_ref().unwrap();
            routes.push(Route {
                line: Rc::from(line_name.as_str()),
                num_stops: first_trip.stop_times.len() as StopIndex,
                num_trips: route_trips.len() as TripIndex,
                route_stops_idx: route_stops.len(),
                stop_times_idx: stop_times.len(),
            });

            // Because of how routes are constructed, all trips in a route have the same stops.
            // So grab the stops from the first trip.
            for stop_time in first_trip.stop_times.iter() {
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

        let transfer_times = vec![default_transfer_time; stops.len()];

        Self {
            routes,
            stops,
            stop_index,
            stop_times,
            stop_routes,
            route_stops,
            transfer_times,
        }
    }

    pub fn set_transfer_time_for_stop(&mut self, stop_id: &str, transfer_time: Timestamp) {
        let stop_idx = self.get_stop_idx(stop_id) as usize;
        self.transfer_times[stop_idx] = transfer_time;
    }

    pub fn get_stop(&self, stop: StopIndex) -> &Stop { &self.stops[stop as usize] }

    pub fn get_stop_idx(&self, stop_id: &str) -> StopIndex { self.stop_index[stop_id] }

    pub fn num_stops(&self) -> usize { self.stops.len() }

    pub fn num_routes(&self) -> usize { self.routes.len() }

    pub fn num_trips(&self, route_idx: usize) -> usize { self.routes[route_idx].num_trips as usize }

    pub fn num_stops_in_route(&self, route_idx: usize) -> usize { self.routes[route_idx].num_stops as usize }

    pub fn get_trip(&self, route_idx: usize, trip_idx: usize) -> &[StopTime] {
        let route = &self.routes[route_idx];
        route.get_trip(trip_idx, &self.stop_times)
    }
}