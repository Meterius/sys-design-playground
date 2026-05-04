use geo_app::app::main::initialize;

fn main() {
    dotenvy::dotenv().unwrap();
    initialize(1920, 1080);
}
