use crate::journey::Connection;
use crate::utils;
use chrono::NaiveDate;
use gtfs_structures::{DirectionType, Gtfs, Trip};
use rgb::RGB8;
use std::collections::HashMap;
use std::sync::Arc;

// Timestamp is seconds since midnight.
pub type Timestamp = u32;
pub type StopIndex = u32;
pub type StopBitfield = bnum::BUint<7>; // Maximum 64*7 = 448 stops per route. This is required for the 901 bus route in Melbourne?

const STOP_BITFIELD_SIZE_BITS: usize = utils::get_size_bits::<StopBitfield>();

pub type RouteIndex = u32;
pub type TripOrder = u32;
pub type PathfindingCost = f32;

pub type CoordType = f32;

// Used to globally identify a trip in the network.
#[derive(Default, Clone, Copy, PartialEq)]
pub struct GlobalTripIndex {
    pub route_idx: RouteIndex,
    pub trip_order: TripOrder,
}

#[derive(Clone, Copy)]
pub struct NetworkPoint {
    pub latitude: CoordType,
    pub longitude: CoordType,
}

impl NetworkPoint {
    const EARTH_RADIUS: CoordType = 6371.; // km
    const CLOSE_THRESHOLD: CoordType = 0.1; // 0.1 km = 100 m, because shaped points sometimes aren't exactly on station points. Closest stations are 504 m apart (West and North Richmond).

    // Equirectangular projection (on a unit sphere).
    pub fn equirectangular_delta(self, other: NetworkPoint) -> (CoordType, CoordType) {
        let x = (other.longitude - self.longitude).to_radians() * ((other.latitude + self.latitude) * 0.5).to_radians().cos();
        let y = (other.latitude - self.latitude).to_radians();
        (x * Self::EARTH_RADIUS, y * Self::EARTH_RADIUS)
    }

    // Distance is returned in km.
    pub fn distance(self, other: NetworkPoint) -> CoordType {
        // Equirectangular projection works for small distances.
        let (x, y) = self.equirectangular_delta(other);
        return (x * x + y * y).sqrt();

        // Haversine formula.
        //let lat_diff = (self.latitude - other.latitude).to_radians();
        //let lon_diff = (self.longitude - other.longitude).to_radians();
        //let a = (lat_diff / 2.0).sin().powi(2)
        //    + self.latitude.to_radians().cos()
        //        * other.latitude.to_radians().cos()
        //        * (lon_diff / 2.0).sin().powi(2);
        //let c = 2.0 * a.sqrt().asin();
        //Self::EARTH_RADIUS * c
    }

    #[allow(dead_code)]
    pub fn very_close(self, other: NetworkPoint) -> bool {
        self.distance(other) < Self::CLOSE_THRESHOLD
    }

    // Used to offset shape based on the direction of the trip, so that inbound and outbound trips are drawn on opposite sides of the track.
    // Offset is given in metres.
    #[allow(dead_code)]
    pub fn left_offset(&self, next_point: NetworkPoint, offset: CoordType) -> NetworkPoint {
        let lat1_rad = self.latitude.to_radians();
        let lon1_rad = self.longitude.to_radians();
        let lat2_rad = next_point.latitude.to_radians();
        let lon2_rad = next_point.longitude.to_radians();

        let (lat1_sin, lat1_cos) = lat1_rad.sin_cos();
        let (lat2_sin, lat2_cos) = lat2_rad.sin_cos();
        let delta_long = lon2_rad - lon1_rad;
        let (delta_long_sin, delta_long_cos) = delta_long.sin_cos();

        // Calculate bearing: https://www.movable-type.co.uk/scripts/latlong.html.
        let y = delta_long_sin * lat2_cos;
        let x = lat1_cos * lat2_sin - lat1_sin * lat2_cos * delta_long_cos;

        // Find bearing, and rotate anticlockwise by 90 degrees.
        let bearing = y.atan2(x) - (90 as CoordType).to_radians();
        let (bearing_sin, bearing_cos) = bearing.sin_cos();

        let offset_rad = offset * 0.001 / Self::EARTH_RADIUS;
        let (offset_sin, offset_cos) = offset_rad.sin_cos();
        let lat = (lat1_sin * offset_cos + lat1_cos * offset_sin * bearing_cos).asin();
        let lon = lon1_rad + (bearing_sin * offset_sin * lat1_cos).atan2(offset_cos - lat1_sin * lat.sin());

        NetworkPoint {
            latitude: lat.to_degrees(),
            longitude: lon.to_degrees(),
        }

        // Equirectangular projection works for small distances.
        // let delta_longitude = self.longitude - other.longitude;
        // let delta_latitude = self.latitude - other.latitude;
        // let size = (delta_longitude * delta_longitude + delta_latitude * delta_latitude).sqrt();
        // let normal = (delta_longitude, delta_latitude) / size;
        // let rotated_normal = (normal.1, -normal.0);
        // 
        // return NetworkPoint {
        //     latitude: self.latitude + rotated.1.to_degrees(),
        //     longitude: self.longitude + rotated.0.to_radians(),
        // };
    }
}

pub struct Route {
    pub line: Arc<str>,
    pub num_stops: StopIndex,
    pub num_trips: TripOrder,
    pub route_stops_idx: usize,
    pub stop_times_idx: usize,
    // Visual properties
    pub trip_ids: Vec<Box<str>>,
    pub colour: RGB8,
    pub shape: Box<[NetworkPoint]>,
    pub shape_height: CoordType,
}

impl Route {
    pub fn get_stops<'a>(&self, route_stops: &'a [StopIndex]) -> &'a [StopIndex] {
        &route_stops[self.route_stops_idx..(self.route_stops_idx + self.num_stops as usize)]
    }
    pub fn iter_stops<'a>(&self, earliest_stop_order: usize, route_stops: &'a [StopIndex]) -> impl Iterator<Item=(usize, usize)> + 'a {
        self.get_stops(route_stops)
            .iter()
            .map(|&stop| stop as usize)
            .enumerate()
            .skip(earliest_stop_order)
    }
    pub fn get_trip_range(&self, trip_order: usize) -> std::ops::Range<usize> {
        let start = self.stop_times_idx + trip_order * self.num_stops as usize;
        let end = start + self.num_stops as usize;
        start..end
    }
    pub fn get_stop_times_index(&self, trip_order: usize, stop_order: usize) -> usize {
        let trip_range = self.get_trip_range(trip_order);
        let index = trip_range.start + stop_order;
        debug_assert!(trip_range.contains(&index));
        index
    }
    pub fn get_trip<'a>(&self, trip_order: usize, stop_times: &'a [StopTime]) -> &'a [StopTime] {
        &stop_times[self.get_trip_range(trip_order)]
    }
}

pub struct StopTime {
    pub arrival_time: Timestamp,
    pub departure_time: Timestamp,
}

#[derive(Debug)]
pub struct Stop {
    pub name: Box<str>,
    pub id: Box<str>,
    pub routes_idx: usize,
    pub num_routes: usize,
}

impl Stop {
    pub fn new(name: &str, id: &str) -> Self {
        Self {
            name: name.to_owned().into_boxed_str(),
            id: id.to_owned().into_boxed_str(),
            routes_idx: 0,
            num_routes: 0,
        }
    }

    pub fn get_routes<'a>(&self, stop_routes: &'a [RouteIndex]) -> &'a [RouteIndex] {
        &stop_routes[self.routes_idx..(self.routes_idx + self.num_routes)]
    }
}

pub struct Network {
    // Metadata for routes in the network.
    pub routes: Vec<Route>,
    // Metadata for stops in the network.
    pub stops: Vec<Stop>,
    // Number of trips. Not encoded anywhere else, like stops.len().
    pub num_trips: TripOrder,
    // The stop index for a given stop ID.
    pub stop_index: HashMap<String, StopIndex>,
    // The stop times for each trip (Indexed by [route.stop_times_idx..(route.stop_times_idx + route.num_trips * route.num_stops)]).
    pub stop_times: Vec<StopTime>,
    // The routes for each route (Indexed by [stop.routes_idx..(self.routes_idx + self.num_routes)]).
    pub stop_routes: Vec<RouteIndex>,
    // The stops in each route (Indexed by [route.route_stops_idx..(route.route_stops_idx + route.num_stops)]).
    pub route_stops: Vec<StopIndex>,
    // The Latitudes and Longitudes of each stop.
    pub stop_points: Vec<NetworkPoint>,
    // A linear list of all connections in the network.
    pub connections: Vec<Connection>,
    // Transfer time between stops in seconds (Indexed by stop index).
    pub transfer_times: Vec<Timestamp>,
    // The date for which the network is valid.
    pub date: NaiveDate,
    pub has_shapes: bool,
}

impl Network {
    pub fn new(gtfs: &Gtfs, journey_date: NaiveDate, default_transfer_time: Timestamp) -> Self {
        // GTFS optional fields that are unwrapped: stop.name, trip.direction_id, stop_time.arrival_time, stop_time.departure_time.

        // We use one stop index as the direction of the trip when grouping as routes.
        assert!(
            gtfs.stops.len() < (StopIndex::MAX - 1) as usize,
            "Too many stops ({}, max {}) in GTFS.",
            gtfs.stops.len(),
            StopIndex::MAX
        );

        let mut stop_index = HashMap::with_capacity(gtfs.stops.capacity());
        let mut stops = Vec::with_capacity(gtfs.stops.len());
        for (i, (id, value)) in gtfs.stops.iter().enumerate() {
            stop_index.insert(id.clone(), i as StopIndex);
            stops.push(Stop::new(utils::get_short_stop_name(value.name.as_ref().unwrap()), id));
        }

        // Construct route-local stop indices.
        struct RouteStopIndices<'a> {
            num_stops: StopIndex,
            mapping: Vec<Option<StopIndex>>,
            trips: Vec<&'a Trip>,
        }
        impl RouteStopIndices<'_> {
            fn default(len: usize) -> Self {
                Self { num_stops: 0, mapping: vec![None; len], trips: Vec::new() }
            }
        }

        let mut route_stop_indices = HashMap::<&str, RouteStopIndices>::new();

        for trip in gtfs.trips.values() {
            if !utils::does_trip_run(&gtfs, &trip, journey_date) {
                continue;
            }

            let route = route_stop_indices.entry(trip.route_id.as_str()).or_insert(RouteStopIndices::default(stops.len()));

            // Group trips by GTFS route.
            route.trips.push(trip);

            for stop_time in trip.stop_times.iter() {
                let stop_idx = &mut route.mapping[stop_index[stop_time.stop.id.as_str()] as usize];
                if stop_idx.is_none() {
                    *stop_idx = Some(route.num_stops);
                    route.num_stops += 1;
                }
            }
        }

        // Construct our own routes as collections of trips, because the ones defined in the GTFS contain different amounts of stops.

        let mut route_maps = Vec::new();

        let mut num_routes = 0;
        for (&route_id, RouteStopIndices { num_stops, mapping, trips }) in route_stop_indices.iter() {
            // Check that there aren't too many stops in a route.
            let num_stops = *num_stops as usize;
            if num_stops == 0 {
                continue;
            }
            if num_stops >= STOP_BITFIELD_SIZE_BITS {
                log::error!("Too many stops in route {route_id} ({}, max {}).", num_stops, STOP_BITFIELD_SIZE_BITS - 1);
                for (stop_idx, mapped_stop) in mapping.iter().enumerate() {
                    if mapped_stop.is_some() {
                        log::error!("Stop: {}", stops[stop_idx].name);
                    }
                }
                assert!(false, "Too many stops in route {route_id} ({}, max {}).", num_stops, STOP_BITFIELD_SIZE_BITS - 1);
                continue;
            }

            let mut route_map = HashMap::new();
            let direction_bit = StopBitfield::ONE << (STOP_BITFIELD_SIZE_BITS - 1);
            for &trip in trips.iter() {
                // Construct a big integer where the most significant bit is the direction of the trip, and the rest are stops.
                let mut stop_field = match trip.direction_id.unwrap_or_else(|| {
                    // TODO: Can the direction be calculated in the absence of a direction_id?
                    log::warn!("Trip {} has no direction_id, assuming outbound.", trip.id);
                    DirectionType::Outbound
                }) {
                    DirectionType::Inbound => direction_bit,
                    DirectionType::Outbound => StopBitfield::ZERO,
                };
                for stop_time in trip.stop_times.iter() {
                    let stop_idx = stop_index[stop_time.stop.id.as_str()] as usize;
                    let route_relative_stop_idx = mapping[stop_idx].unwrap();
                    stop_field |= StopBitfield::ONE << route_relative_stop_idx;
                }
                let route: &mut Vec<&Trip> = route_map.entry(stop_field).or_default();
                route.push(trip);
            }

            num_routes += route_map.len();
            route_maps.push(route_map);
        }

        assert!(
            num_routes < RouteIndex::MAX as usize,
            "Too many routes in GTFS (we currently use a {}-bit index for routes).",
            utils::get_size_bits::<RouteIndex>()
        );
        assert!(
            gtfs.trips.len() < TripOrder::MAX as usize,
            "Too many trips in GTFS (we currently use a {}-bit index for trips).",
            utils::get_size_bits::<TripOrder>()
        );

        // Construct routes, which point to a series of stops and stop times.
        let mut routes = Vec::new();
        let mut route_stops = Vec::new();
        let mut stop_times = Vec::new();
        let mut num_trips = 0 as TripOrder;

        // Keep track of the height of each colour.
        let mut colour_to_height_map = HashMap::new();
        let mut last_height = 0. as CoordType;

        for route_map in route_maps.iter_mut() {
            for route_trips in route_map.values_mut() {
                let first_trip = match route_trips.get(0) {
                    Some(&first_trip) => first_trip,
                    None => continue,
                };

                // Sort trips in route based on earliest arrival time.
                route_trips.sort_unstable_by_key(|x| { x.stop_times[0].arrival_time });

                let first_route = &gtfs.routes[first_trip.route_id.as_str()];
                let line_name = first_route.short_name.as_ref().unwrap_or(first_route.long_name.as_ref().unwrap_or(&first_trip.route_id));

                // Determine height based on colour. TODO: Hardcode heights for colours for consistency.
                let colour = first_route.color;
                let height = if let Some(&height) = colour_to_height_map.get(&colour) {
                    height
                } else {
                    last_height += 10.;
                    colour_to_height_map.insert(colour, last_height);
                    last_height
                };

                // Extract shape.
                let shape = if gtfs.shapes.len() > 0 {
                    if let Some(shape_id) = first_trip.shape_id.as_ref() {
                        let shapes = &gtfs.shapes[shape_id.as_str()];
                        let mut shape = Vec::with_capacity(shapes.len());
                        for shape_point in shapes.iter() {
                            shape.push(NetworkPoint {
                                longitude: shape_point.longitude as CoordType,
                                latitude: shape_point.latitude as CoordType,
                            });
                        }
                        shape
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                routes.push(Route {
                    line: Arc::from(line_name.as_str()),
                    num_stops: first_trip.stop_times.len() as StopIndex,
                    num_trips: route_trips.len() as TripOrder,
                    route_stops_idx: route_stops.len(),
                    stop_times_idx: stop_times.len(),
                    trip_ids: route_trips.iter().map(|trip| trip.id.clone().into_boxed_str()).collect(),
                    colour,
                    shape: shape.into_boxed_slice(),
                    shape_height: height,
                });

                // Because of how routes are constructed, all trips in a route have the same stops.
                // So grab the stops from the first trip.
                for stop_time in first_trip.stop_times.iter() {
                    route_stops.push(stop_index[stop_time.stop.id.as_str()]);
                }

                num_trips += route_trips.len() as TripOrder;

                for trip in route_trips {
                    for stop_time in trip.stop_times.iter() {
                        stop_times.push(StopTime {
                            arrival_time: stop_time.arrival_time.unwrap(),
                            departure_time: stop_time.departure_time.unwrap(),
                        });
                    }
                }
            }
        }

        // Index the routes for a given stop.
        let mut stop_routes_map = vec![Vec::new(); stops.len()];
        for (route_idx, route) in routes.iter().enumerate() {
            for &stop in route.get_stops(&route_stops) {
                stop_routes_map[stop as usize].push(route_idx as RouteIndex);
            }
        }

        let mut stop_routes = Vec::new();
        for (stop_idx, stop) in stops.iter_mut().enumerate() {
            stop.routes_idx = stop_routes.len();
            for &route_idx in stop_routes_map[stop_idx].iter() {
                stop_routes.push(route_idx);
            }
            stop.num_routes = stop_routes.len() - stop.routes_idx;
        }

        // Precalculate stop points.
        let mut stop_points = Vec::with_capacity(stops.len());
        for stop_id in gtfs.stops.keys() {
            let stop = &gtfs.stops[stop_id];
            stop_points.push(NetworkPoint { longitude: stop.longitude.unwrap_or(0.) as CoordType, latitude: stop.latitude.unwrap_or(0.) as CoordType });
        }

        let transfer_times = vec![default_transfer_time; stops.len()];

        Self {
            routes,
            stops,
            num_trips,
            stop_index,
            stop_times,
            stop_routes,
            route_stops,
            stop_points,
            connections: Vec::new(), // These will be built later if required.
            transfer_times,
            date: journey_date,
            has_shapes: gtfs.shapes.len() > 0,
        }
    }

    pub fn set_transfer_time_for_stop(&mut self, stop_id: &str, transfer_time: Timestamp) {
        let stop_idx = self.get_stop_idx(stop_id) as usize;
        self.transfer_times[stop_idx] = transfer_time;
    }

    // Call build connections if running a CSA query. 
    pub fn build_connections(&mut self) {
        // Construct list of connections from trips in network.
        let mut connections = Vec::new();
        let mut sequential_trip_idx = 0 as TripOrder;
        for (route_idx, route) in self.routes.iter().enumerate() {
            let route_idx = route_idx as RouteIndex;
            let num_stops = route.num_stops as usize;
            let stops = route.get_stops(&self.route_stops);
            for trip_order in 0..route.num_trips as usize {
                let trip = route.get_trip(trip_order, &self.stop_times);
                let trip_order = trip_order as TripOrder;
                for arrival_stop_order in 1..num_stops {
                    let departure_stop_order = arrival_stop_order - 1;
                    connections.push(Connection {
                        sequential_trip_idx,
                        trip: GlobalTripIndex {
                            route_idx,
                            trip_order,
                        },
                        departure_idx: stops[departure_stop_order],
                        departure_stop_order: departure_stop_order as StopIndex,
                        departure_time: trip[departure_stop_order].departure_time,
                        arrival_idx: stops[arrival_stop_order],
                        arrival_time: trip[arrival_stop_order].arrival_time,
                    });
                }
                sequential_trip_idx += 1;
            }
        }

        // Sort connections by departure time.
        connections.sort_unstable_by_key(|x| x.departure_time);

        self.connections = connections;
    }

    pub fn get_stop(&self, stop: usize) -> &Stop { &self.stops[stop] }

    pub fn get_stop_idx(&self, stop_id: &str) -> StopIndex { self.stop_index[stop_id] }

    pub fn stop_name_cmp(a: &str, b: &str) -> bool {
        utils::get_short_stop_name(a).to_lowercase().replace(" ", "") == b.to_lowercase().replace(" ", "")
    }

    pub fn get_stop_idx_from_name(&self, stop_name: &str) -> Option<StopIndex> {
        self.stops.iter().position(|stop| Network::stop_name_cmp(&stop.name, stop_name)).map(|stop_idx| stop_idx as StopIndex)
    }

    pub fn get_stop_in_route(&self, route_idx: usize, stop_order: usize) -> StopIndex {
        self.routes[route_idx].get_stops(&self.route_stops)[stop_order]
    }

    pub fn get_departure_time(&self, route_idx: usize, trip_idx: usize, stop_idx: usize) -> Timestamp {
        self.get_trip(route_idx, trip_idx)[stop_idx].departure_time
    }

    pub fn get_arrival_time(&self, route_idx: usize, trip_idx: usize, stop_idx: usize) -> Timestamp {
        self.get_trip(route_idx, trip_idx)[stop_idx].arrival_time
    }

    pub fn num_stops(&self) -> usize { self.stops.len() }

    pub fn num_routes(&self) -> usize { self.routes.len() }

    pub fn num_trips(&self, route_idx: usize) -> usize { self.routes[route_idx].num_trips as usize }

    pub fn num_stops_in_route(&self, route_idx: usize) -> usize { self.routes[route_idx].num_stops as usize }

    pub fn get_trip(&self, route_idx: usize, trip_idx: usize) -> &[StopTime] {
        let route = &self.routes[route_idx];
        route.get_trip(trip_idx, &self.stop_times)
    }

    pub fn get_trip_id(&self, trip_idx: GlobalTripIndex) -> &str {
        let route = &self.routes[trip_idx.route_idx as usize];
        route.trip_ids[trip_idx.trip_order as usize].as_ref()
    }

    pub fn print_stats(&self) {
        log::info!("Network has {} stops, {} routes, {} trips and {} connections.", self.stops.len(), self.routes.len(), self.num_trips, self.connections.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn west_north_richmond() {
        let west_richmond = NetworkPoint {
            latitude: -37.8149489647782,
            longitude: 144.991422784199,
        };
        let north_richmond = NetworkPoint {
            latitude: -37.8103983564789,
            longitude: 144.992500261754,
        };
        let distance = west_richmond.distance(north_richmond);
        assert!((distance - 0.5146).abs() < NetworkPoint::CLOSE_THRESHOLD)
    }
}