use regex::Regex;
use serenity::{builder::EditMessage, model::channel::Message, prelude::Context};

use crate::features::Features;

pub struct TikTok;

impl TikTok {
    pub async fn handler(ctx: &Context, msg: &Message) {
        if Features::is_enabled("tiktok") {
            if let Some(fixed_message) = Self::fx_rewriter(&msg.content.to_owned()).await {
                if let Err(why) = msg
                    .clone()
                    .edit(&ctx.http, EditMessage::new().suppress_embeds(true))
                    .await
                {
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

#[cfg(test)]
mod tests {
    use regex::Regex;

    #[test]
    fn test_tiktok_url_regex_matches() {
        let re = Regex::new(r"https://((www.)?tiktok.com)/.+").unwrap();

        // Should match standard tiktok URLs
        assert!(re.is_match("https://www.tiktok.com/@user/video/123456"));
        assert!(re.is_match("https://tiktok.com/@user/video/123456"));

        // Should not match other domains
        assert!(!re.is_match("https://twitter.com/user"));
    }

    #[test]
    fn test_tiktok_redirect_regex() {
        let re_redirect = Regex::new(r"NEXT_REDIRECT;replace;(/post/\d+);").unwrap();

        // The regex captures /post/\d+ as a group, the parens in regex are for capturing
        let test_str = r#"something NEXT_REDIRECT;replace;/post/12345; more text"#;
        assert!(re_redirect.is_match(test_str));

        let caps = re_redirect.captures(test_str).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "/post/12345");
    }

    #[test]
    fn test_get_url_replacement() {
        // Test the URL replacement logic
        let original = "https://www.tiktok.com/@user/video/123";
        let expected = "https://www.offtiktok.com/@user/video/123";
        assert_eq!(original.replace("tiktok", "offtiktok"), expected);
    }

    // Integration test - commented out as it requires network access
    // Uncomment to test with real network requests
    /*
    #[tokio::test]
    async fn rewrite() {
        match TikTok::fx_rewriter(
            "https://www.tiktok.com/@centralparkturtle/video/7412424505374674207",
        )
        .await
        {
            None => panic!("No URL Found"),
            Some(url) => assert!(url.starts_with("https://offtiktok.com/post/")),
        }
    }
    */
}
