use crate::features::Features;
use crate::handlers::get_pool;
use serenity::{
    model::prelude::{Message, ReactionType},
    prelude::Context,
};

pub struct Teh;
impl Teh {
    pub async fn handler(ctx: &Context, msg: &Message) {
        let pool = get_pool(ctx).await;
        if Features::is_enabled(&pool, "teh") && msg.content.to_lowercase().contains("teh") {
            // React with "🇹"
            if let Err(why) = msg.react(ctx, ReactionType::Unicode("🇹".to_string())).await {
                eprintln!("Error reacting with emoji T: {:?}", why);
            }
            // React with "🇪"
            if let Err(why) = msg.react(ctx, ReactionType::Unicode("🇪".to_string())).await {
                eprintln!("Error reacting with emoji E: {:?}", why);
            }
            // React with "🇭"
            if let Err(why) = msg.react(ctx, ReactionType::Unicode("🇭".to_string())).await {
                eprintln!("Error reacting with emoji H: {:?}", why);
            }
        }
    }
}
