mod utils;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use gtfs_structures::{Gtfs, Trip};
use bnum::types::U256;

const K: usize = 8;

// Convert GTFS maps into structures useful for querying and iteration.
fn linearise<'a, T>(map: &'a HashMap<String, T>) -> (HashMap<&'a str, usize>, Vec<&T>) {
    let mut index_map = HashMap::with_capacity(map.capacity());
    let mut linear = Vec::with_capacity(map.len());
    for (i, (id, value)) in map.iter().enumerate() {
        index_map.insert(id.as_str(), i);
        linear.push(value);
    }
    (index_map, linear)
}

fn get_short_stop_name(stop: &gtfs_structures::Stop) -> Option<&str> {
    // Convert "Laburnum Railway Station (Blackburn)" to "Laburnum", and "Noble Park Railway Station (Noble Park)" to "Noble Park", etc.
    stop.name.as_ref()?.split(" Railway Station").next()
}

#[allow(dead_code)]
fn print_stop_names(gtfs: &Gtfs, routes: &HashMap<U256, Vec<&Trip>>) {
    let mut route_stop_names = Vec::new();
    for route in routes.values() {
        for trip in route {
            let mut stop_names = Vec::new();
            for stop_time in trip.stop_times.iter() {
                stop_names.push(get_short_stop_name(stop_time.stop.as_ref()).unwrap());
            }

            route_stop_names.push((gtfs.routes[trip.route_id.as_str()].short_name.as_ref().unwrap(), stop_names));

            break;
        }
    }

    route_stop_names.sort_unstable_by(|(trip_a, names_a), (trip_b, names_b)| trip_a.cmp(trip_b).then(names_a.len().cmp(&names_b.len())));

    for (name, route_stop_name) in route_stop_names {
        println!("{name}: {route_stop_name:?}");
        println!();
    }
}

struct StopIterator {
    bitfield: U256,
    index: usize,
}

impl StopIterator {
    fn new(bitfield: U256) -> Self {
        StopIterator {
            bitfield,
            index: 0,
        }
    }
}

impl Iterator for StopIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < 255 {
            if self.bitfield & (U256::ONE << self.index) != U256::ZERO {
                let result = self.index;
                self.index += 1;
                return Some(result);
            }
            self.index += 1;
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gtfs = Gtfs::new("D:\\Projects\\Honours\\gtfs\\2\\google_transit.zip")?;

    let (stop_index, stops) = linearise(&gtfs.stops);
    let num_stops = stops.len();

    // Construct our own routes as collections of trips, because the ones defined in the GTFS contain different amounts of stops.
    assert!(num_stops < 255, "Too many stops in GTFS");
    let mut routes = HashMap::new();
    for (_, trip) in gtfs.trips.iter() {
        // Construct a 256 bit integer where the most significant bit is the direction of the trip, and the rest are stops.
        let mut stop_field = U256::from_digit(trip.direction_id.unwrap() as u64) << 255;
        for stop in trip.stop_times.iter() {
            let stop_idx = stop_index[stop.stop.id.as_str()];
            stop_field |= U256::ONE << stop_idx;
        }
        let route: &mut Vec<&Trip> = routes.entry(stop_field).or_default(); 
        route.push(trip);
    }

    // print_stop_names(&gtfs, &routes);

    // Sort trips in route based on earliest arrival time.
    for route_trips in routes.values_mut() {
        route_trips.sort_unstable_by(|a, b| {
            a.stop_times[0].arrival_time.cmp(&b.stop_times[0].arrival_time)
        });
    }

    // Index the routes for a given stop.
    let mut routes_for_stops = vec![Vec::new(); num_stops];
    for (i, route_trip) in routes.values().enumerate() {
        // Record the routes that use a particular stop.
        for stop_time in route_trip[0].stop_times.iter() {
            routes_for_stops[stop_index[stop_time.stop.id.as_str()]].push(i);
        }
    }

    let start = stop_index["19866"];
    let start_time = utils::parse_time("8:30:00").unwrap();
    let end = stop_index["19985"];

    println!("Start: {} at time {start_time}", stops[start].name.as_ref().unwrap());
    println!("End: {}", stops[end].name.as_ref().unwrap());

    // τ[p][i] = earliest known arrival time at stop p with up to i trips.
    // TODO: see if indices would benefit from being swapped around with profiling.
    let mut tau = vec![[u32::MAX; K]; num_stops];

    // Set initial departure time from start station.
    tau[start][0] = start_time;

    let mut touched_stops = vec![false; num_stops];
    touched_stops[start] = true;

    
    // RAPTOR
    for k in 1..K {
        // Stage 1
        for stop in 0..num_stops {
            tau[stop][k] = tau[stop][k - 1];
        }

        // Stage 2
        for (stops, route) in routes.iter() {
            let mut earliest_trip = None;
            for (stop_num, stop_time) in route[0].stop_times.iter().enumerate() {
                let stop_idx = stop_index[stop_time.stop.id.as_str()];
                if let Some(current_trip) = earliest_trip {
                    // Traverse this trip
                } else {
                    for trip in route.iter() {
                        let stop_time = trip.stop_times[stop_num].departure_time.unwrap();
                        if stop_time > tau[stop_idx][k - 1] {
                            // Hop on this trip
                            earliest_trip = Some(trip);
                            break;
                        }
                    }
                }
            }
        }

        // touched_stops.fill(false);
    }

    Ok(())
}
