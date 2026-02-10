use diesel::*;
use serenity::{
    client::Context,
    http::GuildPagination,
    model::id::{GuildId, RoleId},
};

use crate::db::{create_server, models::Server, schema::servers::dsl::*, DbPool};

pub struct Servers {
    pub guild_id: GuildId,
    pub gulag_id: RoleId,
}

impl Servers {
    pub async fn get_servers(ctx: &Context, pool: &DbPool) -> Vec<Servers> {
        let mut serverss = Vec::new();

        // Use spawn_blocking to avoid blocking async runtime
        let pool_clone = pool.clone();
        let results = match tokio::task::spawn_blocking(move || {
            let mut connection = pool_clone.get().map_err(|e| {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UnableToSendCommand,
                    Box::new(e.to_string()),
                )
            })?;
            servers.load::<Server>(&mut connection)
        })
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                eprintln!("Database error loading servers: {}", e);
                return serverss;
            }
            Err(e) => {
                eprintln!("Task join error: {}", e);
                return serverss;
            }
        };

        if results.is_empty() {
            println!("Nothing found in DB");
            let current_guild_id = GuildId::new(1);
            let guilds = match ctx
                .http
                .get_guilds(Some(GuildPagination::After(current_guild_id)), Some(10))
                .await
            {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Failed to get guilds: {}", e);
                    return serverss;
                }
            };

            for guild_info in guilds {
                let id64: u64 = u64::from(guild_info.id);
                let roles = match ctx.http.get_guild_roles(id64.into()).await {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Failed to get roles for guild {}: {}", id64, e);
                        continue;
                    }
                };

                for role in roles {
                    if role.name == "gulag" {
                        // Safe conversion with overflow check
                        let guild_id_i64 = match i64::try_from(guild_info.id.get()) {
                            Ok(gid) => gid,
                            Err(e) => {
                                eprintln!("Guild ID overflow: {}", e);
                                continue;
                            }
                        };
                        let role_id_i64 = match i64::try_from(role.id.get()) {
                            Ok(rid) => rid,
                            Err(e) => {
                                eprintln!("Role ID overflow: {}", e);
                                continue;
                            }
                        };

                        match create_server(pool, guild_id_i64, role_id_i64) {
                            Ok(_) => {
                                serverss.push(Servers {
                                    guild_id: guild_info.id,
                                    gulag_id: role.id,
                                });
                            }
                            Err(e) => {
                                eprintln!("Failed to create server in DB: {}", e);
                            }
                        }
                    }
                }
            }
        } else {
            println!("found in DB");

            for s in results {
                match ctx.http.get_guild((s.guild_id as u64).into()).await {
                    Ok(guildid) => {
                        serverss.push(Servers {
                            guild_id: guildid.id,
                            gulag_id: RoleId::new(s.gulag_id as u64),
                        });
                    }
                    Err(err) => {
                        println!("Couldn't connect to server with guild_id {:?}", err);
                        // Delete server in spawn_blocking to avoid blocking async runtime
                        let pool_clone = pool.clone();
                        let server_id = s.id;
                        tokio::spawn(async move {
                            let _ = tokio::task::spawn_blocking(move || {
                                if let Ok(mut conn) = pool_clone.get() {
                                    let _ = diesel::delete(servers.filter(id.eq(server_id)))
                                        .execute(&mut conn);
                                }
                            })
                            .await;
                        });
                    }
                }
            }
        }

        serverss
    }
}
