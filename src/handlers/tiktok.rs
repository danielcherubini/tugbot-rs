use regex::Regex;
use reqwest::Result;
use serenity::{model::channel::Message, prelude::Context};

pub struct TikTok;

impl TikTok {
    pub async fn handler(ctx: &Context, msg: &Message) {
        match Self::fx_rewriter(&msg.content.to_owned()).await {
            None => (),
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

    async fn fx_rewriter(url: &str) -> Option<String> {
        let re = Regex::new(r"https://((www.)?tiktok.com)/.+").unwrap();
        let re_redirect = Regex::new(r"NEXT_REDIRECT;replace;(/post/\d+);").unwrap();
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
                    Err(_e) => None,
                },
            },
        }
    }

    async fn get_url(url: &str) -> Result<String> {
        let base_url = "https://offtiktok.com/api/by_url/".to_owned();
        let full_url = base_url + url;
        reqwest::get(full_url).await?.text().await
    }
}

#[cfg(test)]
mod tests {
    use super::TikTok;

    #[tokio::test]
    async fn rewrite() {
        match TikTok::fx_rewriter(
            "https://www.tiktok.com/@centralparkturtle/video/7412424505374674207",
        )
        .await
        {
            None => panic!(),
            Some(url) => assert_eq!(url, "https://offtiktok.com/post/6760",),
        }
    }
}
