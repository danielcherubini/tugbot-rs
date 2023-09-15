use serenity::{
    builder::{
        CreateActionRow, CreateApplicationCommand, CreateComponents, CreateSelectMenu,
        CreateSelectMenuOption,
    },
    client::Context,
    http::CacheHttp,
    model::application::interaction::application_command::ApplicationCommandInteraction,
};

use super::handlers::HandlerResponse;

#[derive(Default, Clone, Debug)]
pub struct Game {
    pub name: String,
    pub role_id: u64,
}

impl Game {
    pub fn new(name: String, role_id: u64) -> Self {
        Self { name, role_id }
    }

    pub fn menu_option(&self) -> CreateSelectMenuOption {
        let mut opt = CreateSelectMenuOption::default();
        opt.label(self.name.to_string());
        opt.value(self.role_id);
        opt
    }
}

pub struct GameHandler {
    pub games: Vec<Game>,
}

impl GameHandler {
    pub async fn new(ctx: &Context, guild_id: u64) -> Self {
        let mut g: Vec<Game> = Vec::new();
        // Get Roles from Discord and iterate over them to create the game array
        let roles = &ctx.http.get_guild_roles(guild_id).await.unwrap();
        for role in roles {
            if role.name.starts_with("tag-") {
                g.push(Game::new(role.name.to_string(), *role.id.as_u64()));
            }
        }

        Self { games: g }
    }

    fn menu_options(&self) -> Vec<CreateSelectMenuOption> {
        let mut options = vec![];
        for color in self.games.to_owned() {
            options.push(color.menu_option())
        }
        options
    }

    fn select_menu(&self) -> CreateSelectMenu {
        let mut menu = CreateSelectMenu::default();
        menu.custom_id("game_select");
        menu.placeholder("Select your game");
        menu.options(|opt| opt.set_options(self.menu_options()));
        menu
    }

    fn action_row(&self) -> CreateActionRow {
        let mut ar = CreateActionRow::default();
        ar.add_select_menu(self.select_menu());
        ar
    }

    pub async fn add_or_remove_game_role(ctx: &Context, guild_id: u64, user_id: u64, role_id: u64) {
        let mut member = ctx.http().get_member(guild_id, user_id).await.unwrap();

        if member.user.has_role(&ctx, guild_id, role_id).await.unwrap() {
            member.remove_role(&ctx.http, role_id).await.unwrap();
        } else {
            member.add_role(&ctx.http, role_id).await.unwrap();
        }
    }

    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command.name("game").description("Add or Remove the tag");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let _options = &command.data.options;
        let guild_id = command.guild_id.unwrap().as_u64().to_owned();
        let games = GameHandler::new(ctx, guild_id).await;
        let components = CreateComponents::default()
            .add_action_row(games.action_row())
            .to_owned();

        HandlerResponse {
            content: "Select which tag you want to be on, if you select a tag you have already, it will be removed".to_string(),
            components: Some(components),
            ephemeral: true,
        }
    }
}
