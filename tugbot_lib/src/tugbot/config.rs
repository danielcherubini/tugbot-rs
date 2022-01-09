extern crate dotenv;
use dotenv::dotenv;
use std::env;

pub struct Config {
    pub token: String,
    pub application_id: u64,
}

impl Config {
    pub fn get_config() -> Config {
        dotenv().ok();
        let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

        let application_id: u64 = env::var("APPLICATION_ID")
            .expect("Expected an application id in the environment")
            .parse()
            .expect("application id is not a valid id");

        return Config {
            token,
            application_id,
        };
    }
}
