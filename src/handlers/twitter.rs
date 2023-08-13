use regex::Regex;

use serenity::{model::channel::Message, prelude::Context};

pub struct Twitter;

impl Twitter {
    pub async fn fx_twitter_handler(ctx: Context, mut msg: Message) {
        let re = Regex::new(r"https://(twitter.com|x.com)/.+/status/\d+").unwrap();

        match re.captures(&msg.content.to_owned()) {
            None => return,
            Some(caps) => {
                if let Err(why) = msg.suppress_embeds(ctx.to_owned()).await {
                    println!("Error supressing embeds {:?}", why);
                }

                println!("Suppressed Embed");
                let current_capture = &caps[0];
                if let Err(why) = msg
                    .channel_id
                    .say(ctx, current_capture.replace(&caps[1], "vxtwitter.com"))
                    .await
                {
                    println!("Error Editing Message to Tweet {:?}", why);
                }

                println!("Posted Tweet");
            }
        }
    }
}
