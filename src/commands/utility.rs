// Utility commands (phony, horny, feature)

use crate::{db::queries::features::FeatureQueries, Context, Error};
use poise::serenity_prelude as serenity;
use serenity::EditMember;

pub fn commands() -> Vec<poise::Command<crate::Data, Error>> {
    vec![phony(), horny(), feature()]
}

/// Mark yourself as phony/watching
#[poise::command(slash_command, guild_only, category = "Utility")]
pub async fn phony(ctx: Context<'_>) -> Result<(), Error> {
    nickname_command(ctx, "phony").await
}

/// Mark yourself as horny/lfg
#[poise::command(slash_command, guild_only, category = "Utility")]
pub async fn horny(ctx: Context<'_>) -> Result<(), Error> {
    nickname_command(ctx, "horny").await
}

/// Shared logic for nickname commands
async fn nickname_command(ctx: Context<'_>, prefix: &str) -> Result<(), Error> {
    // Check if feature is enabled
    let pool = &ctx.data().db_pool;
    if !FeatureQueries::is_enabled(pool, prefix) {
        ctx.say("This feature is currently disabled").await?;
        return Ok(());
    }

    // Get guild and member
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| Error::from("This command can only be used in a server"))?;
    
    let author = ctx.author();
    let mut member = ctx.http().get_member(guild_id, author.id).await?;

    // Get current nickname or display name
    let current_nick = member
        .nick
        .as_deref()
        .unwrap_or_else(|| member.display_name());

    // Apply nickname transformation
    let new_nick = fix_nickname(current_nick, prefix);

    // Update nickname
    member
        .edit(ctx.http(), EditMember::new().nickname(&new_nick))
        .await?;

    ctx.say("Done").await?;
    Ok(())
}

/// Toggle or manage feature flags
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "ADMINISTRATOR",
    category = "Utility"
)]
pub async fn feature(
    ctx: Context<'_>,
    #[description = "Feature name to toggle"] name: Option<String>,
) -> Result<(), Error> {
    let pool = &ctx.data().db_pool;

    match name {
        Some(feature_name) => {
            // Toggle specific feature
            let features = FeatureQueries::all(pool)?;
            let mut found = false;

            for feat in features {
                if feat.name == feature_name {
                    found = true;
                    FeatureQueries::update(pool, &feat.name, !feat.enabled)?;
                    break;
                }
            }

            if !found {
                ctx.say("Couldn't match feature").await?;
                return Ok(());
            }

            // Show updated list
            list_features(ctx).await?;
        }
        None => {
            // List all features
            list_features(ctx).await?;
        }
    }

    Ok(())
}

/// Helper to list all features
async fn list_features(ctx: Context<'_>) -> Result<(), Error> {
    let pool = &ctx.data().db_pool;
    let features = FeatureQueries::all(pool)?;

    let mut content = "Here's all the features".to_string();
    for feat in features {
        content = format!(
            "{}\nName: `{}` Enabled: `{}`",
            content, feat.name, feat.enabled
        );
    }

    ctx.say(content).await?;
    Ok(())
}

// Nickname transformation logic (copied from handlers/nickname.rs)

fn clean_username(nick: &str) -> String {
    nick.replace("phony | ", "").replace("horny | ", "")
}

/// fix_nickname is a function to add the nickname for horny/phony
fn fix_nickname(nick: &str, prefix: &str) -> String {
    // check if the nickname has the prefix in it
    let nick_to_find = format!("{} | ", prefix);
    if nick.contains(&nick_to_find) {
        // the prefix is already in the nick so just clean
        clean_username(nick)
    } else if nick.contains(" | ") {
        // the prefix doesn't match, but there's a pipe in there
        format!("{} | {}", prefix, clean_username(nick))
    } else {
        format!("{} | {}", prefix, nick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horny() {
        let nick = String::from("foo");
        let prefix = String::from("horny");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("horny | foo"));
    }

    #[test]
    fn test_phony() {
        let nick = String::from("foo");
        let prefix = String::from("phony");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("phony | foo"));
    }

    #[test]
    fn test_swap() {
        let nick = String::from("horny | foo");
        let prefix = String::from("phony");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("phony | foo"));
    }

    #[test]
    fn test_nickname_clean_one() {
        let nick = String::from("horny | foo");
        let prefix = String::from("horny");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }

    #[test]
    fn test_nickname_clean_all() {
        let nick = String::from("phony | horny | foo");
        let prefix = String::from("phony");
        let positive_test = fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }

    #[test]
    fn test_empty_nickname() {
        let nick = String::from("");
        let prefix = String::from("horny");
        let result = fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("horny | "));
    }

    #[test]
    fn test_clean_username_removes_both_prefixes() {
        let nick = String::from("phony | horny | username");
        let result = clean_username(&nick);
        assert_eq!(result, String::from("username"));
    }

    #[test]
    fn test_clean_username_no_prefix() {
        let nick = String::from("username");
        let result = clean_username(&nick);
        assert_eq!(result, String::from("username"));
    }

    #[test]
    fn test_nickname_with_multiple_pipes() {
        let nick = String::from("other | prefix | username");
        let prefix = String::from("horny");
        let result = fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("horny | other | prefix | username"));
    }

    #[test]
    fn test_nickname_already_has_correct_prefix() {
        let nick = String::from("phony | username");
        let prefix = String::from("phony");
        let result = fix_nickname(&nick, &prefix);
        // Should clean it (toggle off)
        assert_eq!(result, String::from("username"));
    }
}
