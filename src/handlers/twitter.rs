use regex::Regex;

use serenity::{model::channel::Message, prelude::Context};

pub struct Twitter;

impl Twitter {
    pub async fn handler(ctx: &Context, msg: &Message) {
        match Self::fx_rewriter(&msg.content.to_owned()) {
            None => return,
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

    fn fx_rewriter(url: &str) -> Option<String> {
        let re = Regex::new(r"https://(twitter.com|x.com)/.+/status/\d+").unwrap();

        match re.captures(&url) {
            None => None,
            Some(caps) => match caps.get(0) {
                None => None,
                Some(full) => match caps.get(1) {
                    None => None,
                    Some(short) => Some(full.as_str().replace(short.as_str(), "vxtwitter.com")),
                },
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
            None => assert!(false),
            Some(url) => assert_eq!(
                url,
                "https://vxtwitter.com/davidbcooper/status/1684840110259404802",
            ),
        }
    }

    #[test]
    fn x_rewrite() {
        match Twitter::fx_rewriter("https://x.com/davidbcooper/status/1684840110259404802") {
            None => assert!(false),
            Some(url) => assert_eq!(
                url,
                "https://vxtwitter.com/davidbcooper/status/1684840110259404802",
            ),
        }
    }
}
