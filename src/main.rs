mod rust01_print;
mod rust02_variables;
mod rust03_functions;
mod rust04_control_flow;
mod rust05_ownership;
mod rust06_structs_enums;
mod rust07_collections;
mod rust08_errors;
mod rust09_generics_traits;
mod rust10_lifetimes;
mod rust11_closures_iterators;
mod rust12_smart_pointers;
mod rust13_concurrency;
mod rust14_modules_tests;
mod rust15_macros;
mod rust16_async;
mod rust17_datetime;
mod rust18_text;
mod rust19_files_io;
mod rust20_serde;

use rust01_print::print_demo;
use rust02_variables::variables_demo;
use rust03_functions::functions_demo;
use rust04_control_flow::control_flow_demo;
use rust05_ownership::ownership_demo;
use rust06_structs_enums::structs_enums_demo;
use rust07_collections::collections_demo;
use rust08_errors::errors_demo;
use rust09_generics_traits::generics_traits_demo;
use rust10_lifetimes::lifetimes_demo;
use rust11_closures_iterators::closures_iterators_demo;
use rust12_smart_pointers::smart_pointers_demo;
use rust13_concurrency::concurrency_demo;
use rust14_modules_tests::modules_tests_demo;
use rust15_macros::macros_demo;
use rust16_async::async_demo;
use rust17_datetime::datetime_demo;
use rust18_text::text_demo;
use rust19_files_io::files_io_demo;
use rust20_serde::serde_demo;

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

    lifetimes_demo();

    closures_iterators_demo();

    smart_pointers_demo();

    concurrency_demo();

    modules_tests_demo();

    macros_demo();

    async_demo();

    datetime_demo();

    text_demo();

    files_io_demo();

    serde_demo();
}
