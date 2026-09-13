mod rust01_print;
mod rust02_variables;
mod rust03_functions;
mod rust04_control_flow;

use rust01_print::print_demo;
use rust02_variables::variables_demo;
use rust03_functions::functions_demo;
use rust04_control_flow::control_flow_demo;

fn main() {
    println!("Hello, world!");

    print_demo();

    variables_demo();

    functions_demo();

    control_flow_demo();
}
