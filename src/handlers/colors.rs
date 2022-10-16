use serenity::{
    builder::{CreateApplicationCommand, CreateComponents, CreateSelectMenuOption},
    client::Context,
    model::{interactions::application_command::ApplicationCommandInteraction, prelude::Role},
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
    pub fn new() -> Self {
        let mut g: Vec<Color> = Vec::new();
        g.push(Color::new("Red".to_string(), "color-red".to_string()));
        g.push(Color::new("Green".to_string(), "color-green".to_string()));
        g.push(Color::new("Blue".to_string(), "color-blue".to_string()));

        Self { colors: g }
    }

    // fn menu_options(&self) -> Vec<CreateSelectMenuOption> {
    //     let mut options = vec![];
    //     for color in self.colors.to_owned() {
    //         options.push(color.menu_option())
    //     }
    //     options
    // }
    //
    // fn select_menu(&self) -> CreateSelectMenu {
    //     let mut menu = CreateSelectMenu::default();
    //     menu.custom_id("color_select");
    //     menu.placeholder("Select your color");
    //     menu.options(|opt| opt.set_options(self.menu_options()));
    //     menu
    // }
    //
    // fn action_row(&self) -> CreateActionRow {
    //     let mut ar = CreateActionRow::default();
    //     ar.add_select_menu(self.select_menu());
    //     ar
    // }
    //
    // fn find_color_from_role_name(colors: Vec<Color>, role_name: String) -> Option<Color> {
    //     let unescaped_role_name = snailquote::unescape(role_name.as_str()).unwrap();
    //     let mut found_color = false;
    //     let mut color = Color::default();
    //     for g in colors.clone() {
    //         if g.role_name == unescaped_role_name {
    //             color = g;
    //             found_color = true;
    //             break;
    //         }
    //     }
    //
    //     println!("found? {} - {:#?}", found_color, color);
    //     if found_color {
    //         Some(color)
    //     } else {
    //         None
    //     }
    // }

    pub fn match_roles_and_colors(colors: Vec<Color>, roles: Vec<Role>) -> Vec<Color> {
        let mut g: Vec<Color> = Vec::new();
        for role in roles {
            for color in colors.clone() {
                if role.name == color.role_name {
                    let new_color = Color {
                        name: color.name.to_owned(),
                        role_name: color.role_name.to_owned(),
                        role_id: *role.id.as_u64(),
                    };
                    g.push(new_color);
                }
            }
        }
        return g;
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
        _ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> HandlerResponse {
        let _options = &command.data.options;
        let components = CreateComponents::default()
            .create_action_row(|row| {
                row.create_select_menu(|menu| {
                    menu.custom_id("color_select");
                    menu.placeholder("No color selected");
                    menu.options(|m| {
                        m.create_option(|o| o.label("Fuck").value("Fuck"));
                        m.create_option(|o| o.label("This").value("This"));
                        m.create_option(|o| o.label("Shit").value("Shit"))
                    })
                })
            })
            .to_owned();

        HandlerResponse {
            content: "Choose Which Color you want on your name".to_string(),
            components: Some(components),
            ephemeral: true,
        }
    }
}
