use std::{sync::Arc, time::Duration};

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        interactions::application_command::ApplicationCommandInteraction,
        prelude::{MessageFlags, ReactionType, RoleId},
    },
    utils::MessageBuilder,
};

use super::{games::Games, handlers::HandlerResponse};

pub struct Roles;

impl Roles {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command.name("games").description("Which games to you play");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        // let member = command.member.as_ref().unwrap();
        let guild_id = command.guild_id.unwrap();
        let user = &command.user;
        // let prefix = &command.data.name;
        let channel_id = command.channel_id.0;

        let roles = &ctx.http.get_guild_roles(guild_id.0).await.unwrap();
        let games = Games::match_roles_and_games(Games::new().games, roles.to_owned());
        let channel = &ctx.http.get_channel(channel_id).await.unwrap();
        let mut message = MessageBuilder::new();
        let mut reactions: Vec<ReactionType> = Vec::new();

        for game in games.clone() {
            message.push_line(format!(
                "{} - {}: @{}",
                game.emoji,
                game.name.to_owned(),
                game.role_name
            ));
            reactions.push(ReactionType::Unicode(game.emoji.to_owned()));
        }
        message.build();

        let msg = channel
            .id()
            .send_message(&ctx.http, |m| {
                m.content(message).flags(MessageFlags::EPHEMERAL)
            })
            .await
            .unwrap();
        for emoji in reactions {
            let _ = msg.react(ctx, emoji).await;
        }

        let http = Arc::clone(&ctx.http);
        // spawn(async move {

        tokio::time::sleep(Duration::from_secs(10)).await;
        msg.delete(&ctx.http).await.unwrap();
        let mut mem = ctx
            .http
            .get_member(*guild_id.as_u64(), *user.id.as_u64())
            .await
            .unwrap();

        let m = http
            .get_message(channel_id, *msg.id.as_u64())
            .await
            .unwrap();

        let mut roles_to_add: Vec<RoleId> = Vec::new();
        let mut roles_to_remove: Vec<RoleId> = Vec::new();
        for reaction in m.reactions {
            // Iterate here through the reactions to see which roles the user has selected
            if reaction.count > 1 {
                match Games::find_role_from_emoji(games.clone(), reaction.reaction_type) {
                    Some(role) => {
                        let role_id = RoleId(role.role_id);
                        let user_roles = mem.roles.to_owned();
                        match Self::does_user_have_role(user_roles, role_id) {
                            Some(_) => {
                                roles_to_remove.push(role_id);
                            }
                            None => {
                                roles_to_add.push(role_id);
                            }
                        }
                    }
                    None => {}
                };
            }
        }
        roles_to_add.dedup();
        roles_to_remove.dedup();

        println!("adding {:#?}", roles_to_add);
        if roles_to_add.len() > 0 {
            mem.add_roles(&http, &roles_to_add).await.unwrap();
        }
        println!("removing {:#?}", roles_to_remove);
        if roles_to_remove.len() > 0 {
            mem.remove_roles(&http, &roles_to_remove).await.unwrap();
        }

        // });

        return HandlerResponse {
            content: "done".to_string(),
            ephemeral: true,
        };
    }

    pub fn does_user_have_role(user_roles: Vec<RoleId>, role_id: RoleId) -> Option<RoleId> {
        let mut found_role = false;
        let mut role = RoleId::default();
        for user_role in user_roles {
            if user_role == role_id {
                found_role = true;
                role = role_id;
            }
        }

        if found_role {
            Some(role)
        } else {
            None
        }
    }
}
