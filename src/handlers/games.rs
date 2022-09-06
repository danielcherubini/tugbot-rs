use std::{sync::Arc, time::Duration};

use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    client::Context,
    model::{
        interactions::application_command::ApplicationCommandInteraction,
        prelude::{
            application_command::ApplicationCommandOptionType, MessageFlags, ReactionType, Role,
            RoleId,
        },
    },
    utils::MessageBuilder,
};

use super::handlers::HandlerResponse;
pub struct Games {
    pub games: Vec<Game>,
}

#[derive(Default, Clone, Debug)]
pub struct Game {
    pub name: String,
    pub role_name: String,
    pub role_id: u64,
    pub emoji: String,
}

impl Game {
    pub fn new(name: String, role_name: String, emoji: String) -> Self {
        Self {
            name,
            emoji,
            role_name,
            role_id: 0,
        }
    }
}

impl Games {
    pub fn new() -> Self {
        let mut g: Vec<Game> = Vec::new();
        g.push(Game::new(
            "Warzone".to_string(),
            "tag-warzone".to_string(),
            "👹".to_string(),
        ));
        g.push(Game::new(
            "Apex".to_string(),
            "tag-apex".to_string(),
            "👺".to_string(),
        ));
        g.push(Game::new(
            "Doom".to_string(),
            "tag-doom".to_string(),
            "👻".to_string(),
        ));

        Self { games: g }
    }

    pub fn find_role_from_emoji(games: Vec<Game>, emoji: ReactionType) -> Option<Game> {
        let mut found = false;
        let mut found_game: Game = Game::default();
        for game in games {
            if emoji == ReactionType::Unicode(game.emoji.to_owned()) {
                found_game = game.to_owned().clone();
                found = true;
            }
        }
        if found {
            Some(found_game)
        } else {
            None
        }
    }

    pub fn match_roles_and_games(games: Vec<Game>, roles: Vec<Role>) -> Vec<Game> {
        let mut g: Vec<Game> = Vec::new();
        for role in roles {
            for game in games.clone() {
                if role.name == game.role_name {
                    let new_game = Game {
                        name: game.name.to_owned(),
                        role_name: game.role_name.to_owned(),
                        role_id: *role.id.as_u64(),
                        emoji: game.emoji.to_owned(),
                    };
                    g.push(new_game);
                }
            }
        }
        return g;
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

    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        let games = Games::new().games;
        let mut add_option = CreateApplicationCommandOption::default();
        add_option
            .name("add")
            .description("what do you want to do")
            .kind(ApplicationCommandOptionType::String);

        let mut remove_option = CreateApplicationCommandOption::default();
        remove_option
            .name("remove")
            .description("what do you want to do")
            .kind(ApplicationCommandOptionType::String);

        for game in games {
            add_option.add_string_choice(&game.name, &game.role_name);
            remove_option.add_string_choice(&game.name, &game.role_name);
        }

        return command
            .name("games")
            .description("Which games to you play")
            .add_option(add_option)
            .add_option(remove_option);
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        // let member = command.member.as_ref().unwrap();
        let guild_id = command.guild_id.unwrap();
        let user = &command.user;
        let channel_id = command.channel_id.0;

        let options = &command.data.options;

        println!("{:#?}", options);

        if options.len() == 0 {
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
                    m.content(message)
                        .flags(MessageFlags::EPHEMERAL)
                        .reactions(reactions)
                })
                .await
                .unwrap();

            let http = Arc::clone(&ctx.http);
            // spawn(async move {

            tokio::time::sleep(Duration::from_secs(10)).await;
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

            msg.delete(&ctx.http).await.unwrap();
            // });

            HandlerResponse {
                content: "done".to_string(),
                ephemeral: true,
            }
        } else {
            for option in options {
                match option.name.as_str() {
                    "add" => {
                        println!("{}", option.name);
                    }
                    _ => {
                        println!("nothing");
                    }
                }
            }
            HandlerResponse {
                content: "done".to_string(),
                ephemeral: true,
            }
        }
    }
}
