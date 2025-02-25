use crate::Result;
use ratatui::style::{Color, Style, Stylize};
use serde::{Deserialize, Serialize};
use std::{fs::File, sync::LazyLock};

use crate::utils::novel_catch_dir;

pub static THEME_CONFIG: LazyLock<ThemeConfig> = LazyLock::new(|| {
    let path = novel_catch_dir().unwrap().join("theme.json");
    match File::open(path.clone()) {
        Ok(file) => serde_json::from_reader(file).unwrap_or_default(),
        Err(_) => ThemeConfig::default(),
    }
});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfig {
    pub colors: ThemeColors,

    pub logo: Style,
    pub highlight: Style,
    pub selected: Style,
    pub empty: Style,
    pub detail_info: Style,

    pub basic: BasicSetting,
    pub loading_modal: BasicSetting,
    pub warning_modal: BasicSetting,
    pub error_modal: BasicSetting,
    pub search: SearchSetting,
    pub novel: NovelSetting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColors {
    pub text_color: Color,
    pub primary_color: Color,
    pub warning_color: Color,
    pub error_color: Color,
    pub success_color: Color,
    pub info_color: Color,
}

impl ThemeConfig {
    pub fn save(&self) -> Result<()> {
        let path = novel_catch_dir()?.join("theme.json");
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn from_colors(colors: ThemeColors) -> Self {
        let ThemeColors {
            text_color,
            primary_color,
            warning_color,
            error_color,
            success_color,
            info_color,
        } = colors;

        let basic = BasicSetting {
            text: Style::new().fg(text_color),
            border: Style::new().dim(),
            border_info: Style::new().dim().fg(info_color),
            ..Default::default()
        };
        Self {
            colors,
            detail_info: Style::new().fg(warning_color).bold(),
            logo: Style::new().light_green(),
            selected: Style::new().fg(success_color),
            highlight: Style::new().fg(primary_color),
            empty: Style::new().fg(warning_color).bold(),
            search: SearchSetting {
                success_border: Style::new().fg(success_color),
                error_border: Style::new().fg(error_color),
                error_border_info: Style::new().fg(info_color),
                placeholder: Style::new().fg(primary_color),
                text: Style::new().fg(text_color),
            },
            loading_modal: BasicSetting {
                border: basic.border.fg(primary_color),
                text: Style::new().fg(primary_color),
                ..Default::default()
            },
            warning_modal: BasicSetting {
                border: basic.border.fg(warning_color),
                text: Style::new().fg(warning_color),
                ..Default::default()
            },
            error_modal: BasicSetting {
                border: basic.border.fg(error_color),
                text: Style::new().fg(error_color),
                ..Default::default()
            },
            novel: NovelSetting {
                content: Style::new().fg(text_color),
                chapter: basic.border_title,
                page: basic.border_info,
                progress: basic.border_info,
                border: basic.border,
            },
            basic,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let colors = ThemeColors {
            text_color: Color::default(),
            primary_color: Color::LightBlue,
            warning_color: Color::LightYellow,
            error_color: Color::LightRed,
            success_color: Color::LightGreen,
            info_color: Color::Gray,
        };
        Self::from_colors(colors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BasicSetting {
    pub text: Style,
    pub border: Style,
    pub border_title: Style,
    pub border_info: Style,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchSetting {
    pub success_border: Style,
    pub error_border: Style,
    pub error_border_info: Style,
    pub placeholder: Style,
    pub text: Style,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NovelSetting {
    pub chapter: Style,
    pub page: Style,
    pub content: Style,
    pub progress: Style,
    pub border: Style,
}
