use serenity::{
    builder::CreateApplicationCommand,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
        ApplicationCommandOptionType,
    },
};
pub struct Gulag;

impl Gulag {
    pub fn setup_gulag_application_command(
        command: &mut CreateApplicationCommand,
    ) -> &mut CreateApplicationCommand {
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
    pub fn setup_gulag_interaction(command: &ApplicationCommandInteraction) -> String {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected user object");
        if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
            return format!("Sending {} to the Gulag", user.tag());
        } else {
            return "Please provide a valid user".to_string();
        };
    }
}
