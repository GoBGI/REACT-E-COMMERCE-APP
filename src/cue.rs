use std::rc::Rc;

fn generate_commands(text: &str) -> Vec<Vec<Rc<str>>> {
    let iter = te