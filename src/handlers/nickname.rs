fn clean_username(nick: &String) -> String {
    nick.replace("phony | ", "").replace("horny | ", "")
}

// fix_nickname is a function to add the nickname for horny/phony
pub fn fix_nickname(nick: &String, prefix: &String) -> String {
    // check if the nickname has the prefix in it
    let nick_to_find = format!("{} | ", prefix);
    if nick.contains(&nick_to_find) {
        // the prefix is already in the nick so just clean
        return clean_username(nick);
    } else if nick.contains(" | ") {
        // the prefix doesn't match, but there's a pipe in there
        return format!("{} | {}", prefix, clean_username(nick));
    } else {
        return format!("{} | {}", prefix, nick);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn horny() {
        let nick = String::from("foo");
        let prefix = String::from("horny");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("horny | foo"));
    }
    #[test]
    fn phony() {
        let nick = String::from("foo");
        let prefix = String::from("phony");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("phony | foo"));
    }
    #[test]
    fn swap() {
        let nick = String::from("horny | foo");
        let prefix = String::from("phony");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("phony | foo"));
    }

    #[test]
    fn nickname_clean_one() {
        let nick = String::from("horny | foo");
        let prefix = String::from("horny");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }
    #[test]
    fn nickname_clean_all() {
        let nick = String::from("phony | horny | foo");
        let prefix = String::from("phony");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }
}
