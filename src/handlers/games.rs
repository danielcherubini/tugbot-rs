use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    client::Context,
    model::{
        interactions::application_command::ApplicationCommandInteraction,
        prelude::{
            application_command::{
                ApplicationCommandInteractionDataOption, ApplicationCommandOptionType,
            },
            Role, RoleId,
        },
    },
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
}

impl Game {
    pub fn new(name: String, role_name: String) -> Self {
        Self {
            name,
            role_name,
            role_id: 0,
        }
    }
}

impl Games {
    pub fn new() -> Self {
        let mut g: Vec<Game> = Vec::new();
        g.push(Game::new("Warzone".to_string(), "tag-warzone".to_string()));
        g.push(Game::new("Apex".to_string(), "tag-apex".to_string()));
        g.push(Game::new("Doom".to_string(), "tag-doom".to_string()));

        Self { games: g }
    }

    fn find_game_from_role_name(games: Vec<Game>, role_name: String) -> Option<Game> {
        let unescaped_role_name = snailquote::unescape(role_name.as_str()).unwrap();
        let mut found_game = false;
        let mut game = Game::default();
        for g in games.clone() {
            if g.role_name == unescaped_role_name {
                game = g;
                found_game = true;
                break;
            }
        }

        println!("found? {} - {:#?}", found_game, game);
        if found_game {
            Some(game)
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
                    };
                    g.push(new_game);
                }
            }
        }
        return g;
    }

    fn does_user_have_role(user_roles: Vec<RoleId>, role_id: RoleId) -> Option<RoleId> {
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

    async fn add_or_remove_game(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        options: &Vec<ApplicationCommandInteractionDataOption>,
    ) -> HandlerResponse {
        let mut handler_response = HandlerResponse::default();
        handler_response.ephemeral = true;

        let guild_id = command.guild_id.unwrap();
        let roles = &ctx.http.get_guild_roles(guild_id.0).await.unwrap();
        let games = Games::match_roles_and_games(Games::new().games, roles.to_owned());
        let user = &command.user;

        let mut mem = ctx
            .http
            .get_member(*guild_id.as_u64(), *user.id.as_u64())
            .await
            .unwrap();

        for option in options {
            match &option.value {
                Some(value) => match option.name.as_str() {
                    "add" => {
                        match Self::find_game_from_role_name(games.to_owned(), value.to_string()) {
                            Some(game) => {
                                mem.add_role(&ctx, game.role_id).await.unwrap();
                                handler_response.content = "Added Role".to_string();
                            }
                            None => {
                                println!("couldn't find game from role name");
                                handler_response.content =
                                    "You didnt fill out the request correctly".to_string();
                            }
                        };
                    }
                    "remove" => {
                        match Self::find_game_from_role_name(games.to_owned(), value.to_string()) {
                            Some(game) => {
                                match Self::does_user_have_role(
                                    mem.roles.to_owned(),
                                    RoleId(game.role_id),
                                ) {
                                    Some(role) => {
                                        mem.remove_role(&ctx, role).await.unwrap();
                                        handler_response.content = "Removed Role".to_string();
                                    }
                                    None => {
                                        println!("user didn't have role");
                                        handler_response.content =
                                            "You didnt fill out the request correctly".to_string();
                                    }
                                }
                            }
                            None => {
                                println!("couldn't find game from role name");
                                handler_response.content =
                                    "You didnt fill out the request correctly".to_string();
                            }
                        };
                    }
                    _ => {
                        println!("nothing");
                        handler_response.content =
                            "You didnt fill out the request correctly".to_string();
                    }
                },
                None => {
                    println!("nothing");
                    handler_response.content =
                        "You didnt fill out the request correctly".to_string();
                }
            }
        }

        return handler_response;
    }

    async fn no_options_passed() -> HandlerResponse {
        HandlerResponse {
            content: "No Response".to_string(),
            ephemeral: true,
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
        let options = &command.data.options;

        if options.len() == 0 {
            return Self::no_options_passed().await;
        } else {
            return Self::add_or_remove_game(ctx, command, options).await;
        }
    }
}
