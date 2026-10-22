//! Check a tutorial's format, print its commands as one script, and step them into real shells.

fn main() {
    std::process::exit(i32::from(tutorial_stepper::run()));
}
