fn clean_username(nick: &str) -> String {
    nick.replace("phony | ", "").replace("horny | ", "")
}

/// Adds or removes the horny/phony prefix from a nickname.
/// If the prefix is already present, it's removed (toggle off).
/// If a different prefix is present, it's swapped.
/// Otherwise the prefix is prepended.
pub fn fix_nickname(nick: &str, prefix: &str) -> String {
    let nick_to_find = format!("{} | ", prefix);
    if nick.contains(&nick_to_find) {
        clean_username(nick)
    } else if nick.contains(" | ") {
        format!("{} | {}", prefix, clean_username(nick))
    } else {
        format!("{} | {}", prefix, nick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horny() {
        assert_eq!(fix_nickname("foo", "horny"), "horny | foo");
    }

    #[test]
    fn phony() {
        assert_eq!(fix_nickname("foo", "phony"), "phony | foo");
    }

    #[test]
    fn swap() {
        assert_eq!(fix_nickname("horny | foo", "phony"), "phony | foo");
    }

    #[test]
    fn toggle_off() {
        assert_eq!(fix_nickname("horny | foo", "horny"), "foo");
    }

    #[test]
    fn clean_all_prefixes() {
        assert_eq!(fix_nickname("phony | horny | foo", "phony"), "foo");
    }

    #[test]
    fn empty_nickname() {
        assert_eq!(fix_nickname("", "horny"), "horny | ");
    }

    #[test]
    fn clean_username_removes_both() {
        assert_eq!(clean_username("phony | horny | username"), "username");
    }

    #[test]
    fn clean_username_no_prefix() {
        assert_eq!(clean_username("username"), "username");
    }

    #[test]
    fn multiple_pipes() {
        assert_eq!(
            fix_nickname("other | prefix | username", "horny"),
            "horny | other | prefix | username"
        );
    }

    #[test]
    fn already_correct_prefix_toggles_off() {
        assert_eq!(fix_nickname("phony | username", "phony"), "username");
    }
}
