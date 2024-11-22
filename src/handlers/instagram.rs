use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

use crate::db::features;

pub async fn handler(ctx: &Context, msg: &Message) {
    if features::is_enabled("instagram".to_string()) {
        match fx_rewriter(&msg.content.to_owned()) {
            None => (),
            Some(fixed_message) => {
                if let Err(why) = msg.to_owned().suppress_embeds(ctx.to_owned()).await {
                    println!("Error supressing embeds {:?}", why);
                }

                println!("Suppressed Embed");
                if let Err(why) = msg.channel_id.say(ctx, fixed_message).await {
                    println!("Error Editing Message to Tweet {:?}", why);
                }

                println!("Posted Tweet");
            }
        }
    }
}

fn fx_rewriter(url: &str) -> Option<String> {
    let re = Regex::new(r"https://(www\.)?(instagram\.com)/.+").unwrap();

    match re.captures(url) {
        None => None,
        Some(caps) => match caps.get(0) {
            None => None,
            Some(full) => caps
                .get(2)
                .map(|short| full.as_str().replace(short.as_str(), "ddinstagram.com")),
        },
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn instagram_rewrite() {
        match super::fx_rewriter(
            "https://www.instagram.com/reel/DCkUQSry42v/?igsh=MXNrMDFwbTEzZnFvMg==",
        ) {
            None => panic!(),
            Some(url) => assert_eq!(
                url,
                "https://www.ddinstagram.com/reel/DCkUQSry42v/?igsh=MXNrMDFwbTEzZnFvMg==",
            ),
        }
    }
}
