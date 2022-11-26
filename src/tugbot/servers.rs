use diesel::*;
use serenity::{
    client::Context,
    http::GuildPagination,
    model::id::{GuildId, RoleId},
    prelude::TypeMapKey,
};

use crate::db::{create_server, schema::servers::dsl::*};
use crate::db::{establish_connection, models::Server};

pub struct PostgresClient;
impl TypeMapKey for PostgresClient {
    type Value = diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>;
}

pub struct Servers {
    pub guild_id: GuildId,
    pub gulag_id: RoleId,
}

impl Servers {
    pub async fn get_servers(ctx: &Context) -> Vec<Servers> {
        let mut serverss = Vec::new();

        let connection = &mut establish_connection();
        let results = servers
            .load::<Server>(connection)
            .expect("Error loading Servers");

        if results.len() == 0 {
            println!("Nothing found in DB");
            let current_guild_id = GuildId(0);
            let guilds = ctx
                .http
                .get_guilds(Some(&GuildPagination::After(current_guild_id)), Some(10))
                .await
                .unwrap();

            for guild_info in guilds {
                let id64: u64 = u64::from(guild_info.id);
                let roles = ctx.http.get_guild_roles(id64).await.unwrap();

                for role in roles {
                    if role.name == "gulag" {
                        create_server(
                            connection,
                            *guild_info.id.as_u64() as i64,
                            *role.id.as_u64() as i64,
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
                serverss.push(Servers {
                    guild_id: GuildId(s.guild_id as u64),
                    gulag_id: RoleId(s.gulag_id as u64),
                })
            }
        }

        return serverss;
    }
}