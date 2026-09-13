mod rust01_print;
mod rust02_variables;
mod rust03_functions;
mod rust04_control_flow;
mod rust05_ownership;

use rust01_print::print_demo;
use rust02_variables::variables_demo;
use rust03_functions::functions_demo;
use rust04_control_flow::control_flow_demo;
use rust05_ownership::ownership_demo;

fn main() {
    println!("Hello, world!");

    print_demo();

    variables_demo();

    functions_demo();

    control_flow_demo();

    ownership_demo();
}
