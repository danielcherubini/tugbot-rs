extern crate dotenv;
use dotenv::dotenv;
use serenity::prelude::GatewayIntents;
use std::env;

pub struct Config {
    pub token: String,
    pub application_id: u64,
    pub db_url: String,
    pub intents: GatewayIntents,
    /// Discord user ID that bypasses mention-feature cooldowns.
    /// Kept for backward compatibility; prefer COOLDOWN_EXEMPT_USER_IDS.
    pub admin_user_id: u64,
    /// Discord user IDs that bypass mention-feature cooldowns.
    pub cooldown_exempt_user_ids: Vec<u64>,
    /// Discord user IDs that get the slower cooldown (2h instead of 5m)
    /// and trigger the auto-gulag on bot mention.
    pub slow_user_ids: Vec<u64>,
}

impl Config {
    pub fn get_config() -> Config {
        dotenv().ok();
        let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
        let db_url = env::var("DATABASE_URL").expect("Expected a DB URL in the environment");

        let application_id: u64 = env::var("APPLICATION_ID")
            .expect("Expected an application id in the environment")
            .parse()
            .expect("application id is not a valid id");

        // ADMIN_USER_ID — bypasses mention cooldowns. Default: 0 (disabled).
        let admin_user_id: u64 = env::var("ADMIN_USER_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        // COOLDOWN_EXEMPT_USER_IDS — comma-separated Discord user IDs that
        // bypass the mention cooldown entirely. Default: empty.
        let mut cooldown_exempt_user_ids: Vec<u64> = env::var("COOLDOWN_EXEMPT_USER_IDS")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter_map(|part| part.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();

        // ADMIN_USER_ID kept for backward compatibility — include it in the
        // exemption list if it's set to a non-zero value.
        if admin_user_id != 0 && !cooldown_exempt_user_ids.contains(&admin_user_id) {
            cooldown_exempt_user_ids.push(admin_user_id);
        }

        // SLOW_USER_IDS — comma-separated Discord user IDs that get the slow
        // cooldown AND trigger the auto-gulag on mention. Default: empty.
        let slow_user_ids: Vec<u64> = env::var("SLOW_USER_IDS")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter_map(|part| part.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();

        let intents = GatewayIntents::privileged()
            .union(GatewayIntents::MESSAGE_CONTENT)
            .union(GatewayIntents::GUILD_MESSAGES)
            .union(GatewayIntents::GUILD_MESSAGE_REACTIONS)
            .union(GatewayIntents::GUILD_MESSAGE_POLLS);
        Config {
            db_url,
            token,
            application_id,
            intents,
            admin_user_id,
            cooldown_exempt_user_ids,
            slow_user_ids,
        }
    }
}
