use crate::data;

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALPHABET_LEN: usize = 26;

/// Convert an index into its cell row value.
/// e.g. `0` -> `1`, `1` -> `2`.
pub fn index_to_row(idx: data::IndexType) -> String {
    (idx + 1).to_string()
}

/// Convert a numerical index into its
/// cell column index -- which is alphabetic.
/// e.g. `0` -> `"A"`, `1` -> `"B"`.
pub fn index_to_col(idx: data::IndexType) -> String {
    let idx = idx as usize;
    let scale = idx / ALPHABET_LEN;
    if scale == 0 {
        ALPHABET[idx..idx + 1].to_string()
    } else if scale <= ALPHABET_LEN {
        let second = idx % ALPHABET_LEN;
        let first = ALPHABET[scale - 1..scale].to_string();
        let second = ALPHABET[second..second + 1].to_string();
        format!("{first}{second}")
    } else {
        unreachable!("wasn't expecting an index this big");
    }
}

/// Convert an index into its cell row value.
/// e.g. `1` -> `0`, `2` -> `1`.
pub fn row_to_index(row: data::IndexType) -> Option<data::IndexType> {
    row.checked_sub(1)
}

/// Convert a cell column index -- which is alphabetic --
/// into its numerical index.
/// e.g. `"a"` -> `0`, `"b"` -> `1`.
/// Letters are case insensitive.
pub fn col_to_index(col: impl AsRef<str>) -> Option<data::IndexType> {
    let chars = col.as_ref().chars().collect::<Vec<_>>();
    match chars[..] {
        [c1] => {
            let c1 = c1.to_ascii_uppercase();

            let Some(idx) = ALPHABET.chars().position(|ch| ch == c1) else {
                return None;
            };

            let idx = idx as data::IndexType;
            Some(idx)
        }
        [c1, c0] => {
            let c0 = c0.to_ascii_uppercase();
            let c1 = c1.to_ascii_uppercase();

            let Some(a0) = ALPHABET.chars().position(|ch| ch == c0) else {
                return None;
            };
            let Some(a1) = ALPHABET.chars().position(|ch| ch == c1) else {
                return None;
            };

            let idx = ((a1 + 1) * ALPHABET_LEN + a0) as data::IndexType;
            Some(idx)
        }
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn index_to_col_label() {
        assert_eq!(index_to_col(0), "A".to_string());
        assert_eq!(index_to_col(1), "B".to_string());
        assert_eq!(index_to_col(25), "Z".to_string());
        assert_eq!(index_to_col(26), "AA".to_string());
        assert_eq!(index_to_col(27), "AB".to_string());
        assert_eq!(index_to_col(51), "AZ".to_string());
        assert_eq!(index_to_col(52), "BA".to_string());
        assert_eq!(index_to_col(53), "BB".to_string());
        assert_eq!(index_to_col(675), "YZ".to_string());
        assert_eq!(index_to_col(676), "ZA".to_string());
        assert_eq!(index_to_col(701), "ZZ".to_string());
    }

    #[test]
    fn col_label_to_index() {
        assert_eq!(col_to_index("A"), Some(0));
        assert_eq!(col_to_index("B"), Some(1));
        assert_eq!(col_to_index("Z"), Some(25));
        assert_eq!(col_to_index("AA"), Some(26));
        assert_eq!(col_to_index("AB"), Some(27));
        assert_eq!(col_to_index("AZ"), Some(51));
        assert_eq!(col_to_index("BA"), Some(52));
        assert_eq!(col_to_index("BB"), Some(53));
        assert_eq!(col_to_index("YZ"), Some(675));
        assert_eq!(col_to_index("ZA"), Some(676));
        assert_eq!(col_to_index("ZZ"), Some(701));

        assert_eq!(col_to_index("a"), Some(0));
        assert_eq!(col_to_index("b"), Some(1));
        assert_eq!(col_to_index("z"), Some(25));
        assert_eq!(col_to_index("aa"), Some(26));
        assert_eq!(col_to_index("ab"), Some(27));
        assert_eq!(col_to_index("az"), Some(51));
        assert_eq!(col_to_index("ba"), Some(52));
        assert_eq!(col_to_index("bb"), Some(53));
        assert_eq!(col_to_index("yz"), Some(675));
        assert_eq!(col_to_index("za"), Some(676));
        assert_eq!(col_to_index("zz"), Some(701));
    }
}
