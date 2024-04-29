mod raptor;
mod utils;

use chrono::NaiveDate;
use gtfs_structures::Gtfs;
use std::io::{stdout, Write};

use raptor::Raptor;

pub fn get_stop_from_user(gtfs: &Gtfs, prompt: &str) -> Result<String, std::io::Error> {
    loop {
        print!("Where are you {prompt}? ");
        stdout().flush()?;
        let mut stop_name = String::new();
        std::io::stdin().read_line(&mut stop_name)?;
        let stop_name = stop_name.trim().to_lowercase();
        if let Some(stop) = gtfs.stops.values().find(|stop| {
            utils::get_short_stop_name(stop.name.as_ref().unwrap())
                .to_lowercase()
                .contains(&stop_name)
        }) {
            return Ok(stop.id.clone());
        }
        println!("Stop not found. Please try again.");
    }
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
    println!();

    // Get user input for query.
    let journey_date = loop {
        // Default to 2024.
        let mut date_str = String::from("2024/");
        print!("When are you travelling (in 2024)? (DD/MM): ");
        stdout().flush()?;
        std::io::stdin().read_line(&mut date_str)?;
        match NaiveDate::parse_from_str(date_str.trim(), "%Y/%d/%m") {
            Ok(parsed_date) => break parsed_date,
            Err(e) => {
                println!("Invalid date format: {e:?}. Please try again.");
            }
        }
    };

    println!();

    let mut raptor = Raptor::new(&gtfs, journey_date, 3 * 60);
    // Hardcode extra time at Flinders Street Station.
    raptor.set_transfer_time_for_stop("19854", 4 * 60);

    loop {
        let start = raptor.get_stop_idx(get_stop_from_user(&gtfs, "starting")?.as_str());
        let start_time = loop {
            let mut time_str = String::new();
            print!("What time are you starting? (HH:MM): ");
            stdout().flush()?;
            std::io::stdin().read_line(&mut time_str)?;
            // Remove trailing whitespace and append seconds so it can be parsed.
            let mut time_str = String::from(time_str.trim_end());
            time_str += ":00";
            match utils::parse_time(&time_str) {
                Ok(time) => break time,
                Err(e) => {
                    println!("Invalid time format: {e:?}. Please try again.");
                }
            }
        };
        let end = raptor.get_stop_idx(get_stop_from_user(&gtfs, "going")?.as_str());

        println!();
        println!(
            "Start: {} at time {}",
            raptor.get_stop(start).name,
            utils::get_time_str(start_time)
        );
        println!("End: {}", raptor.get_stop(end).name);
        println!();

        let mut journey = Vec::new();
        let query_start = std::time::Instant::now();
        for _ in 0..10 {
            journey = raptor.query(start, start_time, end)
        };
        let query_end = std::time::Instant::now();
        println!(
            "Query took {}μs.",
            (query_end - query_start).as_micros() / 10
        );

        raptor.print_journey(&journey);
    }
}
