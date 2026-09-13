mod rust01_print;
mod rust02_variables;
mod rust03_functions;
mod rust04_control_flow;
mod rust05_ownership;
mod rust06_structs_enums;
mod rust07_collections;
mod rust08_errors;
mod rust09_generics_traits;

use rust01_print::print_demo;
use rust02_variables::variables_demo;
use rust03_functions::functions_demo;
use rust04_control_flow::control_flow_demo;
use rust05_ownership::ownership_demo;
use rust06_structs_enums::structs_enums_demo;
use rust07_collections::collections_demo;
use rust08_errors::errors_demo;
use rust09_generics_traits::generics_traits_demo;

fn main() {
    println!("Hello, world!");

    print_demo();

    variables_demo();

    functions_demo();

    control_flow_demo();

    ownership_demo();

    structs_enums_demo();

    collections_demo();

    errors_demo();

    generics_traits_demo();
}
