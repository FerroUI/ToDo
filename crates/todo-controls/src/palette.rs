//! App-specific tokens; system controls continue to use the underlying WinUI theme.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: &'static str,
    pub sidebar: &'static str,
    pub surface: &'static str,
    pub surface_hover: &'static str,
    pub text: &'static str,
    pub secondary: &'static str,
    pub border: &'static str,
    pub accent: &'static str,
    pub selection: &'static str,
}

impl Palette {
    pub const fn light() -> Self {
        Self {
            background: "#F5F5F5",
            sidebar: "#FAFAFA",
            surface: "#FFFFFF",
            surface_hover: "#F3F3F3",
            text: "#292827",
            secondary: "#616161",
            border: "#E5E5E5",
            accent: "#2564CF",
            selection: "#E7EFFB",
        }
    }

    pub const fn dark() -> Self {
        Self {
            background: "#1F1F1F",
            sidebar: "#202020",
            surface: "#292929",
            surface_hover: "#353535",
            text: "#F5F5F5",
            secondary: "#C1C1C1",
            border: "#414141",
            accent: "#8AB4F8",
            selection: "#243851",
        }
    }

    pub const fn for_dark(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListTheme {
    pub name: &'static str,
    pub accent: &'static str,
    pub background: &'static str,
    pub dark_background: &'static str,
}

impl ListTheme {
    pub fn from_index(index: usize) -> Self {
        ALL_LIST_THEMES[index % ALL_LIST_THEMES.len()]
    }
}

pub const ALL_LIST_THEMES: [ListTheme; 8] = [
    ListTheme {
        name: "Blue",
        accent: "#2564CF",
        background: "#EDF3FC",
        dark_background: "#17293E",
    },
    ListTheme {
        name: "Lavender",
        accent: "#6B5BB5",
        background: "#F0ECF9",
        dark_background: "#2A233C",
    },
    ListTheme {
        name: "Rose",
        accent: "#B94E70",
        background: "#FAEDF2",
        dark_background: "#38232C",
    },
    ListTheme {
        name: "Peach",
        accent: "#B56B2E",
        background: "#FCF0E6",
        dark_background: "#382B20",
    },
    ListTheme {
        name: "Green",
        accent: "#277E59",
        background: "#EAF4EF",
        dark_background: "#1C3028",
    },
    ListTheme {
        name: "Teal",
        accent: "#25828B",
        background: "#E9F3F4",
        dark_background: "#1A3034",
    },
    ListTheme {
        name: "Indigo",
        accent: "#5868B9",
        background: "#EEF0FA",
        dark_background: "#232A43",
    },
    ListTheme {
        name: "Charcoal",
        accent: "#5C626D",
        background: "#F0F1F3",
        dark_background: "#282B30",
    },
];
