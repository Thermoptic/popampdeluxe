//! Base16 colour schemes for vector skins.
//!
//! A scheme is sixteen colours with fixed roles, `base00` to `base0F`; the
//! forty below are the dark schemes of the Base16 collection of
//! tinted-theming (https://github.com/tinted-theming/schemes, MIT licence),
//! listed by their id in that collection.
//!
//! A scheme recolours a skin without any change to the skin's files. Every
//! colour written in the skin's pictures and colour set is replaced, by role:
//!
//! * `base00` is the background: the darkest parts of the skin, the displays
//!   and the large surfaces, which stay in `base00` to `base03` (`base01`
//!   panels, `base02` raised edges and the picked row, `base03` lines).
//! * `base05` is the text: in the playlist and the library, in the title and
//!   in the clock; `base07` stands out (the playing track, the lightest
//!   edges). `base04` is left for small parts.
//! * The metal is tinted 22 per cent with `base0E`.
//! * The lines of the title bars are `base0E` and the lamps of the buttons
//!   (EQ, PL, Shuffle, Repeat, On, Auto, the play lamp) `base0B`, each from a
//!   dark edge through the accent to a light one, as the skin drew them.
//! * Any other part with colour in it becomes the accent (`base08` to
//!   `base0F`) nearest in hue.
//! * The light buttons stay light: `base05` to `base06`.

/// One Base16 scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheme {
    /// The name the scheme has in the collection, and in the settings.
    pub id: &'static str,
    /// What the player shows.
    pub name: &'static str,
    /// `base00` to `base0F`, as `0xrrggbb`.
    pub colors: [u32; 16],
}

/// The forty dark schemes, in the order the Skin Browser lists them.
pub const SCHEMES: [Scheme; 40] = [
    Scheme { id: "gruvbox-dark-hard", name: "Gruvbox dark, hard", colors: [0x1d2021, 0x3c3836, 0x504945, 0x665c54, 0xbdae93, 0xd5c4a1, 0xebdbb2, 0xfbf1c7, 0xfb4934, 0xfe8019, 0xfabd2f, 0xb8bb26, 0x8ec07c, 0x83a598, 0xd3869b, 0xd65d0e] },
    Scheme { id: "gruvbox-dark-medium", name: "Gruvbox dark, medium", colors: [0x282828, 0x3c3836, 0x504945, 0x665c54, 0xbdae93, 0xd5c4a1, 0xebdbb2, 0xfbf1c7, 0xfb4934, 0xfe8019, 0xfabd2f, 0xb8bb26, 0x8ec07c, 0x83a598, 0xd3869b, 0xd65d0e] },
    Scheme { id: "gruvbox-dark-soft", name: "Gruvbox dark, soft", colors: [0x32302f, 0x3c3836, 0x504945, 0x665c54, 0xbdae93, 0xd5c4a1, 0xebdbb2, 0xfbf1c7, 0xfb4934, 0xfe8019, 0xfabd2f, 0xb8bb26, 0x8ec07c, 0x83a598, 0xd3869b, 0xd65d0e] },
    Scheme { id: "solarized-dark", name: "Solarized Dark", colors: [0x002b36, 0x073642, 0x586e75, 0x657b83, 0x839496, 0x93a1a1, 0xeee8d5, 0xfdf6e3, 0xdc322f, 0xcb4b16, 0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0x6c71c4, 0xd33682] },
    Scheme { id: "nord", name: "Nord", colors: [0x2e3440, 0x3b4252, 0x434c5e, 0x4c566a, 0xd8dee9, 0xe5e9f0, 0xeceff4, 0x8fbcbb, 0xbf616a, 0xd08770, 0xebcb8b, 0xa3be8c, 0x88c0d0, 0x81a1c1, 0xb48ead, 0x5e81ac] },
    Scheme { id: "monokai", name: "Monokai", colors: [0x272822, 0x383830, 0x49483e, 0x75715e, 0xa59f85, 0xf8f8f2, 0xf5f4f1, 0xf9f8f5, 0xf92672, 0xfd971f, 0xf4bf75, 0xa6e22e, 0xa1efe4, 0x66d9ef, 0xae81ff, 0xcc6633] },
    Scheme { id: "tomorrow-night", name: "Tomorrow Night", colors: [0x1d1f21, 0x282a2e, 0x373b41, 0x969896, 0xb4b7b4, 0xc5c8c6, 0xe0e0e0, 0xffffff, 0xcc6666, 0xde935f, 0xf0c674, 0xb5bd68, 0x8abeb7, 0x81a2be, 0xb294bb, 0xa3685a] },
    Scheme { id: "tomorrow-night-eighties", name: "Tomorrow Night Eighties", colors: [0x2d2d2d, 0x393939, 0x515151, 0x999999, 0xb4b7b4, 0xcccccc, 0xe0e0e0, 0xffffff, 0xf2777a, 0xf99157, 0xffcc66, 0x99cc99, 0x66cccc, 0x6699cc, 0xcc99cc, 0xa3685a] },
    Scheme { id: "onedark", name: "One Dark", colors: [0x282c34, 0x353b45, 0x3e4451, 0x545862, 0x565c64, 0xabb2bf, 0xb6bdca, 0xc8ccd4, 0xe06c75, 0xd19a66, 0xe5c07b, 0x98c379, 0x56b6c2, 0x61afef, 0xc678dd, 0xbe5046] },
    Scheme { id: "ocean", name: "Ocean", colors: [0x2b303b, 0x343d46, 0x4f5b66, 0x65737e, 0xa7adba, 0xc0c5ce, 0xdfe1e8, 0xeff1f5, 0xbf616a, 0xd08770, 0xebcb8b, 0xa3be8c, 0x96b5b4, 0x8fa1b3, 0xb48ead, 0xab7967] },
    Scheme { id: "oceanicnext", name: "Oceanic Next", colors: [0x1b2b34, 0x343d46, 0x4f5b66, 0x65737e, 0xa7adba, 0xc0c5ce, 0xcdd3de, 0xd8dee9, 0xec5f67, 0xf99157, 0xfac863, 0x99c794, 0x5fb3b3, 0x6699cc, 0xc594c5, 0xab7967] },
    Scheme { id: "catppuccin-mocha", name: "Catppuccin Mocha", colors: [0x1e1e2e, 0x181825, 0x313244, 0x45475a, 0x585b70, 0xcdd6f4, 0xf5e0dc, 0xb4befe, 0xf38ba8, 0xfab387, 0xf9e2af, 0xa6e3a1, 0x94e2d5, 0x89b4fa, 0xcba6f7, 0xf2cdcd] },
    Scheme { id: "catppuccin-macchiato", name: "Catppuccin Macchiato", colors: [0x24273a, 0x1e2030, 0x363a4f, 0x494d64, 0x5b6078, 0xcad3f5, 0xf4dbd6, 0xb7bdf8, 0xed8796, 0xf5a97f, 0xeed49f, 0xa6da95, 0x8bd5ca, 0x8aadf4, 0xc6a0f6, 0xf0c6c6] },
    Scheme { id: "catppuccin-frappe", name: "Catppuccin Frappe", colors: [0x303446, 0x292c3c, 0x414559, 0x51576d, 0x626880, 0xc6d0f5, 0xf2d5cf, 0xbabbf1, 0xe78284, 0xef9f76, 0xe5c890, 0xa6d189, 0x81c8be, 0x8caaee, 0xca9ee6, 0xeebebe] },
    Scheme { id: "dracula", name: "Dracula", colors: [0x282a36, 0x21222c, 0x44475a, 0x6272a4, 0x9ea8c7, 0xf8f8f2, 0xf8f8f2, 0xffffff, 0xff5555, 0xffb86c, 0xf1fa8c, 0x50fa7b, 0x8be9fd, 0xbd93f9, 0xff79c6, 0x993333] },
    Scheme { id: "default-dark", name: "Default Dark", colors: [0x181818, 0x282828, 0x383838, 0x585858, 0xb8b8b8, 0xd8d8d8, 0xe8e8e8, 0xf8f8f8, 0xab4642, 0xdc9656, 0xf7ca88, 0xa1b56c, 0x86c1b9, 0x7cafc2, 0xba8baf, 0xa16946] },
    Scheme { id: "eighties", name: "Eighties", colors: [0x2d2d2d, 0x393939, 0x515151, 0x747369, 0xa09f93, 0xd3d0c8, 0xe8e6df, 0xf2f0ec, 0xf2777a, 0xf99157, 0xffcc66, 0x99cc99, 0x66cccc, 0x6699cc, 0xcc99cc, 0xd27b53] },
    Scheme { id: "tokyo-night-dark", name: "Tokyo Night Dark", colors: [0x1a1b26, 0x16161e, 0x2f3549, 0x444b6a, 0x787c99, 0xa9b1d6, 0xcbccd1, 0xd5d6db, 0xc0caf5, 0xa9b1d6, 0x0db9d7, 0x9ece6a, 0xb4f9f8, 0x2ac3de, 0xbb9af7, 0xf7768e] },
    Scheme { id: "tokyo-night-storm", name: "Tokyo Night Storm", colors: [0x24283b, 0x16161e, 0x343a52, 0x444b6a, 0x787c99, 0xa9b1d6, 0xcbccd1, 0xd5d6db, 0xc0caf5, 0xa9b1d6, 0x0db9d7, 0x9ece6a, 0xb4f9f8, 0x2ac3de, 0xbb9af7, 0xf7768e] },
    Scheme { id: "rose-pine", name: "Rosé Pine", colors: [0x191724, 0x1f1d2e, 0x26233a, 0x6e6a86, 0x908caa, 0xe0def4, 0xe0def4, 0x524f67, 0xeb6f92, 0xf6c177, 0xebbcba, 0x31748f, 0x9ccfd8, 0xc4a7e7, 0xf6c177, 0x524f67] },
    Scheme { id: "rose-pine-moon", name: "Rosé Pine Moon", colors: [0x232136, 0x2a273f, 0x393552, 0x6e6a86, 0x908caa, 0xe0def4, 0xe0def4, 0x56526e, 0xeb6f92, 0xf6c177, 0xea9a97, 0x3e8fb0, 0x9ccfd8, 0xc4a7e7, 0xf6c177, 0x56526e] },
    Scheme { id: "everforest-dark-hard", name: "Everforest Dark Hard", colors: [0x272e33, 0x2e383c, 0x414b50, 0x859289, 0x9da9a0, 0xd3c6aa, 0xedeada, 0xfffbef, 0xe67e80, 0xe69875, 0xdbbc7f, 0xa7c080, 0x83c092, 0x7fbbb3, 0xd699b6, 0x9da9a0] },
    Scheme { id: "kanagawa", name: "Kanagawa", colors: [0x1f1f28, 0x16161d, 0x223249, 0x54546d, 0x727169, 0xdcd7ba, 0xc8c093, 0x717c7c, 0xc34043, 0xffa066, 0xc0a36e, 0x76946a, 0x6a9589, 0x7e9cd8, 0x957fb8, 0xd27e99] },
    Scheme { id: "github-dark", name: "GitHub Dark", colors: [0x0d1117, 0x151b23, 0x2f3742, 0x656c76, 0x9198a1, 0xd1d7e0, 0xf0f6fc, 0xffffff, 0xff7b72, 0xffa657, 0xd29922, 0x3fb950, 0x39c5cf, 0x58a6ff, 0xbe8fff, 0xffa198] },
    Scheme { id: "ayu-dark", name: "Ayu Dark", colors: [0x0b0e14, 0x131721, 0x202229, 0x3e4b59, 0xbfbdb6, 0xe6e1cf, 0xece8db, 0xf2f0e7, 0xf07178, 0xff8f40, 0xffb454, 0xaad94c, 0x95e6cb, 0x59c2ff, 0xd2a6ff, 0xe6b450] },
    Scheme { id: "ayu-mirage", name: "Ayu Mirage", colors: [0x1f2430, 0x242936, 0x323844, 0x4a5059, 0x707a8c, 0xcccac2, 0xd9d7ce, 0xf3f4f5, 0xf28779, 0xffad66, 0xffd173, 0xd5ff80, 0x95e6cb, 0x73d0ff, 0xd4bfff, 0xf27983] },
    Scheme { id: "material-darker", name: "Material Darker", colors: [0x212121, 0x303030, 0x353535, 0x4a4a4a, 0xb2ccd6, 0xeeffff, 0xeeffff, 0xffffff, 0xf07178, 0xf78c6c, 0xffcb6b, 0xc3e88d, 0x89ddff, 0x82aaff, 0xc792ea, 0xff5370] },
    Scheme { id: "material-palenight", name: "Material Palenight", colors: [0x292d3e, 0x444267, 0x32374d, 0x676e95, 0x8796b0, 0x959dcb, 0x959dcb, 0xffffff, 0xf07178, 0xf78c6c, 0xffcb6b, 0xc3e88d, 0x89ddff, 0x82aaff, 0xc792ea, 0xff5370] },
    Scheme { id: "zenburn", name: "Zenburn", colors: [0x383838, 0x404040, 0x606060, 0x6f6f6f, 0x808080, 0xdcdccc, 0xc0c0c0, 0xffffff, 0xdca3a3, 0xdfaf8f, 0xe0cf9f, 0x5f7f5f, 0x93e0e3, 0x7cb8bb, 0xdc8cc3, 0x000000] },
    Scheme { id: "railscasts", name: "Railscasts", colors: [0x2b2b2b, 0x272935, 0x3a4055, 0x5a647e, 0xd4cfc9, 0xe6e1dc, 0xf4f1ed, 0xf9f7f3, 0xda4939, 0xcc7833, 0xffc66d, 0xa5c261, 0x519f50, 0x6d9cbe, 0xb6b3eb, 0xbc9458] },
    Scheme { id: "snazzy", name: "Snazzy", colors: [0x282a36, 0x34353e, 0x43454f, 0x78787e, 0xa5a5a9, 0xe2e4e5, 0xeff0eb, 0xf1f1f0, 0xff5c57, 0xff9f43, 0xf3f99d, 0x5af78e, 0x9aedfe, 0x57c7ff, 0xff6ac1, 0xb2643c] },
    Scheme { id: "synth-midnight-dark", name: "Synth Midnight Terminal Dark", colors: [0x050608, 0x1a1b1c, 0x28292a, 0x474849, 0xa3a5a6, 0xc1c3c4, 0xcfd1d2, 0xdddfe0, 0xb53b50, 0xea770d, 0xc9d364, 0x06ea61, 0x42fff9, 0x03aeff, 0xea5ce2, 0xcd6320] },
    Scheme { id: "tender", name: "Tender", colors: [0x282828, 0x383838, 0x484848, 0x4c4c4c, 0xb8b8b8, 0xeeeeee, 0xe8e8e8, 0xfeffff, 0xf43753, 0xdc9656, 0xffc24b, 0xc9d05c, 0x73cef4, 0xb3deef, 0xd3b987, 0xa16946] },
    Scheme { id: "twilight", name: "Twilight", colors: [0x1e1e1e, 0x323537, 0x464b50, 0x5f5a60, 0x838184, 0xa7a7a7, 0xc3c3c3, 0xffffff, 0xcf6a4c, 0xcda869, 0xf9ee98, 0x8f9d6a, 0xafc4db, 0x7587a6, 0x9b859d, 0x9b703f] },
    Scheme { id: "spacemacs", name: "Spacemacs", colors: [0x1f2022, 0x282828, 0x444155, 0x585858, 0xb8b8b8, 0xa3a3a3, 0xe8e8e8, 0xf8f8f8, 0xf2241f, 0xffa500, 0xb1951d, 0x67b11d, 0x2d9574, 0x4f97d7, 0xa31db1, 0xb03060] },
    Scheme { id: "nightfox", name: "Nightfox", colors: [0x192330, 0x212e3f, 0x29394f, 0x575860, 0x71839b, 0xcdcecf, 0xaeafb0, 0xe4e4e5, 0xc94f6d, 0xf4a261, 0xdbc074, 0x81b29a, 0x63cdcf, 0x719cd6, 0x9d79d6, 0xd67ad2] },
    Scheme { id: "horizon-dark", name: "Horizon Dark", colors: [0x1c1e26, 0x232530, 0x2e303e, 0x6f6f70, 0x9da0a2, 0xcbced0, 0xdcdfe4, 0xe3e6ee, 0xe93c58, 0xe58d7d, 0xefb993, 0xefaf8e, 0x24a8b4, 0xdf5273, 0xb072d1, 0xe4a382] },
    Scheme { id: "atelier-forest", name: "Atelier Forest", colors: [0x1b1918, 0x2c2421, 0x68615e, 0x766e6b, 0x9c9491, 0xa8a19f, 0xe6e2e0, 0xf1efee, 0xf22c40, 0xdf5320, 0xc38418, 0x7b9726, 0x3d97b8, 0x407ee7, 0x6666ea, 0xc33ff3] },
    Scheme { id: "darcula", name: "Darcula", colors: [0x2b2b2b, 0x323232, 0x323232, 0x606366, 0xa4a3a3, 0xa9b7c6, 0xffc66d, 0xffffff, 0x4eade5, 0x689757, 0xbbb529, 0x6a8759, 0x629755, 0x9876aa, 0xcc7832, 0x808080] },
    Scheme { id: "gotham", name: "Gotham", colors: [0x0c1014, 0x11151c, 0x091f2e, 0x0a3749, 0x245361, 0x599cab, 0x99d1ce, 0xd3ebe9, 0xc23127, 0xd26937, 0xedb443, 0x33859e, 0x2aa889, 0x195466, 0x888ca6, 0x4e5166] },
];

/// The scheme with this id.
pub fn find(id: &str) -> Option<&'static Scheme> {
    SCHEMES.iter().find(|s| s.id == id)
}

type Rgb = [f32; 3];

fn channels(c: u32) -> Rgb {
    [((c >> 16) & 0xff) as f32, ((c >> 8) & 0xff) as f32, (c & 0xff) as f32]
}

fn pack(c: Rgb) -> u32 {
    let b = |v: f32| v.round().clamp(0.0, 255.0) as u32;
    (b(c[0]) << 16) | (b(c[1]) << 8) | b(c[2])
}

/// Lightness from 0 to 1 as a viewer sees it (weights of red, green, blue).
pub fn luma(c: u32) -> f32 {
    let [r, g, b] = channels(c);
    (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Hue (degrees), saturation and lightness (each 0 to 1).
fn to_hsl(c: Rgb) -> [f32; 3] {
    let (r, g, b) = (c[0] / 255.0, c[1] / 255.0, c[2] / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let (l, d) = ((mx + mn) / 2.0, mx - mn);
    if d <= f32::EPSILON {
        return [0.0, 0.0, l];
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if mx == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    [h * 60.0, s, l]
}

fn from_hsl([h, s, l]: [f32; 3]) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

/// Hue (degrees), strength and brightness (each 0 to 1) as a painter sees
/// colour: a pale cream has little strength, a gold edge much.
fn to_hsv(c: Rgb) -> [f32; 3] {
    let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    [to_hsl(c)[0], if mx > 0.0 { (mx - mn) / mx } else { 0.0 }, mx / 255.0]
}

/// Where each lightness of the skin lands on the neutral colours, as places
/// from `base00` (0) to `base07` (7): the skin's large surfaces are the dark
/// ones, and only its light buttons and edges go up to `base05`.
const ANCHORS: [(f32, f32); 8] = [(0.0, 0.0), (0.22, 0.15), (0.4, 0.9), (0.55, 1.7), (0.68, 2.6), (0.8, 3.0), (0.9, 5.0), (1.0, 6.6)];

/// How much of `base0E` the neutral colours are tinted with.
const TINT: f32 = 0.22;

fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

impl Scheme {
    /// `base00` to `base07` as a ramp. `base04` is left out (it is the
    /// colour of small status parts, and some schemes make it an accent) and
    /// a `base07` darker than `base06` (an accent there) is not used.
    fn ramp(&self) -> [Rgb; 8] {
        let c: [Rgb; 8] = std::array::from_fn(|i| channels(self.colors[i]));
        let top = if luma(self.colors[7]) < luma(self.colors[6]) { c[6] } else { c[7] };
        [c[0], c[1], c[2], c[3], mix(c[3], c[5], 0.5), c[5], c[6], top]
    }

    /// The accent (`base08`..`base0F`) whose hue is nearest `hue`, in degrees.
    fn accent_near(&self, hue: f32) -> u32 {
        let away = |c: u32| hue_gap(to_hsl(channels(c))[0], hue);
        self.colors[8..].iter().copied().min_by(|a, b| away(*a).total_cmp(&away(*b))).unwrap_or(self.colors[13])
    }

    /// The colour of text: `base05`.
    pub fn text(&self) -> u32 {
        self.colors[5]
    }

    /// The colour of text that stands out, the playing track: the scheme's lightest.
    pub fn bright(&self) -> u32 {
        pack(self.ramp()[7])
    }

    /// One accent from a dark edge (`base00`) through itself, at brightness
    /// `reference`, to a light edge (`base07`), as the part of the skin drawn
    /// with brightness `v` was shaded.
    fn shaded(&self, accent: u32, v: f32, reference: f32) -> u32 {
        let a = channels(accent);
        let out = if v >= reference {
            mix(a, self.ramp()[7], ((v - reference) / (1.0 - reference + 1e-6) * 0.55).min(1.0))
        } else {
            mix(channels(self.colors[0]), a, (v / reference).powf(0.9))
        };
        pack(out)
    }

    /// The colour that replaces `old`.
    pub fn map(&self, old: u32) -> u32 {
        let rgb = channels(old);
        let [hue, strength, v] = to_hsv(rgb);
        // The gold lines of a title bar and the green of a lamp have accents of their own.
        if (25.0..=65.0).contains(&hue) && strength > 0.25 {
            return self.shaded(self.colors[14], v, 0.83);
        }
        if (85.0..=155.0).contains(&hue) && strength > 0.3 {
            return self.shaded(self.colors[11], v, 1.0);
        }
        // The neutral colour for this lightness.
        let y = luma(old);
        let ramp = self.ramp();
        let at = ANCHORS.windows(2).find(|w| y <= w[1].0).map_or(7.0, |w| w[0].1 + (w[1].1 - w[0].1) * (y - w[0].0) / (w[1].0 - w[0].0));
        let i = (at as usize).min(6);
        let mut out = mix(ramp[i], ramp[i + 1], (at - i as f32).min(1.0));
        // The metal is tinted with an accent.
        let [h, s, _] = to_hsl(channels(self.colors[14]));
        out = mix(out, from_hsl([h, s * 0.7, to_hsl(out)[2]]), TINT);
        // A part with colour in it becomes the accent nearest in hue, as light or dark as it was.
        let weight = ((strength - 0.08) * 3.5).clamp(0.0, 1.0);
        if weight > 0.0 {
            let [h, s, l] = to_hsl(channels(self.accent_near(hue)));
            let lightness = (l + (to_hsl(rgb)[2] - 0.5) * 0.5).clamp(0.12, 0.88);
            out = mix(out, from_hsl([h, s, lightness]), weight);
        }
        pack(out)
    }

    /// `svg` with every colour written in it replaced (`ink`: a sheet of
    /// letters or digits, whose light colour becomes the scheme's text
    /// colour). A colour in a `url(#id)` is an id, and is left.
    pub fn recolour_svg(&self, svg: &str, ink: bool) -> String {
        let bytes = svg.as_bytes();
        let mut out = String::with_capacity(svg.len());
        let (mut i, mut last) = (0, 0);
        while i < bytes.len() {
            let after_a_value = i > 0 && matches!(bytes[i - 1], b'"' | b'\'' | b':' | b' ' | b';');
            if bytes[i] == b'#' && after_a_value {
                let digits = bytes[i + 1..].iter().take_while(|b| b.is_ascii_hexdigit()).count();
                if digits == 6 || digits == 3 {
                    let word = &svg[i + 1..i + 1 + digits];
                    let value = u32::from_str_radix(word, 16).unwrap_or(0);
                    let old = if digits == 6 {
                        value
                    } else {
                        let (r, g, b) = ((value >> 8) & 0xf, (value >> 4) & 0xf, value & 0xf);
                        ((r * 17) << 16) | ((g * 17) << 8) | (b * 17)
                    };
                    let new = if ink && luma(old) > 0.5 { self.text() } else { self.map(old) };
                    out.push_str(&svg[last..i]);
                    out.push_str(&format!("#{new:06x}"));
                    i += 1 + digits;
                    last = i;
                    continue;
                }
            }
            i += 1;
        }
        out.push_str(&svg[last..]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRACULA: &str = "dracula";

    #[test]
    fn there_are_forty_dark_schemes_with_different_ids_and_names() {
        assert_eq!(SCHEMES.len(), 40);
        let mut ids: Vec<_> = SCHEMES.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 40);
        let mut names: Vec<_> = SCHEMES.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 40);
        // Dark: the background is darker than the text.
        for s in &SCHEMES {
            assert!(luma(s.colors[0]) < luma(s.colors[5]), "{} is dark", s.name);
        }
        assert_eq!(find("nord").map(|s| s.name), Some("Nord"));
        assert_eq!(find("nord").map(|s| s.colors[0]), Some(0x2e3440));
        assert!(find("gone").is_none());
    }

    #[test]
    fn black_is_the_background_and_the_big_dark_surfaces_stay_dark() {
        for s in &SCHEMES {
            assert!((luma(s.map(0x000000)) - luma(s.colors[0])).abs() < 0.06, "{}: black is the background", s.name);
            // The metal of the skin (a mid grey) is darker than base04: the window is not light.
            let metal = s.map(0x6e6e6e);
            assert!(luma(metal) < luma(s.colors[4]).max(luma(s.colors[3]) + 0.05), "{}: {metal:06x}", s.name);
        }
        let dracula = find(DRACULA).unwrap();
        assert!(luma(dracula.map(0x6e6e6e)) < 0.35, "a mid grey is dark in Dracula, not light blue");
    }

    #[test]
    fn the_light_buttons_stay_light_and_neutral() {
        let s = find(DRACULA).unwrap();
        let cream = s.map(0xfaefe3);
        assert!(luma(cream) > 0.7, "{cream:06x}");
        let [_, strength, _] = to_hsv(channels(cream));
        assert!(strength < 0.25, "a cream is not made into an accent: {cream:06x}");
    }

    #[test]
    fn the_lines_of_the_title_bar_are_base0e_and_the_lamps_base0b() {
        let s = find(DRACULA).unwrap();
        // The skin's gold, at the brightness it has in the middle of a line, and its bright green.
        let near = |a: u32, b: u32| channels(a).iter().zip(channels(b)).all(|(x, y)| (x - y).abs() <= 3.0);
        assert!(near(s.map(0xd3b069), s.colors[14]), "gold lines are base0e: {:06x}", s.map(0xd3b069));
        assert_eq!(s.map(0x00ff00), s.colors[11], "a lit lamp is base0b");
        // From a dark edge to a light one.
        assert!(luma(s.map(0x2b2415)) < luma(s.map(0xd3b069)) && luma(s.map(0xd3b069)) < luma(s.map(0xffffac)));
        assert!(luma(s.map(0x162b14)) < luma(s.map(0x008600)), "an unlit lamp is darker than a lit one");
    }

    #[test]
    fn a_part_with_other_colour_goes_to_the_accent_nearest_in_hue() {
        let s = find("gruvbox-dark-hard").unwrap();
        let hue = |c: u32| to_hsl(channels(c))[0];
        let red = s.map(0xff0000);
        let blue = s.map(0x0000ff);
        assert!(hue_gap(hue(red), hue(s.colors[8])) < 20.0, "red is near base08: {red:06x}");
        assert!(hue_gap(hue(blue), hue(s.colors[13])) < 40.0, "blue is near base0d: {blue:06x}");
        assert_ne!(red, blue);
    }

    #[test]
    fn text_is_base05_and_what_stands_out_the_lightest() {
        let s = find(DRACULA).unwrap();
        assert_eq!((s.text(), s.bright()), (0xf8f8f2, 0xffffff));
        // Nord has an accent in base07: the lightest used is base06.
        let nord = find("nord").unwrap();
        assert_eq!(nord.bright(), nord.colors[6]);
    }

    #[test]
    fn colours_in_a_picture_are_replaced_but_ids_are_not() {
        let nord = find("nord").unwrap();
        let svg = r##"<svg><rect fill="#000000" style="stroke:#ffffff"/><use href="url(#abc123)"/><stop stop-color='#fff'/></svg>"##;
        let out = nord.recolour_svg(svg, false);
        assert!(out.contains(&format!("fill=\"#{:06x}\"", nord.map(0))), "{out}");
        assert!(out.contains("url(#abc123)"), "an id that looks like a colour stays");
        assert!(!out.contains("#ffffff") && !out.contains("'#fff'"), "{out}");
        // A sheet of letters: the light colour is the scheme's text colour.
        let letters = nord.recolour_svg(r##"<path fill="#ffffff"/><path fill="#000000"/>"##, true);
        assert!(letters.contains(&format!("#{:06x}", nord.text())) && letters.contains(&format!("#{:06x}", nord.map(0))), "{letters}");
    }
}
