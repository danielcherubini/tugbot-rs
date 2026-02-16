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
    let pool = ctx.data().db_pool.clone();
    let prefix_owned = prefix.to_string();

    let enabled = tokio::task::spawn_blocking({
        let pool = pool.clone();
        let prefix = prefix_owned.clone();
        move || FeatureQueries::is_enabled(&pool, &prefix)
    })
    .await
    .map_err(|e| Error::from(format!("spawn_blocking failed: {}", e)))?;

    if !enabled {
        ctx.send(ephemeral_reply("This feature is currently disabled")).await?;
        return Ok(());
    }

    let member = ctx
        .author_member()
        .await
        .ok_or_else(|| Error::from("Could not find member"))?;
    let mut member = member.into_owned();

    let current_nick = member
        .nick
        .as_deref()
        .unwrap_or_else(|| member.display_name());

    let new_nick = nickname::fix_nickname(current_nick, &prefix_owned);

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
    let pool = ctx.data().db_pool.clone();

    match name {
        Some(feature_name) => {
            let result = tokio::task::spawn_blocking({
                let pool = pool.clone();
                let name = feature_name.clone();
                move || {
                    let is_currently_enabled = FeatureQueries::is_enabled(&pool, &name);
                    FeatureQueries::update(&pool, &name, !is_currently_enabled)
                }
            })
            .await
            .map_err(|e| Error::from(format!("spawn_blocking failed: {}", e)))?;

            match result {
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

    let pool = ctx.data().db_pool.clone();
    let features = tokio::task::spawn_blocking(move || FeatureQueries::all(&pool))
        .await
        .map_err(|e| Error::from(format!("spawn_blocking failed: {}", e)))??;

    let mut content = String::from("Here's all the features");
    for feat in features {
        write!(content, "\nName: `{}` Enabled: `{}`", feat.name, feat.enabled).unwrap();
    }

    ctx.send(ephemeral_reply(content)).await?;
    Ok(())
}
