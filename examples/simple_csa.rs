use chrono::NaiveDate;
use raptor::{csa_query, utils, Network};

mod common;
use common::load_example_gtfs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load GTFS timetable from disk.
    let gtfs = load_example_gtfs()?;

    // Parse into network format, based on a specific date.
    let date = const { utils::const_unwrap(NaiveDate::from_ymd_opt(2024, 5, 10)) };
    let transfer_time = 3 * 60; // 3 minutes transfer time.
    let mut network = Network::new(&gtfs, date, transfer_time);
    // CSA requires the connections to be built.
    network.build_connections();
    network.print_stats();

    // Run raptor query.
    let start = network.get_stop_idx_from_name("Cheltenham").unwrap();
    let start_time = utils::parse_time("08:30:00")?;
    let end = network.get_stop_idx_from_name("Greensborough").unwrap();
    let journey = csa_query(&network, start, start_time, end);

    println!("{journey}");

    Ok(())
}
