use serenity::{
    client::Context,
    http::GuildPagination,
    model::id::{GuildId, RoleId},
};

pub struct Server {
    pub guild_id: GuildId,
    pub gulag_id: RoleId,
}

impl Server {
    pub async fn get_servers(ctx: &Context) -> Vec<Server> {
        let mut servers = Vec::new();

        let guild_id = GuildId(0);
        let guilds = ctx
            .http
            .get_guilds(Some(&GuildPagination::After(guild_id)), Some(10))
            .await
            .unwrap();

        for guild_info in guilds {
            let id64: u64 = u64::from(guild_info.id);
            let roles = ctx.http.get_guild_roles(id64).await.unwrap();

            for role in roles {
                if role.name == "gulag" {
                    servers.push(Server {
                        guild_id: guild_info.id,
                        gulag_id: role.id,
                    });
                }
            }
        }

        return servers;
    }
}
