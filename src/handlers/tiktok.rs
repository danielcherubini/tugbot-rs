use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

pub struct TikTok;

impl TikTok {
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

                println!("Posted Tickeytackey");
            }
        }
    }

    fn fx_rewriter(url: &str) -> Option<String> {
        let re = Regex::new(r"https://(tiktok.com)/.+").unwrap();

        match re.captures(&url) {
            None => None,
            Some(caps) => match caps.get(0) {
                None => None,
                Some(full) => match caps.get(1) {
                    None => None,
                    Some(short) => Some(full.as_str().replace(short.as_str(), "vm.offtiktok.com")),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TikTok;

    #[test]
    fn rewrite() {
        match TikTok::fx_rewriter("https://tiktok.com/t/7m4Kxl") {
            None => assert!(false),
            Some(url) => assert_eq!(url, "https://vm.offtiktok.com/t/7m4Kxl",),
        }
    }
}
