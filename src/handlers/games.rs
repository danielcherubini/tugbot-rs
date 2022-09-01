use serenity::model::prelude::{ReactionType, Role};

pub struct Games {
    pub games: Vec<Game>,
}

#[derive(Default, Clone, Debug)]
pub struct Game {
    pub name: String,
    pub role_name: String,
    pub role_id: u64,
    pub emoji: String,
}

impl Game {
    pub fn new(name: String, role_name: String, emoji: String) -> Self {
        Self {
            name,
            emoji,
            role_name,
            role_id: 0,
        }
    }
}

impl Games {
    pub fn new() -> Self {
        let mut g: Vec<Game> = Vec::new();
        g.push(Game::new(
            "Warzone".to_string(),
            "tag-warzone".to_string(),
            "👹".to_string(),
        ));
        g.push(Game::new(
            "Apex".to_string(),
            "tag-apex".to_string(),
            "👺".to_string(),
        ));
        g.push(Game::new(
            "Doom".to_string(),
            "tag-doom".to_string(),
            "👻".to_string(),
        ));

        Self { games: g }
    }

    pub fn find_role_from_emoji(games: Vec<Game>, emoji: ReactionType) -> Option<Game> {
        let mut found = false;
        let mut found_game: Game = Game::default();
        for game in games {
            if emoji == ReactionType::Unicode(game.emoji.to_owned()) {
                found_game = game.to_owned().clone();
                found = true;
            }
        }
        if found {
            Some(found_game)
        } else {
            None
        }
    }

    pub fn match_roles_and_games(games: Vec<Game>, roles: Vec<Role>) -> Vec<Game> {
        let mut g: Vec<Game> = Vec::new();
        for role in roles {
            for game in games.clone() {
                if role.name == game.role_name {
                    let new_game = Game {
                        name: game.name.to_owned(),
                        role_name: game.role_name.to_owned(),
                        role_id: *role.id.as_u64(),
                        emoji: game.emoji.to_owned(),
                    };
                    g.push(new_game);
                }
            }
        }
        return g;
    }
}
