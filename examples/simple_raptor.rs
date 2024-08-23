use raptor::raptor_query;

use dev_utils::get_example_scenario;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (network, start, start_time, end) = get_example_scenario();
    network.print_stats();

    let journey = raptor_query(&network, start, start_time, end);

    println!("{journey}");

    Ok(())
}