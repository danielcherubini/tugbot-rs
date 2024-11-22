use crate::db::{features, models};
use serenity::{
    builder::CreateApplicationCommand,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::{application_command::CommandDataOptionValue, command::CommandOptionType},
    },
};

use super::eventhandler::HandlerResponse;

pub fn setup_command(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    return command
        .name("feature")
        .description("Toggle Feature")
        .create_option(|option| {
            option
                .name("name")
                .description("The feature to enable")
                .kind(CommandOptionType::String)
                .required(false)
        });
}

pub async fn setup_interaction(command: &ApplicationCommandInteraction) -> HandlerResponse {
    match command.data.options.first() {
        Some(feature_option_value) => {
            if let CommandDataOptionValue::String(feature_name) =
                feature_option_value.resolved.as_ref().unwrap()
            {
                match features::all() {
                    Ok(f) => handle_feature(f, feature_name),
                    Err(e) => handle_error(e.to_string()),
                }
            } else {
                handle_error("Please provide a valid user".to_string())
            }
        }

        None => match features::all() {
            Ok(features) => handle_list_features(features),
            Err(e) => handle_error(e.to_string()),
        },
    }
}

fn handle_feature(features: Vec<models::Features>, feature_name: &String) -> HandlerResponse {
    for feat in features {
        if feat.name == *feature_name {
            println!("{:?}", feature_name);
            features::update(feat.name, !feat.enabled);
            return HandlerResponse {
                content: String::from("Done"),
                components: None,
                ephemeral: true,
            };
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
