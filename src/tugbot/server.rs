use serenity::{
    client::Context,
    http::GuildPagination,
    model::{
        id::{GuildId, RoleId},
        prelude::ChannelId,
    },
};

pub struct Server {
    pub guild_id: GuildId,
    pub gulag_id: RoleId,
    pub elon_id: RoleId,
    pub get_roles_channel_id: ChannelId,
}

impl Server {
    pub async fn get_servers(ctx: &Context) -> Vec<Server> {
        let mut servers = Vec::new();

        let guild_id = GuildId::new(1);
        let guilds = ctx
            .http
            .get_guilds(Some(&GuildPagination::After(guild_id)), Some(10))
            .await
            .unwrap();

        for guild_info in guilds {
            let id64: u64 = u64::from(guild_info.id);
            let roles = ctx.http.get_guild_roles(id64).await.unwrap();
            let channels = ctx.http.get_channels(id64).await.unwrap();

            let mut get_roles_cid = ChannelId::default();

            for channel in channels {
                if channel.name == "get-roles" {
                    get_roles_cid = channel.id;
                }
            }

            let server = Server {
                guild_id: guild_info.id,
                get_roles_channel_id: get_roles_cid,
            };

            for role in roles {
                match role.name {
                    "gulag" => server.gulag_id = role.id,
                    "#1ElonMuskFan" => server.elon_id = role.id,
                }
            }

            servers.push(server);
        }

        return servers;
    }
}
