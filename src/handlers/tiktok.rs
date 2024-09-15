use anyhow::{anyhow, Result};
use regex::Regex;
use serenity::{model::channel::Message, prelude::Context};

pub struct TikTok;

impl TikTok {
    pub async fn handler(ctx: &Context, msg: &Message) {
        match Self::fx_rewriter(&msg.content.to_owned()).await {
            Err(e) => println!("Error in fx_rewriter {:?}", e),
            Ok(fixed_message) => {
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

    async fn fx_rewriter(url: &str) -> Result<String> {
        let re = Regex::new(r"https://((www.)?tiktok.com)/.+").unwrap();
        let re_redirect = Regex::new(r"NEXT_REDIRECT;replace;(/post/\d+);").unwrap();
        match re.captures(url) {
            None => Err(anyhow!("No Matches")),
            Some(caps) => match caps.get(0) {
                None => Err(anyhow!("No Match inner")),
                Some(full) => match Self::get_url(full.as_str()).await {
                    Ok(off_tiktok) => match re_redirect.captures(&off_tiktok) {
                        Some(final_url_caps) => Ok(final_url_caps
                            .get(1)
                            .map(|f| "https://offtiktok.com".to_owned() + f.as_str())
                            .unwrap()),
                        None => Err(anyhow!("No redirect regex match")),
                    },
                    Err(e) => Err(anyhow!("Error with request {:?}", e.to_string())),
                },
            },
        }
    }

    async fn get_url(url: &str) -> reqwest::Result<String> {
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
            Err(e) => panic!("{:?}", e),
            Ok(url) => assert_eq!(url, "https://offtiktok.com/post/6760",),
        }
    }
}
