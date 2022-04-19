use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        guild::Role,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
    },
};

pub struct Gulag;

async fn find_gulag_role(ctx: &Context, guild_id: u64) -> Option<Role> {
    match ctx.http.get_guild_roles(guild_id).await {
        Err(_why) => None,
        Ok(roles) => {
            for role in roles {
                if role.name == "gulag" {
                    return Some(role);
                }
            }
            None
        }
    }
}

impl Gulag {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        return command
            .name("gulag")
            .description("Send a user to the Gulag")
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
    ) -> String {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");

        if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
            match command.guild_id {
                None => return "no member".to_string(),
                Some(guild_id) => match find_gulag_role(&ctx, *guild_id.as_u64()).await {
                    None => return "couldn't find gulag role".to_string(),
                    Some(gulag_role) => {
                        let mut mem = ctx
                            .http
                            .get_member(*guild_id.as_u64(), *user.id.as_u64())
                            .await
                            .unwrap();

                        mem.add_role(&ctx.http, gulag_role.id).await.unwrap();

                        return format!("Sending @{} to the Gulag", user.name);
                    }
                },
            }
        } else {
            return "Please provide a valid user".to_string();
        };
    }
}
