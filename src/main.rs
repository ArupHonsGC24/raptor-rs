mod utils;

use std::collections::HashMap;

use gtfs_structures::{Gtfs, Trip};

const K: usize = 8;

type Timestamp = u32;
type StopIndex = u8;

const STOP_BITFIELD_LENGTH: usize = (StopIndex::MAX / 64) as usize;

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

struct Route {
    pub num_stops: StopIndex,
    pub num_trips: TripIndex,
    pub route_stops_idx: usize,
    pub stop_times_idx: usize,
}

impl Route {
    pub fn get_stops<'a>(&self, route_stops: &'a [StopIndex]) -> &'a [StopIndex] {
        &route_stops[self.route_stops_idx..self.route_stops_idx + self.num_stops as usize]
    }
}

struct StopTime {
    pub arrival_time: Timestamp,
    pub departure_time: Timestamp,
}

struct Stop {
    pub name: String,
    pub routes_idx: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // On large networks GTFS parsing can overflow the stack, so we increase the stack size for this call.
    const GTFS_STACK_SIZE: usize = 2 * 1024 * 1024;
    let gtfs = std::thread::Builder::new().stack_size(GTFS_STACK_SIZE).spawn(|| {
        Gtfs::new("D:\\Projects\\Honours\\gtfs\\2\\google_transit.zip")
    })?.join().unwrap().unwrap();
    // let gtfs = Gtfs::new("D:\\Projects\\Honours\\gtfs\\3\\google_transit.zip")?;
    // GTFS optional fields that are unwrapped: stop.name, trip.direction_id, stop_time.arrival_time, stop_time.departure_time.
    println!("GTFS loaded with {} stops, {} routes, and {} trips.", gtfs.stops.len(), gtfs.routes.len(), gtfs.trips.len());
    // We use one stop index as the direction of the trip when grouping as routes.
    assert!(gtfs.stops.len() < (StopIndex::MAX - 1) as usize, "Too many stops ({}) in GTFS (we currently use a {}-bitfield for stops).", gtfs.stops.len(), StopIndex::MAX);

    return Ok(());
    
    let mut stop_index = HashMap::with_capacity(gtfs.stops.capacity());
    let mut stops = Vec::with_capacity(gtfs.stops.len());
    for (i, (id, value)) in gtfs.stops.iter().enumerate() {
        stop_index.insert(id.as_str(), i as StopIndex);
        stops.push(Stop {
            name: value.name.as_ref().unwrap().to_string(),
            routes_idx: 0,
        });
    }
    let num_stops = stops.len();

    // Construct our own routes as collections of trips, because the ones defined in the GTFS contain different amounts of stops.
    let mut routes_map = HashMap::new();
    for (_, trip) in gtfs.trips.iter() {
        // Construct a 256 bit integer where the most significant bit is the direction of the trip, and the rest are stops.
        let mut stop_field = StopBitfield::from_digit(trip.direction_id.unwrap() as u64) << STOP_BITFIELD_LENGTH-1;
        for stop in trip.stop_times.iter() {
            let stop_idx = stop_index[stop.stop.id.as_str()];
            stop_field |= StopBitfield::ONE << stop_idx;
        }
        let route: &mut Vec<&Trip> = routes_map.entry(stop_field).or_default();
        route.push(trip);
    }

    assert!(routes_map.len() < (RouteIndex::MAX - 1) as usize, "Too many routes in GTFS (we currently use a {}-bit index for routes).", std::mem::size_of::<RouteIndex>() * 8);
    assert!(gtfs.trips.len() < (TripIndex::MAX - 1) as usize, "Too many trips in GTFS (we currently use a {}-bit index for trips).", std::mem::size_of::<TripIndex>() * 8);
    // print_stop_names(&gtfs, &routes);

    let mut routes = Vec::new();
    let mut route_stops = Vec::new();
    let mut stop_times = Vec::new();
    for route_trips in routes_map.values_mut() {
        // Sort trips in route based on earliest arrival time.
        route_trips.sort_unstable_by(|a, b| {
            a.stop_times[0].arrival_time.cmp(&b.stop_times[0].arrival_time)
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
            if route.get_stops(&route_stops).contains(&(stop_idx as StopIndex)) {
                stop_routes.push(route_idx as RouteIndex);
            }
        }
    }
    

    let start = stop_index["19866"] as usize;
    let start_time = utils::parse_time("8:30:00").unwrap();
    let end = stop_index["19985"] as usize;

    // No more hashmap accesses?

    println!("Start: {} at time {start_time}", stops[start].name);
    println!("End: {}", stops[end].name);

    // τ[p][i] = earliest known arrival time at stop p with up to i trips.
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
        for (stops, route) in routes_map.iter() {
            let mut earliest_trip = None;
            for (stop_num, stop_time) in route[0].stop_times.iter().enumerate() {
                let stop_idx = stop_index[stop_time.stop.id.as_str()] as usize;
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
