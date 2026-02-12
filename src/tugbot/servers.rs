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
                diesel::result::Error::QueryBuilderError(Box::new(e))
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

                        // Wrap blocking DB call in spawn_blocking
                        let pool_clone = pool.clone();
                        let create_result = tokio::task::spawn_blocking(move || {
                            create_server(&pool_clone, guild_id_i64, role_id_i64)
                        })
                        .await;

                        match create_result {
                            Ok(Ok(_)) => {
                                serverss.push(Servers {
                                    guild_id: guild_info.id,
                                    gulag_id: role.id,
                                });
                            }
                            Ok(Err(e)) => {
                                eprintln!("Failed to create server in DB: {}", e);
                            }
                            Err(e) => {
                                eprintln!("Task join error creating server: {}", e);
                            }
                        }
                    }
                }
            }
        } else {
            println!("found in DB");

            for s in results {
                // Safe conversion with overflow check
                let guild_id_u64 = match u64::try_from(s.guild_id) {
                    Ok(gid) => gid,
                    Err(e) => {
                        eprintln!("Guild ID conversion error for server {}: {}", s.id, e);
                        continue;
                    }
                };
                let gulag_id_u64 = match u64::try_from(s.gulag_id) {
                    Ok(rid) => rid,
                    Err(e) => {
                        eprintln!("Gulag ID conversion error for server {}: {}", s.id, e);
                        continue;
                    }
                };

                match ctx.http.get_guild(guild_id_u64.into()).await {
                    Ok(guildid) => {
                        serverss.push(Servers {
                            guild_id: guildid.id,
                            gulag_id: RoleId::new(gulag_id_u64),
                        });
                    }
                    Err(err) => {
                        eprintln!("Couldn't connect to server with guild_id {:?}", err);
                        // Delete server in spawn_blocking to avoid blocking async runtime
                        let pool_clone = pool.clone();
                        let server_id = s.id;
                        tokio::spawn(async move {
                            if let Err(e) = tokio::task::spawn_blocking(move || {
                                match pool_clone.get() {
                                    Ok(mut conn) => {
                                        diesel::delete(servers.filter(id.eq(server_id)))
                                            .execute(&mut conn)
                                            .map_err(|e| {
                                                eprintln!("Failed to delete server {}: {}", server_id, e);
                                            })
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to get DB connection for delete: {}", e);
                                        Ok(0)
                                    }
                                }
                            })
                            .await
                            {
                                eprintln!("Task join error during delete: {}", e);
                            }
                        });
                    }
                }
            }
        }

        serverss
    }
}
