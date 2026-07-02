use std::{fmt::Display, path::PathBuf};

use serde::{Serialize, Deserialize};

use crate::engine::terminal::print_terminal::{LogType, print_logs};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)] 
pub struct NESColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub enum PaletteTheme {
    #[default]
    DefaultNtsc,
    SonyCxa,
    Fceux,
    InvertedNtsc,
    Nestopia,
    ShovelKnight,
    Custom(String, Vec<NESColor>), 
}
impl PaletteTheme {
    /// Returns a reference to the current color theme.
    /// 
    /// That being an array of arranged RGB values: **`[[u8; 3]; 64]`**.
    /// 
    /// The theme can be easily cycled to the next one via [`PaletteTheme::cycle_palettes()`].
    pub fn get_collors(&self) -> &[NESColor; 64] {
        match self {
            Self::DefaultNtsc   => &NTSC_DEFAULT_PALETTE,
            Self::Nestopia      => &NESTOPIA_PALETTE,
            Self::SonyCxa       => &SONY_CXA_PALETTE,
            Self::Fceux         => &FCEUX_PALETTE,
            Self::InvertedNtsc  => &NTSC_INVERTED_PALETTE,
            Self::ShovelKnight  => &SHOVEL_KNIGHT_NES_PALETTE,
            Self::Custom(_, custom_colors) => custom_colors
                .as_slice()
                .try_into()
                .expect("The palette must have exactly 64 colors")
        }
    }
    pub fn create_palette_from_dot_pal(path: &PathBuf) -> Result<[NESColor; 64], PaletteError> {
        let bytes = std::fs::read(path)?;
        if let Ok(text) = String::from_utf8(bytes.clone()) {
            if text.starts_with("JASC-PAL") {
                let mut new_colors = [NESColor { r: 0, g: 0, b: 0 }; 64];
                let mut lines = text.lines();

                //see the number of colors, if different than 64 
                //it warns the user that the palette may not be for the nes 
                //since it doenst have all the colors
                let n_of_colors = lines.nth(2).unwrap_or("0").parse::<u32>().unwrap_or(0);
                if n_of_colors != 64 {
                    print_logs(LogType::Warning, 
                    format!(
                        "The given palette has {} colors. \
                        64 colors are expected for the NES. \
                        Make sure this is intended to be used as a NES palette", 
                        n_of_colors
                    ));
                }
                let mut lines = lines.skip(3);
                
                // back to the palette formating
                for i in 0..64 {
                    if let Some(line) = lines.next() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() == 3 {
                            if let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse(), parts[1].parse(), parts[2].parse()) {
                                new_colors[i] = NESColor { r, g, b };
                            }
                        }
                    }
                }
                return Ok(new_colors);
            }
        }
        if bytes.len() == 192 {
            let mut new_colors = [ NESColor{ r: 0, g: 0, b: 0}; 64 ];
            for i in 0..64 {
                new_colors[i] = NESColor { 
                    r: bytes[i * 3], 
                    g: bytes[i * 3 + 1], 
                    b: bytes[i * 3 + 2]
                }
            }
            Ok(new_colors)
        } else {
            Err(PaletteError::InvalidFormat(format!("Invalid file size (Expected 192bytes, found: {})", bytes.len())))
        }
    }
}

#[derive(Debug)]
pub enum PaletteError {
    Io(std::io::Error),
    InvalidFormat(String),
}
impl From<std::io::Error> for PaletteError {
    fn from(err: std::io::Error) -> Self {
        PaletteError::Io(err)
    }
}
impl Display for PaletteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaletteError::Io(err) => write!(f, "IO Error: {}", err),
            PaletteError::InvalidFormat(msg) => write!(f, "Invalid Format: {}", msg),
        }
    }
}

// --------------HARDCODED PALETTES--------------

pub const NTSC_DEFAULT_PALETTE: [NESColor; 64] = [
    NESColor { r: 84, g: 84, b: 84 },     NESColor { r: 0, g: 30, b: 116 },      NESColor { r: 8, g: 16, b: 144 },       NESColor { r: 48, g: 0, b: 136 },
    NESColor { r: 68, g: 0, b: 100 },     NESColor { r: 88, g: 0, b: 40 },       NESColor { r: 84, g: 4, b: 0 },         NESColor { r: 68, g: 24, b: 0 },
    NESColor { r: 32, g: 42, b: 0 },      NESColor { r: 0, g: 58, b: 0 },        NESColor { r: 0, g: 64, b: 0 },         NESColor { r: 0, g: 60, b: 0 },
    NESColor { r: 0, g: 50, b: 60 },      NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 152, g: 152, b: 152 },  NESColor { r: 0, g: 80, b: 188 },      NESColor { r: 56, g: 72, b: 240 },      NESColor { r: 104, g: 64, b: 240 },
    NESColor { r: 140, g: 48, b: 224 },   NESColor { r: 160, g: 32, b: 176 },    NESColor { r: 160, g: 32, b: 100 },     NESColor { r: 144, g: 48, b: 32 },
    NESColor { r: 104, g: 64, b: 32 },    NESColor { r: 60, g: 82, b: 0 },       NESColor { r: 0, g: 96, b: 0 },         NESColor { r: 20, g: 100, b: 0 },
    NESColor { r: 48, g: 96, b: 0 },      NESColor { r: 0, g: 84, b: 96 },       NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 240, g: 240, b: 240 },  NESColor { r: 124, g: 136, b: 252 },   NESColor { r: 188, g: 188, b: 252 },    NESColor { r: 216, g: 176, b: 252 },
    NESColor { r: 228, g: 160, b: 236 },  NESColor { r: 236, g: 144, b: 228 },   NESColor { r: 236, g: 144, b: 176 },    NESColor { r: 220, g: 160, b: 112 },
    NESColor { r: 196, g: 176, b: 96 },   NESColor { r: 148, g: 192, b: 80 },    NESColor { r: 120, g: 204, b: 80 },     NESColor { r: 88, g: 216, b: 120 },
    NESColor { r: 116, g: 208, b: 196 },  NESColor { r: 160, g: 160, b: 160 },   NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 252, g: 252, b: 252 },  NESColor { r: 188, g: 216, b: 252 },   NESColor { r: 224, g: 224, b: 252 },    NESColor { r: 236, g: 236, b: 252 },
    NESColor { r: 248, g: 216, b: 252 },  NESColor { r: 252, g: 204, b: 240 },   NESColor { r: 252, g: 196, b: 224 },    NESColor { r: 244, g: 204, b: 168 },
    NESColor { r: 228, g: 212, b: 148 },  NESColor { r: 204, g: 224, b: 132 },   NESColor { r: 184, g: 232, b: 144 },    NESColor { r: 152, g: 240, b: 180 },
    NESColor { r: 168, g: 236, b: 224 },  NESColor { r: 200, g: 200, b: 200 },   NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
];

pub const SONY_CXA_PALETTE: [NESColor; 64] = [
    NESColor { r: 88, g: 88, b: 88 },     NESColor { r: 0, g: 35, b: 134 },      NESColor { r: 17, g: 15, b: 152 },      NESColor { r: 52, g: 0, b: 136 },
    NESColor { r: 89, g: 0, b: 99 },      NESColor { r: 106, g: 0, b: 46 },      NESColor { r: 101, g: 13, b: 0 },       NESColor { r: 76, g: 34, b: 0 },
    NESColor { r: 41, g: 57, b: 0 },      NESColor { r: 3, g: 74, b: 0 },        NESColor { r: 0, g: 79, b: 0 },         NESColor { r: 0, g: 71, b: 35 },
    NESColor { r: 0, g: 58, b: 86 },      NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 160, g: 160, b: 160 },  NESColor { r: 0, g: 88, b: 230 },      NESColor { r: 54, g: 60, b: 255 },      NESColor { r: 110, g: 30, b: 240 },
    NESColor { r: 161, g: 21, b: 182 },   NESColor { r: 187, g: 30, b: 104 },    NESColor { r: 180, g: 55, b: 23 },      NESColor { r: 144, g: 85, b: 0 },
    NESColor { r: 94, g: 118, b: 0 },     NESColor { r: 36, g: 143, b: 0 },      NESColor { r: 0, g: 153, b: 0 },        NESColor { r: 0, g: 141, b: 68 },
    NESColor { r: 0, g: 118, b: 145 },    NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 255, g: 255, b: 255 },  NESColor { r: 60, g: 168, b: 255 },    NESColor { r: 109, g: 144, b: 255 },    NESColor { r: 166, g: 116, b: 255 },
    NESColor { r: 219, g: 108, b: 255 },  NESColor { r: 247, g: 118, b: 200 },   NESColor { r: 240, g: 143, b: 119 },    NESColor { r: 206, g: 172, b: 58 },
    NESColor { r: 156, g: 204, b: 42 },   NESColor { r: 96, g: 228, b: 47 },     NESColor { r: 56, g: 238, b: 90 },      NESColor { r: 48, g: 228, b: 147 },
    NESColor { r: 48, g: 204, b: 204 },   NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 255, g: 255, b: 255 },  NESColor { r: 166, g: 214, b: 255 },   NESColor { r: 186, g: 204, b: 255 },    NESColor { r: 210, g: 194, b: 255 },
    NESColor { r: 233, g: 190, b: 255 },  NESColor { r: 246, g: 194, b: 235 },   NESColor { r: 243, g: 205, b: 201 },    NESColor { r: 228, g: 217, b: 174 },
    NESColor { r: 207, g: 230, b: 168 },  NESColor { r: 183, g: 240, b: 169 },   NESColor { r: 166, g: 244, b: 187 },    NESColor { r: 162, g: 240, b: 211 },
    NESColor { r: 162, g: 230, b: 230 },  NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
];

pub const FCEUX_PALETTE: [NESColor; 64] = [
    NESColor { r: 117, g: 117, b: 117 },  NESColor { r: 39, g: 27, b: 143 },     NESColor { r: 0, g: 0, b: 171 },        NESColor { r: 71, g: 0, b: 159 },
    NESColor { r: 143, g: 0, b: 119 },    NESColor { r: 171, g: 0, b: 19 },      NESColor { r: 167, g: 0, b: 0 },        NESColor { r: 127, g: 11, b: 0 },
    NESColor { r: 67, g: 47, b: 0 },      NESColor { r: 0, g: 71, b: 0 },        NESColor { r: 0, g: 81, b: 0 },         NESColor { r: 0, g: 63, b: 23 },
    NESColor { r: 27, g: 63, b: 95 },     NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 188, g: 188, b: 188 },  NESColor { r: 0, g: 115, b: 239 },     NESColor { r: 35, g: 59, b: 239 },      NESColor { r: 131, g: 0, b: 243 },
    NESColor { r: 191, g: 0, b: 191 },    NESColor { r: 231, g: 0, b: 91 },      NESColor { r: 219, g: 43, b: 0 },       NESColor { r: 203, g: 79, b: 0 },
    NESColor { r: 139, g: 115, b: 0 },    NESColor { r: 0, g: 151, b: 0 },       NESColor { r: 0, g: 171, b: 0 },        NESColor { r: 0, g: 147, b: 59 },
    NESColor { r: 0, g: 131, b: 139 },    NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 255, g: 255, b: 255 },  NESColor { r: 63, g: 191, b: 255 },    NESColor { r: 95, g: 151, b: 255 },     NESColor { r: 167, g: 139, b: 253 },
    NESColor { r: 247, g: 123, b: 255 },  NESColor { r: 255, g: 119, b: 183 },   NESColor { r: 255, g: 119, b: 99 },     NESColor { r: 255, g: 155, b: 59 },
    NESColor { r: 243, g: 191, b: 63 },   NESColor { r: 131, g: 211, b: 19 },    NESColor { r: 79, g: 223, b: 75 },      NESColor { r: 88, g: 248, b: 152 },
    NESColor { r: 0, g: 235, b: 219 },    NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 255, g: 255, b: 255 },  NESColor { r: 171, g: 231, b: 255 },   NESColor { r: 199, g: 215, b: 255 },    NESColor { r: 215, g: 203, b: 255 },
    NESColor { r: 255, g: 199, b: 255 },  NESColor { r: 255, g: 199, b: 219 },   NESColor { r: 255, g: 191, b: 179 },    NESColor { r: 255, g: 219, b: 171 },
    NESColor { r: 255, g: 231, b: 163 },  NESColor { r: 227, g: 255, b: 163 },   NESColor { r: 171, g: 243, b: 191 },    NESColor { r: 179, g: 255, b: 207 },
    NESColor { r: 159, g: 255, b: 243 },  NESColor { r: 0, g: 0, b: 0 },         NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
];

pub const NTSC_INVERTED_PALETTE: [NESColor; 64] = [
    NESColor { r: 171, g: 171, b: 171 }, NESColor { r: 255, g: 225, b: 139 },    NESColor { r: 247, g: 239, b: 111 },    NESColor { r: 207, g: 255, b: 119 },
    NESColor { r: 187, g: 255, b: 155 }, NESColor { r: 167, g: 255, b: 215 },    NESColor { r: 171, g: 251, b: 255 },    NESColor { r: 187, g: 231, b: 255 },
    NESColor { r: 223, g: 213, b: 255 }, NESColor { r: 255, g: 197, b: 255 },    NESColor { r: 255, g: 191, b: 255 },    NESColor { r: 255, g: 195, b: 255 },
    NESColor { r: 255, g: 205, b: 195 }, NESColor { r: 255, g: 255, b: 255 },    NESColor { r: 255, g: 255, b: 255 },    NESColor { r: 255, g: 255, b: 255 },
    NESColor { r: 103, g: 103, b: 103 }, NESColor { r: 255, g: 175, b: 67 },     NESColor { r: 199, g: 183, b: 15 },     NESColor { r: 151, g: 191, b: 15 },
    NESColor { r: 115, g: 207, b: 31 },  NESColor { r: 95, g: 223, b: 79 },      NESColor { r: 95, g: 223, b: 155 },     NESColor { r: 111, g: 207, b: 223 },
    NESColor { r: 151, g: 191, b: 223 }, NESColor { r: 195, g: 173, b: 255 },    NESColor { r: 255, g: 159, b: 255 },    NESColor { r: 235, g: 155, b: 255 },
    NESColor { r: 207, g: 159, b: 255 }, NESColor { r: 255, g: 171, b: 159 },    NESColor { r: 255, g: 255, b: 255 },    NESColor { r: 255, g: 255, b: 255 },
    NESColor { r: 15, g: 15, b: 15 },    NESColor { r: 131, g: 119, b: 3 },      NESColor { r: 67, g: 67, b: 3 },        NESColor { r: 39, g: 79, b: 3 },
    NESColor { r: 27, g: 95, b: 19 },    NESColor { r: 19, g: 111, b: 27 },      NESColor { r: 19, g: 111, b: 79 },      NESColor { r: 35, g: 95, b: 143 },
    NESColor { r: 59, g: 79, b: 159 },   NESColor { r: 107, g: 63, b: 175 },     NESColor { r: 135, g: 51, b: 175 },     NESColor { r: 167, g: 39, b: 135 },
    NESColor { r: 139, g: 47, b: 59 },   NESColor { r: 95, g: 95, b: 95 },       NESColor { r: 255, g: 255, b: 255 },    NESColor { r: 255, g: 255, b: 255 },
    NESColor { r: 3, g: 3, b: 3 },       NESColor { r: 67, g: 39, b: 3 },        NESColor { r: 31, g: 31, b: 3 },        NESColor { r: 19, g: 19, b: 3 },
    NESColor { r: 7, g: 39, b: 3 },      NESColor { r: 3, g: 51, b: 15 },        NESColor { r: 3, g: 59, b: 31 },        NESColor { r: 11, g: 51, b: 87 },
    NESColor { r: 27, g: 43, b: 107 },   NESColor { r: 51, g: 31, b: 123 },      NESColor { r: 71, g: 23, b: 111 },      NESColor { r: 103, g: 15, b: 75 },
    NESColor { r: 87, g: 19, b: 31 },    NESColor { r: 55, g: 55, b: 55 },       NESColor { r: 255, g: 255, b: 255 },    NESColor { r: 255, g: 255, b: 255 },
];

pub const NESTOPIA_PALETTE: [NESColor; 64] = [
    NESColor { r: 104, g: 104, b: 104 }, NESColor { r: 0, g: 40, b: 156 },       NESColor { r: 20, g: 16, b: 172 },      NESColor { r: 64, g: 0, b: 152 },
    NESColor { r: 100, g: 0, b: 108 },   NESColor { r: 112, g: 0, b: 48 },       NESColor { r: 104, g: 16, b: 0 },       NESColor { r: 76, g: 36, b: 0 },
    NESColor { r: 40, g: 56, b: 0 },     NESColor { r: 0, g: 72, b: 0 },         NESColor { r: 0, g: 80, b: 0 },         NESColor { r: 0, g: 72, b: 40 },
    NESColor { r: 0, g: 56, b: 100 },    NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 176, g: 176, b: 176 }, NESColor { r: 24, g: 104, b: 232 },     NESColor { r: 84, g: 76, b: 252 },      NESColor { r: 140, g: 52, b: 252 },
    NESColor { r: 196, g: 40, b: 200 },  NESColor { r: 216, g: 40, b: 112 },     NESColor { r: 212, g: 56, b: 16 },      NESColor { r: 176, g: 88, b: 0 },
    NESColor { r: 116, g: 116, b: 0 },   NESColor { r: 44, g: 136, b: 0 },       NESColor { r: 0, g: 148, b: 0 },        NESColor { r: 0, g: 140, b: 84 },
    NESColor { r: 0, g: 116, b: 176 },   NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 252, g: 252, b: 252 }, NESColor { r: 108, g: 192, b: 252 },    NESColor { r: 160, g: 172, b: 252 },    NESColor { r: 216, g: 152, b: 252 },
    NESColor { r: 252, g: 140, b: 252 }, NESColor { r: 252, g: 140, b: 200 },    NESColor { r: 252, g: 156, b: 136 },    NESColor { r: 252, g: 180, b: 72 },
    NESColor { r: 212, g: 204, b: 40 },  NESColor { r: 160, g: 224, b: 40 },     NESColor { r: 112, g: 236, b: 56 },     NESColor { r: 80, g: 236, b: 132 },
    NESColor { r: 84, g: 220, b: 216 },  NESColor { r: 156, g: 156, b: 156 },    NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 252, g: 252, b: 252 }, NESColor { r: 196, g: 228, b: 252 },    NESColor { r: 216, g: 220, b: 252 },    NESColor { r: 240, g: 212, b: 252 },
    NESColor { r: 252, g: 208, b: 252 }, NESColor { r: 252, g: 208, b: 232 },    NESColor { r: 252, g: 216, b: 204 },    NESColor { r: 252, g: 228, b: 176 },
    NESColor { r: 240, g: 236, b: 164 }, NESColor { r: 216, g: 244, b: 164 },    NESColor { r: 196, g: 248, b: 176 },    NESColor { r: 180, g: 248, b: 208 },
    NESColor { r: 180, g: 244, b: 240 }, NESColor { r: 212, g: 212, b: 212 },    NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
];

pub const SHOVEL_KNIGHT_NES_PALETTE: [NESColor; 64] = [
    NESColor { r: 0x2c, g: 0x2c, b: 0x2c }, NESColor { r: 0x00, g: 0x00, b: 0xc4 }, NESColor { r: 0x40, g: 0x28, b: 0xc4 }, NESColor { r: 0x94, g: 0x00, b: 0x8c },
    NESColor { r: 0xac, g: 0x00, b: 0x28 }, NESColor { r: 0xac, g: 0x10, b: 0x00 }, NESColor { r: 0x8c, g: 0x18, b: 0x00 }, NESColor { r: 0x50, g: 0x30, b: 0x00 },
    NESColor { r: 0x00, g: 72, b: 0x00 },   NESColor { r: 0x00, g: 104, b: 0x00 },  NESColor { r: 0x00, g: 88, b: 0x00 },   NESColor { r: 0x00, g: 64, b: 88 },
    NESColor { r: 0x00, g: 64, b: 88 },     NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 0x60, g: 0x60, b: 0x60 }, NESColor { r: 0x00, g: 0x78, b: 0xfc }, NESColor { r: 0x68, g: 0x48, b: 0xfc }, NESColor { r: 0xdc, g: 0x00, b: 0xd4 },
    NESColor { r: 0xe4, g: 0x00, b: 0x60 }, NESColor { r: 0xfc, g: 0x38, b: 0x00 }, NESColor { r: 0xe4, g: 0x60, b: 0x18 }, NESColor { r: 0xac, g: 0x80, b: 0x00 },
    NESColor { r: 0x00, g: 0xb8, b: 0x00 }, NESColor { r: 0x00, g: 0xa8, b: 0x00 }, NESColor { r: 0x00, g: 0xa8, b: 0x48 }, NESColor { r: 0x00, g: 0x88, b: 0x94 },
    NESColor { r: 0x00, g: 0x40, b: 0x58 }, NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 0xc8, g: 0xc0, b: 0xc0 }, NESColor { r: 0x38, g: 0xc0, b: 0xfc }, NESColor { r: 0x9c, g: 0x78, b: 0xfc }, NESColor { r: 0xfc, g: 0x78, b: 0xfc },
    NESColor { r: 0xfc, g: 0x58, b: 0x9c }, NESColor { r: 0xfc, g: 0x78, b: 0x58 }, NESColor { r: 0xfc, g: 0xa0, b: 0x48 }, NESColor { r: 0xfc, g: 0xb8, b: 0x00 },
    NESColor { r: 0xbc, g: 0xf8, b: 0x18 }, NESColor { r: 0x58, g: 0xd8, b: 0x58 }, NESColor { r: 0x58, g: 0xf8, b: 0x9c }, NESColor { r: 0x01, g: 0xe8, b: 0xe4 },
    NESColor { r: 0x68, g: 0x88, b: 0xfc }, NESColor { r: 0x78, g: 0x80, b: 0x84 }, NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
    NESColor { r: 0xfc, g: 0xf8, b: 0xfc }, NESColor { r: 0xa4, g: 0xe8, b: 0xfc }, NESColor { r: 0xdc, g: 0xb8, b: 0xfc }, NESColor { r: 0xfc, g: 0xb8, b: 0xfc },
    NESColor { r: 0xf4, g: 0xc0, b: 0xe0 }, NESColor { r: 0xf4, g: 0xd0, b: 0xb4 }, NESColor { r: 0xfc, g: 0xe0, b: 0xb4 }, NESColor { r: 0xfc, g: 0xd8, b: 0x84 },
    NESColor { r: 0xdc, g: 0xf8, b: 0x78 }, NESColor { r: 0xb8, g: 0xf8, b: 0x78 }, NESColor { r: 0xb0, g: 0xf0, b: 0xd8 }, NESColor { r: 0x01, g: 0xf8, b: 0xfc },
    NESColor { r: 0xbc, g: 0xb8, b: 0xfc }, NESColor { r: 0xbc, g: 0xc0, b: 0xc4 }, NESColor { r: 0, g: 0, b: 0 },          NESColor { r: 0, g: 0, b: 0 },
];