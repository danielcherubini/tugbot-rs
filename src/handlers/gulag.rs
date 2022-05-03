use std::{sync::Arc, time::Duration};

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    http::Http,
    model::{
        guild::{Member, Role},
        id::RoleId,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
    },
};
use tokio::{task::spawn, time::sleep};
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
    async fn remove_from_gulag(
        http: Arc<Http>,
        mut mem: Member,
        gulag_role_id: RoleId,
        channel_id: u64,
    ) {
        mem.remove_role(&http, gulag_role_id).await.unwrap();
        let message = format!("Freeing {} from the gulag", mem.display_name());
        let channel = http.get_channel(channel_id).await.unwrap();
        channel
            .id()
            .send_message(http, |m| m.content(message))
            .await
            .unwrap();
    }

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
        let channel_id = command.channel_id.0;
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
                        let http = Arc::clone(&ctx.http);
                        spawn(async move {
                            sleep(Duration::from_secs(300)).await;
                            Gulag::remove_from_gulag(http, mem, gulag_role.id, channel_id).await;
                        });

                        return format!("Sending @{} to the Gulag", user.name);
                    }
                },
            }
        } else {
            return "Please provide a valid user".to_string();
        };
    }
}
