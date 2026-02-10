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

        let mut connection = pool.get().expect("Failed to get database connection from pool");
        let results = servers
            .load::<Server>(&mut connection)
            .expect("Error loading Servers");

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
                        let _s = create_server(
                            pool,
                            guild_info.id.get() as i64,
                            role.id.get() as i64,
                        );
                        serverss.push(Servers {
                            guild_id: guild_info.id,
                            gulag_id: role.id,
                        });
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
                        println!("Couldnt connect to server with guildid {:?}", err);
                        diesel::delete(servers.filter(id.eq(s.id)))
                            .execute(&mut connection)
                            .expect("delete server");
                    }
                }
            }
        }

        serverss
    }
}
