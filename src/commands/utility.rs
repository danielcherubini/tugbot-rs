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
                ctx.send(ephemeral_reply("Couldn't match feature")).await?;
                return Ok(());
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
    let pool = &ctx.data().db_pool;
    let features = FeatureQueries::all(pool)?;

    let mut content = "Here's all the features".to_string();
    for feat in features {
        content = format!(
            "{}\nName: `{}` Enabled: `{}`",
            content, feat.name, feat.enabled
        );
    }

    ctx.send(ephemeral_reply(content)).await?;
    Ok(())
}
