use gtfs_structures::{Error, Gtfs, GtfsReader};

pub fn load_example_gtfs() -> Result<Gtfs, Error> {
    let current_dir = std::env::current_dir()?;
    let gtfs_dir = if current_dir.ends_with("raptor-rs") {
        "examples/gtfs/melbourne.zip"
    } else {
        "raptor-rs/examples/gtfs/melbourne.zip"
    };

    GtfsReader::default().read_shapes(false).read(gtfs_dir)
}