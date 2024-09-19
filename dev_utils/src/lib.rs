use std::ffi::OsStr;
use chrono::NaiveDate;
use gtfs_structures::{Error, Gtfs, GtfsReader};
use raptor::network::{StopIndex, Timestamp};
use raptor::{utils, Network};
use std::fs;
use std::fs::{DirEntry, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use rayon::{ThreadPool, ThreadPoolBuildError};

// Create a rayon thread pool with the given number of threads.
pub fn create_pool(num_threads: usize) -> Result<ThreadPool, ThreadPoolBuildError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
}

// Common example data for the examples and benchmarks.

// Returns if any file in the directory tree matches the condition.
fn visit_dirs(dir: &Path, cb: &mut impl FnMut(&DirEntry) -> bool, ignore: &[&OsStr]) -> io::Result<bool> {
    if dir.file_name().map(|s| ignore.contains(&s)).unwrap_or(false) {
        return Ok(false);
    }
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if cb(&entry) {
                return Ok(true);
            }
            let path = entry.path();
            if path.is_dir() {
                if visit_dirs(&path, cb, ignore)? {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn find_dev_utils_folder() -> Result<PathBuf, io::Error> {
    static DEV_UTILS_PATH: OnceLock<PathBuf> = OnceLock::new();

    Ok(DEV_UTILS_PATH.get_or_init(|| {
        let current_dir = std::env::current_dir().unwrap();
        let mut dev_utils_path = None;

        visit_dirs(&current_dir.join("../"), &mut |entry| {
            let is_dev_utils = entry.path().ends_with("raptor-rs/dev_utils");
            if is_dev_utils {
                dev_utils_path = Some(entry.path());
            }
            is_dev_utils
        }, &[".git".as_ref(), ".idea".as_ref()]).unwrap();

        dev_utils_path.unwrap()
    }).to_owned())
}

pub fn load_example_gtfs() -> Result<Gtfs, Error> {
    let dev_utils_dir = find_dev_utils_folder()?;
    let gtfs_dir = dev_utils_dir.join("gtfs/melbourne.zip");
    GtfsReader::default().read_shapes(false).read_from_path(gtfs_dir.to_str().unwrap())
}

pub fn find_example_patronage_data() -> Result<File, io::Error> {
    let dev_utils_dir = find_dev_utils_folder()?;
    let data_path = dev_utils_dir.join("data/melbourne.parquet");
    File::open(data_path)
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

