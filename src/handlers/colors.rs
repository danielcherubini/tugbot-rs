use serenity::{
    builder::{
        CreateActionRow, CreateApplicationCommand, CreateComponents, CreateSelectMenu,
        CreateSelectMenuOption,
    },
    client::Context,
    http::CacheHttp,
    model::{interactions::application_command::ApplicationCommandInteraction, prelude::RoleId},
};

use super::handlers::HandlerResponse;
pub struct Colors {
    pub colors: Vec<Color>,
}

#[derive(Default, Clone, Debug)]
pub struct Color {
    pub name: String,
    pub role_id: u64,
}

impl Color {
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

impl Colors {
    pub async fn new(ctx: &Context, guild_id: u64) -> Self {
        let mut g: Vec<Color> = Vec::new();
        // Get Roles from Discord and iterate over them to create the colors array
        let roles = &ctx.http.get_guild_roles(guild_id).await.unwrap();
        for role in roles {
            if role.name.starts_with("color-") {
                g.push(Color::new(role.name.to_string(), *role.id.as_u64()));
            }
        }

        Self { colors: g }
    }

    fn menu_options(&self) -> Vec<CreateSelectMenuOption> {
        let mut options = vec![];
        for color in self.colors.to_owned() {
            options.push(color.menu_option())
        }
        options
    }

    fn select_menu(&self) -> CreateSelectMenu {
        let mut menu = CreateSelectMenu::default();
        menu.custom_id("color_select");
        menu.placeholder("Select your color");
        menu.options(|opt| opt.set_options(self.menu_options()));
        menu
    }

    fn action_row(&self) -> CreateActionRow {
        let mut ar = CreateActionRow::default();
        ar.add_select_menu(self.select_menu());
        ar
    }

    pub async fn swap_color_role(ctx: &Context, guild_id: u64, user_id: u64, role_id: u64) {
        let colors = Colors::new(ctx, guild_id).await.colors;
        let mut member = ctx.http().get_member(guild_id, user_id).await.unwrap();
        let mut role_ids = vec![];
        for color in colors {
            role_ids.push(RoleId(color.role_id));
        }

        member.remove_roles(&ctx.http, &role_ids).await.unwrap();

        member.add_role(&ctx.http, role_id).await.unwrap();
    }

    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("color")
            .description("Change your nickname color");
    }

    pub async fn setup_interaction(
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let _options = &command.data.options;
        let guild_id = command.guild_id.unwrap().as_u64().to_owned();
        let colors = Colors::new(ctx, guild_id).await;
        let components = CreateComponents::default()
            .add_action_row(colors.action_row())
            .to_owned();

        HandlerResponse {
            content: "Choose Which Color you want on your name".to_string(),
            components: Some(components),
            ephemeral: true,
        }
    }
}
