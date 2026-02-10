fn clean_username(nick: &str) -> String {
    nick.replace("phony | ", "").replace("horny | ", "")
}

// fix_nickname is a function to add the nickname for horny/phony
pub fn fix_nickname(nick: &str, prefix: &str) -> String {
    // check if the nickname has the prefix in it
    let nick_to_find = format!("{} | ", prefix);
    if nick.contains(&nick_to_find) {
        // the prefix is already in the nick so just clean
        clean_username(nick)
    } else if nick.contains(" | ") {
        // the prefix doesn't match, but there's a pipe in there
        format!("{} | {}", prefix, clean_username(nick))
    } else {
        format!("{} | {}", prefix, nick)
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

    #[test]
    fn empty_nickname() {
        let nick = String::from("");
        let prefix = String::from("horny");
        let result = super::fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("horny | "));
    }

    #[test]
    fn clean_username_removes_both_prefixes() {
        let nick = String::from("phony | horny | username");
        let result = super::clean_username(&nick);
        assert_eq!(result, String::from("username"));
    }

    #[test]
    fn clean_username_no_prefix() {
        let nick = String::from("username");
        let result = super::clean_username(&nick);
        assert_eq!(result, String::from("username"));
    }

    #[test]
    fn nickname_with_multiple_pipes() {
        let nick = String::from("other | prefix | username");
        let prefix = String::from("horny");
        let result = super::fix_nickname(&nick, &prefix);
        assert_eq!(result, String::from("horny | other | prefix | username"));
    }

    #[test]
    fn nickname_already_has_correct_prefix() {
        let nick = String::from("phony | username");
        let prefix = String::from("phony");
        let result = super::fix_nickname(&nick, &prefix);
        // Should clean it (toggle off)
        assert_eq!(result, String::from("username"));
    }
}
