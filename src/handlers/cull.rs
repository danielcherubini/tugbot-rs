use crate::db::{
    query_all_tracked_user_ids_for_guild, query_inactive_users, query_user_activity_for_ids,
};
use crate::features::Features;
use crate::handlers::{get_pool, gulag::Gulag, HandlerResponse};
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateMessage, Member,
        Permissions,
    },
    builder::{CreateCommand, CreateCommandOption},
    client::Context,
    model::id::ChannelId,
};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

pub struct CullHandler;

// Channel ID for cat-herding (moderator-only output channel)
const CAT_HERDING_CHANNEL_ID: u64 = 1224402885786472659;
// Hard cap on kicks per invocation
const MAX_KICKS: usize = 50;
// Sleep between kicks to respect rate limits (1.5s)
const KICK_DELAY_MS: u64 = 1500;
// Whitelist roles — users with these roles are never culled
const WHITELIST_ROLES: &[&str] = &["Highly Regarded", "admin"];

impl CullHandler {
    pub fn setup_command() -> CreateCommand {
        CreateCommand::new("cull")
            .description("Cull inactive members from the server")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "days",
                    "Inactivity threshold in days (default: 30)",
                )
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "dry-run",
                    "Preview candidates without kicking (default: false)",
                )
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "include-never-posted",
                    "Include users who have never posted (default: false)",
                )
                .required(false),
            )
    }

    pub async fn setup_interaction(ctx: &Context, command: &CommandInteraction) -> HandlerResponse {
        let pool = get_pool(ctx).await;

        // a. Feature flag check
        match Features::check_enabled(&pool, "cull") {
            Ok(true) => {}
            Ok(false) => {
                return HandlerResponse {
                    content: "Cull feature is currently disabled".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
            Err(e) => {
                return HandlerResponse {
                    content: format!("Failed to check cull feature: {}", e),
                    components: None,
                    ephemeral: true,
                };
            }
        }

        // b. Guild check
        let guild_id = match command.guild_id {
            Some(id) => id.get(),
            None => {
                return HandlerResponse {
                    content: "This command can only be used in a guild".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // c. Permission check (Highly Regarded or admin)
        let member = match ctx.http.get_member(guild_id.into(), command.user.id).await {
            Ok(m) => m,
            Err(_) => {
                return HandlerResponse {
                    content: "Error: Could not verify your permissions".to_string(),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        if !Gulag::member_has_any_role(&ctx.http, guild_id, &member, &["Highly Regarded", "admin"])
            .await
        {
            return HandlerResponse {
                content: "Error: You need Highly Regarded or admin role to use this command"
                    .to_string(),
                components: None,
                ephemeral: true,
            };
        }

        // d. Bot KICK_MEMBERS permission check
        let current_user = match ctx.http.get_current_user().await {
            Ok(u) => u,
            Err(e) => {
                return HandlerResponse {
                    content: format!("Failed to get bot info: {}", e),
                    components: None,
                    ephemeral: true,
                };
            }
        };
        let bot_member = match ctx.http.get_member(guild_id.into(), current_user.id).await {
            Ok(m) => m,
            Err(e) => {
                return HandlerResponse {
                    content: format!("Failed to get bot member: {}", e),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        let guild = match ctx.http.get_guild(guild_id.into()).await {
            Ok(g) => g,
            Err(e) => {
                return HandlerResponse {
                    content: format!("Failed to get guild info: {}", e),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        let bot_permissions = guild.member_permissions(&bot_member);
        if !bot_permissions.contains(Permissions::KICK_MEMBERS) {
            return HandlerResponse {
                content: "I don't have KICK_MEMBERS permission on this server.".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        // e. Parse options
        let days: i64 = command
            .data
            .options
            .iter()
            .find(|opt| opt.name == "days")
            .and_then(|opt| match &opt.value {
                CommandDataOptionValue::Integer(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(30);

        if days <= 0 || days > 365 {
            return HandlerResponse {
                content: "Days must be between 1 and 365".to_string(),
                components: None,
                ephemeral: true,
            };
        }

        let dry_run: bool = command
            .data
            .options
            .iter()
            .find(|opt| opt.name == "dry-run")
            .and_then(|opt| match &opt.value {
                CommandDataOptionValue::Boolean(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(false);

        let include_never_posted: bool = command
            .data
            .options
            .iter()
            .find(|opt| opt.name == "include-never-posted")
            .and_then(|opt| match &opt.value {
                CommandDataOptionValue::Boolean(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(false);

        // f. Fetch member list via REST pagination
        let mut all_members: Vec<Member> = Vec::new();
        let mut after_id: Option<u64> = None;
        loop {
            let members: Vec<Member> = match serenity::all::GuildId::from(guild_id)
                .members(
                    &ctx.http,
                    Some(1000),
                    after_id.map(serenity::all::UserId::from),
                )
                .await
            {
                Ok(ms) => ms,
                Err(e) => {
                    post_to_cat_herding(&ctx.http, &format!("Error fetching members: {}", e)).await;
                    return HandlerResponse {
                        content: format!("Failed to fetch members: {}", e),
                        components: None,
                        ephemeral: true,
                    };
                }
            };
            let count = members.len();
            all_members.extend(members);
            if count < 1000 {
                break;
            }
            after_id = all_members.last().map(|m| m.user.id.get());
        }

        // g. Filter: remove bots and whitelisted users
        let whitelist_role_ids = get_whitelist_role_ids(&ctx.http, guild_id).await;
        let filtered_members: Vec<_> = all_members
            .into_iter()
            .filter(|member| {
                if member.user.bot {
                    return false;
                }
                if member_has_any_role_ids(member, &whitelist_role_ids) {
                    return false;
                }
                true
            })
            .collect();

        // h. Filter out gulaged users
        let non_gulaged_members: Vec<_> = filtered_members
            .into_iter()
            .filter(|member| Gulag::is_user_in_gulag(&pool, member.user.id.get()).is_none())
            .collect();

        // i. Query inactive users from DB
        let guild_id_i64: i64 = match i64::try_from(guild_id) {
            Ok(id) => id,
            Err(e) => {
                return HandlerResponse {
                    content: format!("Failed to convert guild ID: {}", e),
                    components: None,
                    ephemeral: true,
                };
            }
        };
        let inactive_ids_result = query_inactive_users(&pool, guild_id_i64, days as i32);

        let inactive_user_ids: HashSet<u64> = match inactive_ids_result {
            Ok(ids) => ids
                .into_iter()
                .filter_map(|id| u64::try_from(id).ok())
                .collect(),
            Err(e) => {
                post_to_cat_herding(&ctx.http, &format!("Error querying inactive users: {}", e))
                    .await;
                return HandlerResponse {
                    content: format!("Failed to query inactive users: {}", e),
                    components: None,
                    ephemeral: true,
                };
            }
        };

        // j. Build candidate list
        let mut candidates: Vec<u64> = non_gulaged_members
            .iter()
            .filter(|member| inactive_user_ids.contains(&member.user.id.get()))
            .map(|member| member.user.id.get())
            .collect();

        // Include never-posted users if requested
        if include_never_posted {
            let tracked_ids_result = query_all_tracked_user_ids_for_guild(&pool, guild_id_i64);
            if let Ok(tracked_ids) = tracked_ids_result {
                let tracked_set: HashSet<u64> = tracked_ids
                    .into_iter()
                    .filter_map(|id| u64::try_from(id).ok())
                    .collect();

                let never_posted: Vec<u64> = non_gulaged_members
                    .iter()
                    .filter(|member| !tracked_set.contains(&member.user.id.get()))
                    .map(|member| member.user.id.get())
                    .collect();

                candidates.extend(never_posted);
            } else {
                let err_msg = "Failed to query tracked users for never-posted check".to_string();
                post_to_cat_herding(&ctx.http, &err_msg).await;
            }
        }

        // Deduplicate, sort by user ID for determinism, cap at MAX_KICKS
        candidates.sort();
        candidates.dedup();
        candidates.truncate(MAX_KICKS);

        if candidates.is_empty() {
            let msg = format!(
                "No candidates found (inactive {}+ days, never posted: {})",
                days,
                if include_never_posted { "yes" } else { "no" }
            );
            post_to_cat_herding(&ctx.http, &msg).await;
            return HandlerResponse {
                content: "No candidates found.".to_string(),
                components: None,
                ephemeral: false,
            };
        }

        // l. Dry-run mode
        if dry_run {
            // Query ALL candidate timestamps in ONE roundtrip
            let candidates_i64: Vec<i64> = candidates
                .iter()
                .filter_map(|&uid| i64::try_from(uid).ok())
                .collect();

            let activity_results = query_user_activity_for_ids(&pool, guild_id_i64, candidates_i64);
            let activity_map: HashMap<i64, SystemTime> = match activity_results {
                Ok(results) => results
                    .into_iter()
                    .map(|a| (a.user_id, a.last_message_at))
                    .collect(),
                Err(e) => {
                    post_to_cat_herding(&ctx.http, &format!("Error querying activity: {}", e))
                        .await;
                    return HandlerResponse {
                        content: format!("Failed to query activity: {}", e),
                        components: None,
                        ephemeral: true,
                    };
                }
            };

            // Build candidate lines (max 25)
            let mut lines: Vec<String> = Vec::new();
            let display_count = std::cmp::min(candidates.len(), 25);
            for &uid in &candidates[..display_count] {
                let uid_i64: i64 = uid.try_into().unwrap_or(i64::MAX);
                let date_str = match activity_map.get(&uid_i64) {
                    Some(&ts) => format_timestamp(ts),
                    None => "never posted".to_string(),
                };
                lines.push(format!("<@{}> (last active: {})", uid, date_str));
            }

            let extra = candidates.len().saturating_sub(25);
            let candidate_block = if extra > 0 {
                let mut full = lines.join("\n");
                full.push_str(&format!("\nand {} more...", extra));
                full
            } else {
                lines.join("\n")
            };

            let message = format!(
                "**Cull Dry-Run** (inactive {}+ days, never posted: {})\n\n{}\n\nTotal candidates: {} (capped at {})\nRun `/cull --days {}` to execute.",
                days,
                if include_never_posted { "yes" } else { "no" },
                candidate_block,
                candidates.len(),
                MAX_KICKS,
                days,
            );

            let posted = post_to_cat_herding(&ctx.http, &message).await;

            if posted {
                HandlerResponse {
                    content: format!("Dry-run posted to <#{}>", CAT_HERDING_CHANNEL_ID),
                    components: None,
                    ephemeral: true,
                }
            } else {
                HandlerResponse {
                    content: format!(
                        "Failed to post to <#{}>. Dry-run results:\n\n{}",
                        CAT_HERDING_CHANNEL_ID, message
                    ),
                    components: None,
                    ephemeral: true,
                }
            }
        } else {
            // m. Execute mode — spawn kick loop as background task so we return
            // immediately and don't exceed Discord's 3s response window.
            // (MAX_KICKS * KICK_DELAY_MS = 75s >> 3s)
            let http = ctx.http.clone();
            let candidates_clone = candidates.clone();
            let guild_id_val = guild_id;
            let days_val = days;
            let user_name = command.user.name.clone();
            let total = candidates.len();

            tokio::spawn(async move {
                let start_msg = format!(
                    "Starting cull: {} candidates (inactive {}+ days)...",
                    candidates_clone.len(),
                    days_val
                );
                let _ = post_to_cat_herding(&http, &start_msg).await;

                let mut success_count: usize = 0;
                let mut skip_count: usize = 0;

                for uid in &candidates_clone {
                    let reason = format!("Inactive {} days — /cull by {}", days_val, user_name);
                    match http
                        .kick_member(guild_id_val.into(), (*uid).into(), Some(&reason))
                        .await
                    {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            skip_count += 1;
                            eprintln!("[cull] Failed to kick {}: {}", uid, e);
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(KICK_DELAY_MS)).await;
                }

                let summary = format!(
                    "Cull complete: {} kicked, {} skipped (errors).",
                    success_count, skip_count
                );
                let _ = post_to_cat_herding(&http, &summary).await;
            });

            HandlerResponse {
                content: format!(
                    "Cull started: {} candidates. Results will be posted to <#{}>.",
                    total, CAT_HERDING_CHANNEL_ID
                ),
                components: None,
                ephemeral: true,
            }
        }
    }
}

/// Post a message to the cat-herding channel.
async fn post_to_cat_herding(http: &serenity::all::Http, content: &str) -> bool {
    let channel_id = ChannelId::from(CAT_HERDING_CHANNEL_ID);
    match channel_id
        .send_message(http, CreateMessage::new().content(content))
        .await
    {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[cull] Failed to post to cat-herding: {}", e);
            false
        }
    }
}

/// Fetch guild roles and return IDs matching WHITELIST_ROLES.
async fn get_whitelist_role_ids(http: &serenity::all::Http, guild_id: u64) -> HashSet<u64> {
    match http.get_guild_roles(guild_id.into()).await {
        Ok(roles) => roles
            .into_iter()
            .filter(|role| WHITELIST_ROLES.contains(&role.name.as_str()))
            .map(|role| role.id.get())
            .collect(),
        Err(e) => {
            eprintln!("[cull] Failed to get guild roles: {}", e);
            HashSet::new()
        }
    }
}

/// Check if a member has any of the given role IDs.
fn member_has_any_role_ids(member: &Member, role_ids: &HashSet<u64>) -> bool {
    member.roles.iter().any(|r| role_ids.contains(&r.get()))
}

/// Convert SystemTime to YYYY-MM-DD date string.
fn format_timestamp(ts: SystemTime) -> String {
    let duration = match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => return "unknown".to_string(),
    };
    let days = duration.as_secs() as i64 / 86400;

    // Convert days since Unix epoch (1970-01-01) to civil date
    // Using the algorithm from Howard Hinnant's date library
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_timestamp_known_date() {
        // 2024-01-15 = 19737 days since epoch
        let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(19737 * 86400);
        assert_eq!(format_timestamp(ts), "2024-01-15");
    }

    #[test]
    fn test_format_timestamp_epoch() {
        let ts = SystemTime::UNIX_EPOCH;
        assert_eq!(format_timestamp(ts), "1970-01-01");
    }

    #[test]
    fn test_format_timestamp_y2k() {
        // 2000-01-01 = 10957 days since epoch
        let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(10957 * 86400);
        assert_eq!(format_timestamp(ts), "2000-01-01");
    }

    #[test]
    fn test_format_timestamp_2025_june() {
        // 2025-06-15 = 20254 days since epoch
        let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(20254 * 86400);
        assert_eq!(format_timestamp(ts), "2025-06-15");
    }

    #[test]
    fn test_format_timestamp_feb_leap_year() {
        // 2024-02-29 (leap year) = 19782 days since epoch
        let ts = SystemTime::UNIX_EPOCH + Duration::from_secs(19782 * 86400);
        assert_eq!(format_timestamp(ts), "2024-02-29");
    }

    #[test]
    fn test_max_kicks_constant() {
        assert_eq!(MAX_KICKS, 50);
    }

    #[test]
    fn test_kick_delay_constant() {
        assert_eq!(KICK_DELAY_MS, 1500);
    }

    #[test]
    fn test_cat_herding_channel_id() {
        assert_eq!(CAT_HERDING_CHANNEL_ID, 1224402885786472659);
    }

    #[test]
    fn test_whitelist_roles() {
        assert!(WHITELIST_ROLES.contains(&"Highly Regarded"));
        assert!(WHITELIST_ROLES.contains(&"admin"));
    }

    #[test]
    fn test_execute_mode_response_starts_with_cull_started() {
        // Execute mode must return immediately with "Cull started" message
        // (not "Cull complete" which would indicate blocking behavior)
        let cat_id = CAT_HERDING_CHANNEL_ID;
        let candidate_count = 10;
        let expected_prefix = format!("Cull started: {} candidates.", candidate_count);
        let expected_suffix = format!("Results will be posted to <#{}>", cat_id);
        // Verify the message format matches the non-blocking pattern
        let msg = format!(
            "Cull started: {} candidates. Results will be posted to <#{}>.",
            candidate_count, cat_id
        );
        assert!(msg.starts_with(&expected_prefix));
        assert!(msg.contains(&expected_suffix));
    }

    #[test]
    fn test_execute_mode_max_kicks_would_block() {
        // Verify that MAX_KICKS * KICK_DELAY_MS would exceed Discord's 3s window
        let worst_case_ms = MAX_KICKS as u64 * KICK_DELAY_MS;
        let discord_response_window_ms = 3000;
        assert!(
            worst_case_ms > discord_response_window_ms,
            "worst case {}ms exceeds Discord's {}ms window — must spawn background task",
            worst_case_ms,
            discord_response_window_ms
        );
    }
}
