use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

use crate::features::Features;

pub struct TikTok;

impl TikTok {
    pub async fn handler(ctx: &Context, msg: &Message) {
        if Features::is_enabled("tiktok") {
            if let Some(fixed_message) = Self::fx_rewriter(&msg.content.to_owned()).await {
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

    async fn fx_rewriter(url: &str) -> Option<String> {
        let re = Regex::new(r"https://((www.)?tiktok.com)/.+").unwrap();
        let re_redirect = Regex::new(r"NEXT_REDIRECT;replace;(/post/\d+);").unwrap();

        // Get matches for the original tiktok url
        match re.captures(url) {
            None => None,
            Some(caps) => match caps.get(0) {
                None => None,
                Some(full) => match Self::get_url(full.as_str()).await {
                    Ok(off_tiktok) => match re_redirect.captures(&off_tiktok) {
                        Some(final_url_caps) => final_url_caps
                            .get(1)
                            .map(|f| "https://offtiktok.com".to_owned() + f.as_str()),
                        None => None,
                    },
                    Err(e) => {
                        println!("Error with get request for tiktok {:?}", e);
                        None
                    }
                },
            },
        }
    }

    async fn get_url(url: &str) -> reqwest::Result<String> {
        let full_url = url.replace("tiktok", "offtiktok");
        reqwest::get(full_url).await?.text().await
    }
}

//#[cfg(test)]
//mod tests {
//    use super::TikTok;
//
//    #[tokio::test]
//    async fn rewrite() {
//        match TikTok::fx_rewriter(
//            "https://www.tiktok.com/@centralparkturtle/video/7412424505374674207",
//        )
//        .await
//        {
//            None => panic!("No URL Found"),
//            Some(url) => assert_eq!(url, "https://offtiktok.com/post/15938",),
//        }
//    }
//}
