use crate::scenario::Scenario;

pub fn draw_scenario(scenario: &Scenario, verbose: bool) -> String {
    crate::scenario::visualization::draw(scenario, verbose)
}
