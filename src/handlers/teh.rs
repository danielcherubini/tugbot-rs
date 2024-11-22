use crate::db::features;
use serenity::{
    model::prelude::{Message, ReactionType},
    prelude::Context,
};

pub async fn handler(ctx: &Context, msg: &Message) {
    if features::is_enabled("teh".to_string()) && msg.content.to_lowercase().contains("teh") {
        // React with "🇹"
        if let Err(why) = msg.react(ctx, ReactionType::Unicode("🇹".to_string())).await {
            println!("Error reacting with emoji T: {:?}", why);
        }
        // React with "🇪"
        if let Err(why) = msg.react(ctx, ReactionType::Unicode("🇪".to_string())).await {
            println!("Error reacting with emoji E: {:?}", why);
        }
        // React with "🇭"
        if let Err(why) = msg.react(ctx, ReactionType::Unicode("🇭".to_string())).await {
            println!("Error reacting with emoji H: {:?}", why);
        }
    }
}
