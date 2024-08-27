use raptor::csa_query;

use dev_utils::get_example_scenario;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut network, start, start_time, end) = get_example_scenario();
    network.build_connections();
    network.print_stats();

    let journey = csa_query(&network, start, start_time, end);

    println!("{journey}");

    Ok(())
}