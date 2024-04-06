mod utils;

use std::collections::HashMap;

use chrono::NaiveDate;
use gtfs_structures::{Gtfs, Trip};

const K: usize = 8;

type Timestamp = u32;
type StopIndex = u8;

const STOP_BITFIELD_LENGTH: usize = StopIndex::MAX as usize + 1 / 64;

type StopBitfield = bnum::BUint<STOP_BITFIELD_LENGTH>;
type RouteIndex = u32;
type TripIndex = u16;

fn get_short_stop_name(stop: &gtfs_structures::Stop) -> Option<&str> {
    // Convert "Laburnum Railway Station (Blackburn)" to "Laburnum", and "Noble Park Railway Station (Noble Park)" to "Noble Park", etc.
    stop.name.as_ref()?.split(" Railway Station").next()
}

#[allow(dead_code)]
fn print_stop_names(gtfs: &Gtfs, routes: &HashMap<StopBitfield, Vec<&Trip>>) {
    let mut route_stop_names = Vec::new();
    for route in routes.values() {
        for trip in route {
            let mut stop_names = Vec::new();
            for stop_time in trip.stop_times.iter() {
                stop_names.push(get_short_stop_name(stop_time.stop.as_ref()).unwrap());
            }

            route_stop_names.push((
                gtfs.routes[trip.route_id.as_str()]
                    .short_name
                    .as_ref()
                    .unwrap(),
                stop_names,
            ));

            break;
        }
    }

    route_stop_names.sort_unstable_by(|(trip_a, names_a), (trip_b, names_b)| {
        trip_a.cmp(trip_b).then(names_a.len().cmp(&names_b.len()))
    });

    for (name, route_stop_name) in route_stop_names {
        println!("{name}: {route_stop_name:?}");
        println!();
    }
}

struct Route {
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
    // pub fn get_stop_for_trip<'a>(
    //     &self,
    //     trip: usize,
    //     stop: usize,
    //     stop_times: &'a [StopTime],
    // ) -> &'a StopTime {
    //     &stop_times[self.stop_times_idx + trip * self.num_stops as usize + stop]
    // }
}

struct StopTime {
    pub arrival_time: Timestamp,
    pub departure_time: Timestamp,
}

struct Stop {
    pub name: String,
    pub routes_idx: usize,
    pub num_routes: usize,
}

impl Stop {
    pub fn new(name: String) -> Self {
        Self {
            name,
            routes_idx: 0,
            num_routes: 0,
        }
    }

    pub fn get_routes<'a>(&self, stop_routes: &'a [RouteIndex]) -> &'a [RouteIndex] {
        &stop_routes[self.routes_idx..(self.routes_idx + self.num_routes)]
    }
}

fn does_trip_run(gtfs: &Gtfs, trip: &Trip, date: NaiveDate) -> bool {
    let calender = &gtfs.calendar[trip.service_id.as_str()];
    calender.valid_weekday(date) && calender.start_date <= date && date <= calender.end_date
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gtfs = Gtfs::new("../gtfs/2/google_transit.zip")?;

    // GTFS optional fields that are unwrapped: stop.name, trip.direction_id, stop_time.arrival_time, stop_time.departure_time.
    println!(
        "GTFS loaded with {} stops, {} routes, and {} trips.",
        gtfs.stops.len(),
        gtfs.routes.len(),
        gtfs.trips.len()
    );
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
    let num_stops = stops.len();

    // This is our input data. For now, the date is used to filter trips.
    let start = stop_index["19866"] as usize;
    let start_time = utils::parse_time("8:30:00").unwrap();
    let journey_date = NaiveDate::from_ymd_opt(2024, 4, 16).unwrap();
    let end = stop_index["19985"] as usize;

    // Construct our own routes as collections of trips, because the ones defined in the GTFS contain different amounts of stops.
    let mut routes_map = HashMap::new();
    for (_, trip) in gtfs.trips.iter() {
        // Only consider trips that run on the given date.
        if !does_trip_run(&gtfs, &trip, journey_date) {
            continue;
        }

        // Construct a 256 bit integer where the most significant bit is the direction of the trip, and the rest are stops.
        let mut stop_field =
            StopBitfield::from_digit(trip.direction_id.unwrap() as u64) << StopIndex::MAX;
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
    // print_stop_names(&gtfs, &routes);

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

    // No more hashmap accesses?

    println!("Start: {} at time {}", stops[start].name, utils::get_time_str(start_time));
    println!("End: {}", stops[end].name);

    // τ[p][i] = earliest known arrival time at stop p with up to i trips.
    let mut tau = vec![[Timestamp::MAX; K]; num_stops];
    // τ*[p] = earliest known arrival time at stop p.
    let mut tau_star = vec![(Timestamp::MAX, usize::MAX); num_stops];

    // Set initial departure time from start station.
    tau[start][0] = start_time;

    // Array for recording which stops have been marked in the current round.
    let mut marked_stops = vec![false; num_stops];
    marked_stops[start] = true;

    // The equivalent of the set Q in the paper.
    let mut earliest_stop_for_route = vec![None; routes.len()];

    // RAPTOR
    for k in 1..K {
        earliest_stop_for_route.fill(None);
        for marked_stop in
            marked_stops
                .iter()
                .enumerate()
                .filter_map(|(i, touched)| if *touched { Some(i) } else { None })
        {
            for route_idx in stops[marked_stop]
                .get_routes(&stop_routes)
                .iter()
                .map(|&r| r as usize)
            {
                let route = &routes[route_idx];
                let earliest_stop_in_route_order =
                    earliest_stop_for_route[route_idx].unwrap_or(route.num_stops as usize);

                for (stop_order, &route_stop) in route.get_stops(&route_stops).iter().enumerate() {
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
            let route = &routes[route_idx];
            let mut trip_idx = None;
            for (stop_order, &stop_idx) in route
                .get_stops(&route_stops)
                .iter()
                .enumerate()
                .skip(earliest_stop_order)
            {
                let stop_idx = stop_idx as usize;

                // Can the arrival time at this stop be improved in this round?
                let mut current_departure_time = None;
                if let Some((trip_idx, boarded_stop_order)) = trip_idx {
                    let trip = route.get_trip(trip_idx, &stop_times);
                    let arrival_time = trip[stop_order].arrival_time;
                    current_departure_time = Some(trip[stop_order].departure_time);
                    if arrival_time < tau_star[stop_idx].0.min(tau_star[end].0) {
                        tau[stop_idx][k] = arrival_time;
                        tau_star[stop_idx] = (arrival_time, boarded_stop_order);
                        marked_stops[stop_idx] = true;
                    }
                }

                // NOTE: Why is this after the code to update this stop?
                // Two cases where we update the current trip:
                // 1. This is the first stop in the trip. It was therefore set by the previous round.
                // 2. This is a subsequent stop in the trip, where another route has reached it faster. Similarly, it has already been updated to the fastest time.

                // Can we arrive by an earlier trip at the current stop?
                if !current_departure_time.is_some_and(|dep| tau[stop_idx][k - 1] > dep) {
                    // Compute et(r, p)
                    let num_stops = route.num_stops as usize;
                    let current_trip_idx = match trip_idx {
                        Some((trip_idx, _)) => trip_idx,
                        None => route.num_trips as usize - 1
                    };

                    let found_stop_time_idx = (0..(current_trip_idx * num_stops + stop_order))
                        .step_by(num_stops)
                        .rfind(|&stop_time_idx| {
                            stop_times[stop_time_idx].departure_time > tau[stop_idx][k - 1]
                        });

                    // If no new trip was found, we continue with the current trip. If a new trip was found, we update the trip and the stop we boarded it.
                    if let Some(found_trip_idx) = found_stop_time_idx.map(|idx| idx / num_stops) {
                        trip_idx = Some((found_trip_idx, stop_order));
                    }
                }
            }
        }

        // TODO: Footpaths between stops

        // TODO: Check if this is slow and if it can be optimised.
        // (https://stackoverflow.com/questions/65367552/how-to-efficiently-check-a-vecu8-to-see-if-its-all-zeros)
        if !marked_stops.iter().any(|&b| b) {
            break;
        }
    }

    println!("Earliest arrival time at end stop: {}, boarded from {}", utils::get_time_str(tau_star[end].0), stops[tau_star[end].1].name);

    Ok(())
}
