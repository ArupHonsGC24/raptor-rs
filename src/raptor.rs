use crate::journey::{Boarding, JourneyError, JourneyPreferences, TauEntry};
use crate::multicriteria::{Bag, Label};
use crate::network::{GlobalTripIndex, Network, PathfindingCost, Route, RouteIndex, StopIndex, Timestamp, TripOrder};
use crate::utils::{self, OptionExt};
use crate::Journey;

// Number of rounds to run RAPTOR for.
const K: usize = 8;

struct MarkedStops {
    marked_stops: Vec<bool>,
}

impl MarkedStops {
    pub fn new(network: &Network) -> Self {
        Self {
            marked_stops: vec![false; network.stops.len()],
        }
    }

    pub fn mark_stop(&mut self, stop_idx: usize) {
        self.marked_stops[stop_idx] = true;
    }

    // Calculates the equivalent of the set Q in the paper, and iterates over (route_idx, earliest_stop_order) pairs.
    pub fn iter_marked_routes(&mut self, network: &Network) -> impl Iterator<Item=(usize, usize)> {
        let mut earliest_stop_for_route = vec![None; network.routes.len()];
        for marked_stop in
            self.marked_stops
                .iter()
                .enumerate()
                .filter_map(|(i, &touched)| if touched { Some(i) } else { None })
        {
            for &route_idx in network.stops[marked_stop].get_routes(&network.stop_routes) {
                let route_idx = route_idx as usize;
                let route = &network.routes[route_idx];
                let earliest_stop_in_route_order =
                    earliest_stop_for_route[route_idx].unwrap_or(route.num_stops as usize);

                // TODO: profile to test if stop orders in a route for stop indices should be cached.
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
                debug_assert!(earliest_stop_for_route[route_idx].is_some());
            }
        }
        self.marked_stops.fill(false);

        earliest_stop_for_route.into_iter()
                               .enumerate()
                               .filter_map(|(i, stop)| stop.map(|s| (i, s)))
    }

    pub fn is_empty(&self) -> bool {
        utils::is_zero(&self.marked_stops)
    }
}

// Compute et(r, p).
// Returns the earliest trip boardable from the given stop on the given route before the given time as well as its departure time at the given stop.
fn earliest_trip(network: &Network, route: &Route, stop_order: usize, time: Timestamp, boarding: Option<&Boarding>) -> Option<(usize, Timestamp)> {
    // This is the trip we are currently on.
    // An exclusive range is used below, so we don't scan the current trip and to scan all trips we use num_trips as the default.
    let current_trip_order = match boarding {
        Some(boarding) => boarding.trip.trip_order,
        None => route.num_trips,
    } as usize;

    // Because the trip index can only ever decrease, we start from the next earliest trip and work our way back.
    // Thus, all trips are accessed at most once each round.
    let found_trip_order = (0..current_trip_order)
        .rev()
        .map(|trip_order| {
            // We want to save the departure time of the trip we select.
            (
                trip_order,
                network.stop_times[route.get_index_in_trip(trip_order, stop_order)].departure_time,
            )
        })
        .take_while(|(_, departure_time)| {
            time <= *departure_time
        })
        .last();

    found_trip_order
}

pub fn raptor_query(network: &Network, start: StopIndex, start_time: Timestamp, end: StopIndex) -> Result<Journey, JourneyError> {
    let start = start as usize;
    let end = end as usize;
    let num_stops = network.stops.len();

    // τ[p][i] = earliest known arrival time at stop p with up to i trips.
    let mut tau = vec![[Timestamp::MAX; K]; num_stops];
    // τ*[p] = earliest known arrival time at stop p.
    let mut tau_star = vec![TauEntry::default(); num_stops];

    // Set initial departure time from start station.
    tau[start][0] = start_time;
    tau_star[start] = TauEntry { time: start_time, boarding: None };

    // Array for recording which stops have been marked in the current round.
    let mut marked_stops = MarkedStops::new(network);
    marked_stops.mark_stop(start);

    // RAPTOR
    for k in 1..K {
        // Traverse each marked route.
        for (route_idx, earliest_stop_order) in marked_stops.iter_marked_routes(network)
        {
            let route = &network.routes[route_idx];

            // This keeps track of when and where we got on the current trip.
            let mut boarding: Option<Boarding> = None;
            for (stop_order, stop_idx) in route.iter_stops(earliest_stop_order, &network.route_stops)
            {
                // Can the arrival time at this stop be improved in this round?
                let mut current_departure_time = None;
                if let Some(boarding) = &boarding {
                    let trip = route.get_trip(boarding.trip.trip_order as usize, &network.stop_times);
                    let arrival_time = trip[stop_order].arrival_time;
                    current_departure_time = Some(trip[stop_order].departure_time);
                    if arrival_time < tau_star[stop_idx].time.min(tau_star[end].time) {
                        tau[stop_idx][k] = arrival_time;
                        tau_star[stop_idx] = TauEntry { time: arrival_time, boarding: Some(boarding.clone()) };
                        marked_stops.mark_stop(stop_idx);
                    }
                }

                // NOTE: Why is this after the code to update this stop? 
                // Because there are two cases where we update the current trip:
                // 1. This is the first stop in the trip. The stop was therefore set by the previous round.
                // 2. This is a subsequent stop in the trip, where another route has reached it faster. Similarly, it has already been updated to the fastest time.

                // Ignore transfer time for first round.
                let transfer_time = if k > 1 {
                    network.transfer_times[stop_idx]
                } else {
                    0
                };

                // Can we catch an earlier trip at this stop?
                let current_tau = tau[stop_idx][k - 1].saturating_add(transfer_time);
                if OptionExt::is_none_or(current_departure_time, |departure_time| current_tau <= departure_time)
                {
                    // If no new trip was found, we continue with the current trip.
                    // If a new trip was found, we update the trip and the stop we boarded it.
                    if let Some((found_trip_order, departure_time)) = earliest_trip(network, route, stop_order, current_tau, boarding.as_ref()) {
                        boarding = Some(
                            Boarding {
                                boarded_stop: stop_idx as StopIndex,
                                boarded_stop_order: stop_order as StopIndex,
                                boarded_time: departure_time,
                                trip: GlobalTripIndex {
                                    route_idx: route_idx as RouteIndex,
                                    trip_order: found_trip_order as TripOrder
                                },
                            },
                        )
                    }
                }
            }
        }

        if marked_stops.is_empty() {
            break;
        }
    }

    Journey::from_tau(&tau_star, network, start, end)
}

pub fn mc_raptor_query<'a>(network: &'a Network, 
                           start: StopIndex, 
                           start_time: Timestamp, 
                           end: StopIndex, 
                           costs: &[PathfindingCost], 
                           path_preferences: &JourneyPreferences) -> Result<Journey<'a>, JourneyError> {
    if start == end {
        return Ok(Journey::empty(network));
    }
    
    let start = start as usize;
    let end = end as usize;
    let num_stops = network.stops.len();

    // τ[p][i] = earliest known arrival time at stop p with up to i trips.
    let mut tau = vec![[const { Bag::new() }; K]; num_stops];
    // τ*[p] = earliest known arrival time at stop p.
    let mut tau_star = vec![Bag::new(); num_stops];

    // Set initial departure time from start station.
    let start_label = Label::new(start_time, 0.);
    tau[start][0].add(start_label.clone());
    tau_star[start].add(start_label);

    // Array for recording which stops have been marked in the current round.
    let mut marked_stops = MarkedStops::new(network);
    marked_stops.mark_stop(start);

    // RAPTOR
    for k in 1..K {
        // Traverse each marked route.
        for (route_idx, earliest_stop_order) in marked_stops.iter_marked_routes(network)
        {
            let route = &network.routes[route_idx];

            // B_r
            let mut route_bag = Bag::new();

            // This keeps track of when and where we got on the current trip.
            for (stop_order, stop_idx) in route.iter_stops(earliest_stop_order, &network.route_stops)
            {
                // Multicriteria step 1: Update arrival time of every label in Br according to each labels' trip.
                {
                    let mut new_bag = Bag::new();
                    for label in route_bag.labels.iter() {
                        let boarding = label.boarding.as_ref().unwrap();
                        assert_eq!(boarding.trip.route_idx, route_idx as RouteIndex);
                        let index = route.get_index_in_trip(boarding.trip.trip_order as usize, stop_order);
                        new_bag.add(Label {
                            arrival_time: network.stop_times[index].arrival_time,
                            cost: label.cost + costs[index],
                            boarding: label.boarding.clone(),
                        });
                    }
                    route_bag.labels = new_bag.labels;
                }

                // Multicriteria step 2: Merge B_r into B_k.
                // TODO: Only have boarding data in route bag.
                let mut updated = false;
                for label in &route_bag.labels {
                    if !tau_star[stop_idx].dominates(label) && !tau_star[end].dominates(label) {
                        updated |= tau[stop_idx][k].add(label.clone());
                        updated |= tau_star[stop_idx].add(label.clone());
                    }
                }
                if updated {
                    marked_stops.mark_stop(stop_idx);
                }

                // Multicriteria step 3: Merge B_{k-1} into B_r and assign trips.
                for label in tau[stop_idx][k - 1].labels.iter() {
                    // NOTE: Why is this after the code to update this stop?
                    // Because there are two cases where we update the current trip:
                    // 1. This is the first stop in the trip. The stop was therefore set by the previous round.
                    // 2. This is a subsequent stop in the trip, where another route has reached it faster. Similarly, it has already been updated to the fastest time.

                    // Ignore transfer time for first round.
                    let transfer_time = if k > 1 {
                        network.transfer_times[stop_idx]
                    } else {
                        0
                    };

                    // Can we catch an earlier trip at this stop?
                    let current_tau = label.arrival_time.saturating_add(transfer_time);
                    // TODO: Is there a way to use the existing boarding to optimise the earliest trip calculation? (Currently, the label sometimes has the wrong route.)
                    // if let Some(boarding) = label.boarding.as_ref() {
                    //     assert_eq!(boarding.route_idx, route_idx as RouteIndex);
                    // }
                    if let Some((found_trip_order, departure_time)) = earliest_trip(network, route, stop_order, current_tau, None/*label.boarding.as_ref()*/) {
                        let new_label = Label {
                            arrival_time: label.arrival_time,
                            cost: label.cost,
                            boarding: Some(
                                Boarding {
                                    boarded_stop: stop_idx as StopIndex,
                                    boarded_stop_order: stop_order as StopIndex,
                                    boarded_time: departure_time,
                                    trip: GlobalTripIndex {
                                        route_idx: route_idx as RouteIndex,
                                        trip_order: found_trip_order as TripOrder
                                    },
                                },
                            ),
                        };

                        route_bag.add(new_label);
                    }
                }
            }
        }

        if marked_stops.is_empty() {
            break;
        }
    }

    Journey::from_tau_bag(&tau_star, network, start, end, path_preferences)
}
