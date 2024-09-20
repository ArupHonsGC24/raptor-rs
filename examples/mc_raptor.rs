use std::iter::repeat_with;
use raptor::network::PathfindingCost;
use raptor::mc_raptor_query;

use dev_utils::get_example_scenario;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (network, start, start_time, end) = get_example_scenario();
    network.print_stats();

    // Random pathfinding costs.
    fastrand::seed(7);
    let costs: Vec<_> = repeat_with(|| fastrand::f32() as PathfindingCost).take(network.stop_times.len()).collect();
    let path_preferences = raptor::journey::JourneyPreferences::default();
    let journey = mc_raptor_query(&network, start, start_time, end, &costs, &path_preferences).expect("Failed to find journey.");

    println!("{journey}");

    Ok(())
}