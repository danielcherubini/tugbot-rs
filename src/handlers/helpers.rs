// fix_nickname is a function to add the nickname for horny/phony
pub fn fix_nickname(nick: &String, prefix: &String) -> String {
    let string_to_find = format!("{} | ", prefix);

    // check if the nickname has the prefix in it
    if let Some(_result) = nick.find(&string_to_find) {
        return nick.replace(&string_to_find, "");
    } else {
        return format!("{} | {}", prefix, nick);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nickname_addition_positive() {
        let nick = String::from("foo");
        let prefix = String::from("horny");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, format!("{} | {}", prefix, nick));
    }
    #[test]
    fn nickname_clean_positive() {
        let nick = String::from("horny | foo");
        let prefix = String::from("horny");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("foo"));
    }
    #[test]
    fn clean_dumb() {
        let nick = String::from("phony | horny | foo");
        let prefix = String::from("phony");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("horny | foo"));
    }
    #[test]
    fn clean_dumb_2() {
        let nick = String::from("horny | phony | foo");
        let prefix = String::from("phony");
        let positive_test = super::fix_nickname(&nick, &prefix);
        assert_eq!(positive_test, String::from("horny | foo"));
    }
}
