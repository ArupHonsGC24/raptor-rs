use raptor::network::PathfindingCost;
use raptor::mc_raptor_query;

use dev_utils::get_example_scenario;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (network, start, start_time, end) = get_example_scenario();
    network.print_stats();

    // Random pathfinding costs.
    let mut costs = vec![0 as PathfindingCost; network.stop_times.len()];
    for cost in costs.iter_mut() {
        *cost = fastrand::f32() as PathfindingCost;
    }

    let journey = mc_raptor_query(&network, start, start_time, end, &costs);

    println!("{journey}");

    Ok(())
}