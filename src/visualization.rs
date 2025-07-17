use crate::scenario::Scenario;

pub trait Draw<T> {
    fn draw(&self, item: &T) -> String;
}

pub(crate) struct DiGraphDrawer;

pub fn draw_scenario(scenario: &Scenario) -> String {
    DiGraphDrawer.draw(scenario)
}
