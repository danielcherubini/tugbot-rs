use std::{sync::Arc, time::Duration};

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        channel::ReactionType,
        guild::Role,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
    },
};
use tokio::{task::spawn, time::sleep};

use super::handlers::HandlerResponse;
pub struct ElkMen;

async fn find_elkmen_role(ctx: &Context, guild_id: u64) -> Option<Role> {
    match ctx.http.get_guild_roles(guild_id).await {
        Err(_why) => None,
        Ok(roles) => {
            for role in roles {
                if role.name == "elk-men" {
                    return Some(role);
                }
            }
            None
        }
    }
}

// fn get_reactions() -> MessageReaction {
//     let reaction: MessageReaction = MessageReaction {
//         count: 1,
//         me: true,
//         reaction_type: ReactionType::Unicode("thumbsup".to_string()),
//     };
//     return reaction;
// }

impl ElkMen {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("elk-invite")
            .description("Invite to the Elk Men")
            .create_option(|option| {
                option
                    .name("user")
                    .description("The user to lookup")
                    .kind(ApplicationCommandOptionType::User)
                    .required(true)
            });
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");

        let channel_id = command.channel_id.0;

        if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
            match command.guild_id {
                None => {
                    return HandlerResponse {
                        content: "no member".to_string(),
                        ephemeral: false,
                    }
                }
                Some(guild_id) => match find_elkmen_role(&ctx, *guild_id.as_u64()).await {
                    None => {
                        return HandlerResponse {
                            content: "couldn't find elk-men role".to_string(),
                            ephemeral: false,
                        }
                    }
                    Some(role) => {
                        let mut mem = ctx
                            .http
                            .get_member(*guild_id.as_u64(), *user.id.as_u64())
                            .await
                            .unwrap();

                        let message = format!(
                            "Should we add {} to the {}?, You have 10 mins to vote",
                            mem.to_string(),
                            role.to_string()
                        );
                        let channel = &ctx.http.get_channel(channel_id).await.unwrap();
                        let msg = channel
                            .id()
                            .send_message(&ctx.http, |m| m.content(message))
                            .await
                            .unwrap();
                        let _ = msg.react(ctx, '👍').await;
                        let _ = msg.react(ctx, '👎').await;
                        let http = Arc::clone(&ctx.http);
                        spawn(async move {
                            sleep(Duration::from_secs(600)).await;
                            let m = http
                                .get_message(channel_id, *msg.id.as_u64())
                                .await
                                .unwrap();
                            let mut yay = 0;
                            let mut nay = 0;
                            for reaction in m.reactions {
                                if reaction.reaction_type == ReactionType::from('👍') {
                                    yay = reaction.count - 1;
                                }
                                if reaction.reaction_type == ReactionType::from('👎') {
                                    nay = reaction.count - 1;
                                }
                            }

                            let c = http.get_channel(channel_id).await.unwrap();
                            if yay > nay {
                                mem.add_role(&http, role.id).await.unwrap();
                                let welcome_message = format!(
                                    "Welcome brother {} to the {}!",
                                    mem.to_string(),
                                    role.to_string()
                                );

                                c.id()
                                    .send_message(&http, |m| m.content(welcome_message))
                                    .await
                                    .unwrap();
                            } else {
                                let fail_message = format!(
                                    "Sorry {}, but {} didn't RISE to the occasion!",
                                    role.to_string(),
                                    mem.to_string()
                                );

                                c.id()
                                    .send_message(&http, |m| m.content(fail_message))
                                    .await
                                    .unwrap();
                            }
                        });

                        return HandlerResponse {
                            content: "Asking".to_string(),
                            ephemeral: true,
                        };
                    }
                },
            }
        } else {
            return HandlerResponse {
                content: "Please provide a valid user".to_string(),
                ephemeral: false,
            };
        };
    }
}
