use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

use crate::features::Features;

pub struct Bsky;

impl Bsky {
    pub async fn handler(ctx: &Context, msg: &Message) {
        if Features::is_enabled("bsky") {
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
            None => panic!(),
            Some(url) => assert_eq!(
                url,
                "https://bsyy.app/profile/radleybalko.bsky.social/post/3lb5nsfya6s2o",
            ),
        }
    }

    #[test]
    fn bsky_no_match() {
        // Non-bsky URL should return None
        let result = Bsky::fx_rewriter("https://twitter.com/someone/status/123");
        assert!(result.is_none());
    }

    #[test]
    fn bsky_empty_string() {
        let result = Bsky::fx_rewriter("");
        assert!(result.is_none());
    }

    #[test]
    fn bsky_partial_url() {
        // Regex requires at least some path after domain (/.+)
        let result = Bsky::fx_rewriter("https://bsky.app/");
        // This will be None because regex requires /.+ (something after /)
        assert!(result.is_none());
    }

    #[test]
    fn bsky_with_query_params() {
        let result = Bsky::fx_rewriter("https://bsky.app/profile/user?ref=share");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "https://bsyy.app/profile/user?ref=share");
    }
}
