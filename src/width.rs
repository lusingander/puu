use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaxWidth {
    Fixed(usize),
    Auto,
}

impl FromStr for MaxWidth {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "auto" {
            return Ok(Self::Auto);
        }

        value
            .parse::<usize>()
            .ok()
            .filter(|width| *width >= 2)
            .map(Self::Fixed)
            .ok_or_else(|| "must be 'auto' or an integer of at least 2".to_owned())
    }
}

impl MaxWidth {
    pub fn resolve(self) -> Option<usize> {
        self.resolve_with(|| {
            let stdout = console::Term::stdout();
            if !stdout.is_term() {
                return None;
            }
            stdout
                .size_checked()
                .map(|(_, columns)| usize::from(columns))
        })
    }

    fn resolve_with(self, terminal_width: impl FnOnce() -> Option<usize>) -> Option<usize> {
        match self {
            Self::Fixed(width) => Some(width),
            Self::Auto => terminal_width().filter(|width| *width >= 2),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::MaxWidth;

    #[rstest]
    #[case("auto", MaxWidth::Auto)]
    #[case("2", MaxWidth::Fixed(2))]
    #[case("80", MaxWidth::Fixed(80))]
    fn parses_auto_and_numeric_widths(#[case] value: &str, #[case] expected: MaxWidth) {
        assert_eq!(value.parse::<MaxWidth>(), Ok(expected));
    }

    #[rstest]
    #[case(Some(80), Some(80))]
    #[case(Some(2), Some(2))]
    #[case(None, None)]
    #[case(Some(0), None)]
    #[case(Some(1), None)]
    fn auto_uses_only_a_valid_terminal_width(
        #[case] terminal_width: Option<usize>,
        #[case] expected: Option<usize>,
    ) {
        assert_eq!(MaxWidth::Auto.resolve_with(|| terminal_width), expected);
    }

    #[test]
    fn fixed_width_does_not_query_the_terminal() {
        assert_eq!(
            MaxWidth::Fixed(80).resolve_with(|| panic!("terminal should not be queried")),
            Some(80)
        );
    }
}
