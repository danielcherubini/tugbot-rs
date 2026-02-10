use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

use crate::features::Features;

pub struct Twitter;

impl Twitter {
    pub async fn handler(ctx: &Context, msg: &Message) {
        if Features::is_enabled("twitter") {
            match Self::fx_rewriter(&msg.content.to_owned()) {
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
        let re = Regex::new(r"https://(twitter.com|x.com)/.+/status/\d+").unwrap();

        match re.captures(url) {
            None => None,
            Some(caps) => match caps.get(0) {
                None => None,
                Some(full) => caps
                    .get(1)
                    .map(|short| full.as_str().replace(short.as_str(), "girlcockx.com")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Twitter;

    #[test]
    fn twitter_rewrite() {
        match Twitter::fx_rewriter("https://twitter.com/davidbcooper/status/1684840110259404802") {
            None => panic!(),
            Some(url) => assert_eq!(
                url,
                "https://girlcockx.com/davidbcooper/status/1684840110259404802",
            ),
        }
    }

    #[test]
    fn x_rewrite() {
        match Twitter::fx_rewriter("https://x.com/davidbcooper/status/1684840110259404802") {
            None => panic!("Expected URL to be rewritten"),
            Some(url) => assert_eq!(
                url,
                "https://girlcockx.com/davidbcooper/status/1684840110259404802",
            ),
        }
    }
}
