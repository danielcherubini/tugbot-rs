use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

pub struct Bsky;

impl Bsky {
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
        let re = Regex::new(r"https://(bsky.app)/.+").unwrap();

        match re.captures(url) {
            None => None,
            Some(caps) => match caps.get(0) {
                None => None,
                Some(full) => caps
                    .get(1)
                    .map(|short| full.as_str().replace(short.as_str(), "bsyy.app")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bsky;

    #[test]
    fn bsky_rewrite() {
        match Bsky::fx_rewriter(
            "https://bsky.app/profile/radleybalko.bsky.social/post/3lb5nsfya6s2o",
        ) {
            None => assert!(false),
            Some(url) => assert_eq!(
                url,
                "https://bsyy.app/profile/radleybalko.bsky.social/post/3lb5nsfya6s2o",
            ),
        }
    }
}
