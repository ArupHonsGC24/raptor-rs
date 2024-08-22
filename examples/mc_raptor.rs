use chrono::NaiveDate;
use raptor::{mc_raptor_query, utils, Network};
use raptor::network::PathfindingCost;

mod common;
use common::load_example_gtfs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load GTFS timetable from disk.
    let gtfs = load_example_gtfs()?;

    // Parse into network format, based on a specific date.
    let date = const { utils::const_unwrap(NaiveDate::from_ymd_opt(2024, 5, 10)) };
    let transfer_time = 3 * 60; // 3 minutes transfer time.
    let network = Network::new(&gtfs, date, transfer_time);
    network.print_stats();

    // Run raptor query.
    let start = network.get_stop_idx_from_name("Cheltenham").unwrap();
    let start_time = utils::parse_time("08:30:00")?;
    let end = network.get_stop_idx_from_name("Greensborough").unwrap();

    // Random pathfinding costs.
    let mut costs = vec![0 as PathfindingCost; network.stop_times.len()];
    for cost in costs.iter_mut() {
        *cost = fastrand::f32() as PathfindingCost;
    }

    let journey = mc_raptor_query(&network, start, start_time, end, &costs);

    println!("{journey}");

    Ok(())
}