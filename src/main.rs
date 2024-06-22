use std::io::{stdout, Write};

use chrono::NaiveDate;
use gtfs_structures::GtfsReader;

// A bit unorthodox, perhaps, but it lets me make a binary and a library without duplication.
include!("lib.rs");

use network::StopIndex;

pub fn get_stop_from_user(network: &Network, prompt: &str) -> Result<StopIndex, std::io::Error> {
    loop {
        print!("Where are you {prompt}? ");
        stdout().flush()?;
        let mut stop_name = String::new();
        std::io::stdin().read_line(&mut stop_name)?;
        if let Some(stop) = network.get_stop_idx_from_name(stop_name.trim()) {
            return Ok(stop);
        }
        println!("Stop not found. Please try again.");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gtfs = GtfsReader::default().read_shapes(false).read("../gtfs/2/google_transit.zip")?;

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
        // let mut date_str = String::from("2024/");
        print!("When are you travelling (in 2024)? (DD/MM): ");
        //stdout().flush()?;
        //std::io::stdin().read_line(&mut date_str)?;
        let date_str = String::from("2024/10/5");
        match NaiveDate::parse_from_str(date_str.trim(), "%Y/%d/%m") {
            Ok(parsed_date) => break parsed_date,
            Err(e) => {
                println!("Invalid date format: {e:?}. Please try again.");
            }
        }
    };

    println!();

    let default_transfer_time = 0 * 60;
    let mut network = Network::new(&gtfs, journey_date, default_transfer_time);
    // Hardcode extra time at Flinders Street Station.
    network.set_transfer_time_for_stop("19854", 0 * 60);
    network.build_connections();

    loop {
        //let start = get_stop_from_user(&network, "starting")?;
        let start = network.get_stop_idx_from_name("cheltenham").unwrap();
        let start_time = loop {
            //let mut time_str = String::new();
            print!("What time are you starting? (HH:MM): ");
            stdout().flush()?;
            //std::io::stdin().read_line(&mut time_str)?;
            let time_str = String::from("08:30");
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
        //let end = get_stop_from_user(&network, "going")?;
        let end = network.get_stop_idx_from_name("greensborough").unwrap();
        
        println!();
        println!(
            "Start: {} at time {}",
            network.get_stop(start as usize).name,
            utils::get_time_str(start_time)
        );
        println!("End: {}", network.get_stop(end as usize).name);
        println!();

        let mut journey = Journey::from(Vec::new(), &network);
        let query_start = std::time::Instant::now();
        for _ in 0..10 {
            journey = raptor_query(&network, start, start_time, end);
        }
        println!("RAPTOR:");
        println!("Query took {:?}", query_start.elapsed() / 10);
        println!("{journey}");
        let query_start = std::time::Instant::now();
        for _ in 0..10 {
            journey = csa_query(&network, start, start_time, end);
        }
        println!("CSA:");
        println!("Query took {:?}", query_start.elapsed() / 10);
        println!("{journey}");
        
        break;
    }
    
    Ok(())
}
