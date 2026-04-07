use jlh_sys_design_playground::app::main::initialize;

fn main() {
    dotenvy::dotenv().unwrap();
    initialize(1920, 1080);
}
