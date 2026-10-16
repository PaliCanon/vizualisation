//! The arrow keys in a list of options — the search's results, a chip's choices: one helper
//! for both (`Menu.tsx`'s keys, lines 50–52), so the two lists move alike.

/// The row the arrow `key` moves to from row `active` of a list of `len`: down or up,
/// wrapping around at either end; `None` for another key, or for an empty list. An `active`
/// past the end counts as the last row.
pub fn list_move(key: &str, active: usize, len: usize) -> Option<usize> {
    let last = len.checked_sub(1)?;
    let active = active.min(last);
    match key {
        "ArrowDown" => Some(if active == last { 0 } else { active + 1 }),
        "ArrowUp" => Some(if active == 0 { last } else { active - 1 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrows_move_and_wrap_around() {
        // Menu.tsx:51 `(a + (e.key === "ArrowDown" ? 1 : options.length - 1)) % options.length`
        assert_eq!(list_move("ArrowDown", 0, 3), Some(1));
        assert_eq!(
            list_move("ArrowDown", 2, 3),
            Some(0),
            "past the last: the first"
        );
        assert_eq!(list_move("ArrowUp", 1, 3), Some(0));
        assert_eq!(
            list_move("ArrowUp", 0, 3),
            Some(2),
            "before the first: the last"
        );
        assert_eq!(list_move("ArrowUp", 0, 1), Some(0));
        assert_eq!(list_move("ArrowDown", 0, 1), Some(0));
    }

    #[test]
    fn an_empty_list_or_another_key_moves_nothing() {
        assert_eq!(list_move("ArrowDown", 0, 0), None);
        assert_eq!(list_move("ArrowUp", 0, 0), None, "no underflow");
        assert_eq!(list_move("ArrowUp", 5, 0), None);
        assert_eq!(list_move("Enter", 0, 3), None);
        assert_eq!(list_move("Escape", 0, 3), None);
        // A row past the end (the list shrank): from the last.
        assert_eq!(list_move("ArrowDown", 9, 3), Some(0));
        assert_eq!(list_move("ArrowUp", 9, 3), Some(1));
    }
}
