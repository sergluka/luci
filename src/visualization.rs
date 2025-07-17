use crate::scenario::Scenario;

pub fn draw_scenario(scenario: &Scenario) -> String {
    crate::scenario::visualization::draw(scenario)
}
