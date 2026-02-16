use crate::{db::queries::features::FeatureQueries, utils::nickname, Context, Error};
use poise::CreateReply;
use serenity::EditMember;
use poise::serenity_prelude as serenity;

pub fn commands() -> Vec<poise::Command<crate::Data, Error>> {
    vec![phony(), horny(), feature()]
}

/// Helper to send an ephemeral reply
fn ephemeral_reply(content: impl Into<String>) -> CreateReply {
    CreateReply::default().content(content).ephemeral(true)
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
    let pool = &ctx.data().db_pool;
    if !FeatureQueries::is_enabled(pool, prefix) {
        ctx.send(ephemeral_reply("This feature is currently disabled")).await?;
        return Ok(());
    }

    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| Error::from("This command can only be used in a server"))?;

    let author = ctx.author();
    let mut member = ctx.http().get_member(guild_id, author.id).await?;

    let current_nick = member
        .nick
        .as_deref()
        .unwrap_or_else(|| member.display_name());

    let new_nick = nickname::fix_nickname(current_nick, prefix);

    member
        .edit(ctx.http(), EditMember::new().nickname(&new_nick))
        .await?;

    ctx.send(ephemeral_reply("Done")).await?;
    Ok(())
}

/// Toggle or list feature flags
#[poise::command(slash_command, guild_only, category = "Utility")]
pub async fn feature(
    ctx: Context<'_>,
    #[description = "Feature name to toggle"] name: Option<String>,
) -> Result<(), Error> {
    let pool = &ctx.data().db_pool;

    match name {
        Some(feature_name) => {
            let is_currently_enabled = FeatureQueries::is_enabled(pool, &feature_name);
            match FeatureQueries::update(pool, &feature_name, !is_currently_enabled) {
                Err(diesel::result::Error::NotFound) => {
                    ctx.send(ephemeral_reply("Couldn't match feature")).await?;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
                Ok(_) => {}
            }

            list_features(ctx).await?;
        }
        None => {
            list_features(ctx).await?;
        }
    }

    Ok(())
}

/// Helper to list all features
async fn list_features(ctx: Context<'_>) -> Result<(), Error> {
    use std::fmt::Write;
    
    let pool = &ctx.data().db_pool;
    let features = FeatureQueries::all(pool)?;

    let mut content = String::from("Here's all the features");
    for feat in features {
        write!(content, "\nName: `{}` Enabled: `{}`", feat.name, feat.enabled).unwrap();
    }

    ctx.send(ephemeral_reply(content)).await?;
    Ok(())
}
