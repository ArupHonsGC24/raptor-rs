use crate::{Journey, Network};
use crate::journey::{Boarding, TauEntry};
use crate::network::{StopIndex, Timestamp};

// Run a connection scanning algorithm (CSA) query on the network.
pub fn csa_query(network: &Network, start: StopIndex, start_time: Timestamp, end: StopIndex) -> Journey {
    if start == end {
        return Journey::from(vec![], network);
    }
    
    // Require connections be built
    debug_assert!(network.connections.len() > 0, "Connections must be built before running CSA.");

    let start = start as usize;
    let end = end as usize;

    //  τ[i] records the earliest arrival time at stop i.
    let mut tau  = vec![TauEntry::default(); network.stops.len()];
    tau[start] = TauEntry { time: start_time, boarding: None };
    let mut end_time = Timestamp::MAX;

    let mut trip_reachable = vec![false; network.num_trips as usize];

    // Start Criterion Optimisation: Binary search start connection.
    let start_connection = 0;

    for connection in &network.connections[start_connection..] {
        if connection.departure_time >= end_time {
            break;
        }

        let unique_trip_idx = connection.unique_trip_idx as usize;
        let departure_idx = connection.departure_idx as usize;
        if !trip_reachable[unique_trip_idx] {
            if tau[departure_idx].time > connection.departure_time {
                // Unreachable.
                continue;
            }

            // Reachable.
            trip_reachable[unique_trip_idx] = true;
        }

        let arrival_idx = connection.arrival_idx as usize;
        if connection.arrival_time < tau[arrival_idx].time {
            tau[arrival_idx].time = connection.arrival_time;

            if let Some(boarding) = tau[departure_idx].boarding.clone() {
                // If travelling along the same trip, use the same boarding.
                if boarding.trip_idx == connection.trip_idx && boarding.route_idx == connection.route_idx {
                    tau[arrival_idx].boarding = Some(boarding);
                } else {
                    tau[arrival_idx].boarding = Some(Boarding::from(connection));
                }
            } else {
                // This should only happen to the start stop.
                debug_assert!(departure_idx == start);
                tau[departure_idx].boarding = Some(Boarding::from(connection));
                tau[arrival_idx].boarding = tau[departure_idx].boarding.clone();
            }

            if arrival_idx == end {
                end_time = connection.arrival_time;
            }
        }
    }

    Journey::from_tau(&tau, network, start as StopIndex, end as StopIndex)
}