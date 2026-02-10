use serenity::{
    builder::CreateApplicationCommand,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::{application_command::CommandDataOptionValue, command::CommandOptionType},
    },
};

use crate::{db::models, features};

use super::HandlerResponse;

pub struct Feat;

impl Feat {
    pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        command
            .name("feature")
            .description("Toggle Feature")
            .create_option(|option| {
                option
                    .name("name")
                    .description("The feature to enable")
                    .kind(CommandOptionType::String)
                    .required(false)
            })
    }

    pub async fn setup_interaction(command: &ApplicationCommandInteraction) -> HandlerResponse {
        match command.data.options.first() {
            Some(feature_option_value) => {
                if let CommandDataOptionValue::String(feature_name) =
                    feature_option_value.resolved.as_ref().unwrap()
                {
                    match features::Features::all() {
                        Ok(f) => Feat::handle_feature(f, feature_name),
                        Err(e) => Feat::handle_error(e.to_string()),
                    }
                } else {
                    Feat::handle_error("Please provide a valid user".to_string())
                }
            }
            None => match features::Features::all() {
                Ok(features) => Feat::handle_list_features(features),
                Err(e) => Feat::handle_error(e.to_string()),
            },
        }
    }

    fn handle_feature(features: Vec<models::Features>, feature_name: &String) -> HandlerResponse {
        for feat in features {
            if feat.name == *feature_name {
                println!("{:?}", feature_name);
                features::Features::update(&feat.name, !feat.enabled);
                return Self::handle_list_features(features::Features::all().unwrap());
            }
        }

        HandlerResponse {
            content: String::from("Couldn't match feature"),
            components: None,
            ephemeral: true,
        }
    }

    fn handle_list_features(features: Vec<models::Features>) -> HandlerResponse {
        let mut content = "Here's all the features".to_string();

        for feature in features {
            content = format!(
                "{}\nName: `{}` Enabled: `{}`",
                content, feature.name, feature.enabled
            );
        }

        HandlerResponse {
            content,
            components: None,
            ephemeral: true,
        }
    }

    fn handle_error(content: String) -> HandlerResponse {
        HandlerResponse {
            content,
            components: None,
            ephemeral: true,
        }
    }
}
