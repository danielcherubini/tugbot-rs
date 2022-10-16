use serenity::{
    builder::{
        CreateActionRow, CreateApplicationCommand, CreateComponents, CreateSelectMenu,
        CreateSelectMenuOption,
    },
    client::Context,
    model::interactions::application_command::ApplicationCommandInteraction,
};

use super::handlers::HandlerResponse;
pub struct Colors {
    pub colors: Vec<Color>,
}

#[derive(Default, Clone, Debug)]
pub struct Color {
    pub name: String,
    pub role_name: String,
    pub role_id: u64,
}

impl Color {
    pub fn new(name: String, role_name: String) -> Self {
        Self {
            name,
            role_name,
            role_id: 0,
        }
    }

    pub fn menu_option(&self) -> CreateSelectMenuOption {
        let mut opt = CreateSelectMenuOption::default();
        opt.label(self.name.to_string());
        opt.value(self.role_name.to_ascii_lowercase());
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
                g.push(Color::new(role.name.to_string(), role.id.to_string()));
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

    // fn does_user_have_role(user_roles: Vec<RoleId>, role_id: RoleId) -> Option<RoleId> {
    //     let mut found_role = false;
    //     let mut role = RoleId::default();
    //     for user_role in user_roles {
    //         if user_role == role_id {
    //             found_role = true;
    //             role = role_id;
    //         }
    //     }
    //     if found_role {
    //         Some(role)
    //     } else {
    //         None
    //     }
    // }

    // async fn add_or_remove_color<'a>(
    //     ctx: &Context,
    //     command: &ApplicationCommandInteraction,
    //     options: &Vec<ApplicationCommandInteractionDataOption>,
    // ) -> HandlerResponse<'a> {
    //     let mut handler_response = HandlerResponse::default();
    //     handler_response.ephemeral = true;
    //
    //     let guild_id = command.guild_id.unwrap();
    //     let roles = &ctx.http.get_guild_roles(guild_id.0).await.unwrap();
    //     let colors = Colors::match_roles_and_colors(Colors::new().colors, roles.to_owned());
    //     let user = &command.user;
    //
    //     let mut mem = ctx
    //         .http
    //         .get_member(*guild_id.as_u64(), *user.id.as_u64())
    //         .await
    //         .unwrap();
    //
    //     for option in options {
    //         match &option.value {
    //             Some(value) => match option.name.as_str() {
    //                 "add" => handler_response.content = "Please select the color".to_string(),
    //                 "remove" => {
    //                     match Self::find_color_from_role_name(colors.to_owned(), value.to_string())
    //                     {
    //                         Some(color) => {
    //                             match Self::does_user_have_role(
    //                                 mem.roles.to_owned(),
    //                                 RoleId(color.role_id),
    //                             ) {
    //                                 Some(role) => {
    //                                     mem.remove_role(&ctx, role).await.unwrap();
    //                                     handler_response.content = "Removed Role".to_string();
    //                                 }
    //                                 None => {
    //                                     println!("user didn't have role");
    //                                     handler_response.content =
    //                                         "You didnt fill out the request correctly".to_string();
    //                                 }
    //                             }
    //                         }
    //                         None => {
    //                             println!("couldn't find color from role name");
    //                             handler_response.content =
    //                                 "You didnt fill out the request correctly".to_string();
    //                         }
    //                     };
    //                 }
    //                 _ => {
    //                     println!("nothing");
    //                     handler_response.content =
    //                         "You didnt fill out the request correctly".to_string();
    //                 }
    //             },
    //             None => {
    //                 println!("nothing");
    //                 handler_response.content =
    //                     "You didnt fill out the request correctly".to_string();
    //             }
    //         }
    //     }
    //
    //     return handler_response;
    // }

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
