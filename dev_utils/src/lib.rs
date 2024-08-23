use chrono::NaiveDate;
use gtfs_structures::{Error, Gtfs, GtfsReader};
use raptor::{utils, Network};
use raptor::network::{StopIndex, Timestamp};

// Common example data for the examples and benchmarks.

pub fn load_example_gtfs() -> Result<Gtfs, Error> {
    let current_dir = std::env::current_dir()?;
    let gtfs_dir = if current_dir.ends_with("raptor-rs") {
        "dev_utils/gtfs/melbourne.zip"
    } else {
        "raptor-rs/dev_utils/gtfs/melbourne.zip"
    };

    GtfsReader::default().read_shapes(false).read(gtfs_dir)
}

pub fn get_example_date() -> NaiveDate {
    const { utils::const_unwrap(NaiveDate::from_ymd_opt(2024, 5, 10)) }
}

pub fn get_example_transfer_time() -> Timestamp {
    3 * 60 // 3 minutes transfer time.
}

pub fn build_example_network(gtfs: &Gtfs) -> Network {
    let date = get_example_date();
    let transfer_time = get_example_transfer_time();
    Network::new(&gtfs, date, transfer_time)
}

pub fn get_example_start_time() -> Timestamp {
    utils::parse_time("08:30:00").unwrap()
}

pub fn get_example_start_stop_idx(network: &Network) -> StopIndex {
    network.get_stop_idx_from_name("Cheltenham").unwrap()
}

pub fn get_example_end_stop_idx(network: &Network) -> StopIndex {
    network.get_stop_idx_from_name("Greensborough").unwrap()
}

pub fn get_example_scenario() -> (Network, StopIndex, Timestamp, StopIndex) {
    let gtfs = load_example_gtfs().unwrap();
    let network = build_example_network(&gtfs);
    let start = get_example_start_stop_idx(&network);
    let start_time = get_example_start_time();
    let end = get_example_end_stop_idx(&network);
    (network, start, start_time, end)
}

