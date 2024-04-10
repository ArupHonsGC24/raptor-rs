mod utils;
mod raptor;

use chrono::NaiveDate;
use gtfs_structures::Gtfs;

use raptor::Raptor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gtfs = Gtfs::new("../gtfs/2/google_transit.zip")?;

    // GTFS optional fields that are unwrapped: stop.name, trip.direction_id, stop_time.arrival_time, stop_time.departure_time.
    println!(
        "GTFS loaded with {} stops, {} routes, and {} trips.",
        gtfs.stops.len(),
        gtfs.routes.len(),
        gtfs.trips.len()
    );
    println!();

    let mut raptor = Raptor::new(&gtfs, NaiveDate::from_ymd_opt(2024, 4, 16).unwrap(), 3 * 60);
    // Hardcode extra time at Flinders Street Station.
    raptor.set_transfer_time_for_stop("19854", 4 * 60);

    let start = raptor.get_stop_idx("19866");
    let start_time = utils::parse_time("8:30:00").unwrap();
    let end = raptor.get_stop_idx("19985");

    println!(
        "Start: {} at time {}",
        raptor.get_stop(start).name,
        utils::get_time_str(start_time)
    );
    println!("End: {}", raptor.get_stop(end).name);
    println!();

    let journey = raptor.query(start, start_time, end);
    for leg in journey {
        println!(
            "Boarded at {} at {} ({})",
            utils::get_short_stop_name(&raptor.get_stop(leg.boarded_stop).name),
            utils::get_time_str(leg.boarded_time),
            leg.line,
        );
        println!(
            "Arrived at {} at {}",
            utils::get_short_stop_name(&raptor.get_stop(leg.arrival_stop).name),
            utils::get_time_str(leg.arrival_time)
        );
        println!();
    }

    Ok(())
}
