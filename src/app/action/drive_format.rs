use std::collections::HashMap;

use cosmic::iced::Alignment;
use cosmic::prelude::*;
use cosmic::{iced, widget};

use crate::app::error::Error;
use crate::app::message::AppMessage;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
enum EraseMode {
    #[default]
    Quick,
    Full,
}

impl AsRef<str> for EraseMode {
    fn as_ref(&self) -> &str {
        match self {
            EraseMode::Quick => "Quick (less secure, faster)",
            EraseMode::Full => "Full (more secure, slower)",
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
enum TableType {
    #[default]
    Gpt,
    Mbr,
    Empty,
}
impl AsRef<str> for TableType {
    fn as_ref(&self) -> &str {
        match self {
            Self::Gpt => "GUID Partition Table (Modern)",
            Self::Mbr => "Master Boot Record (Legacy)",
            Self::Empty => "Empty",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriveFormatOptions {
    erase_option: Option<usize>,
    ptable_option: Option<usize>,
}

impl DriveFormatOptions {
    pub fn new() -> Self {
        Self {
            erase_option: Some(0),
            ptable_option: Some(0),
        }
    }

    pub fn control(&self) -> Element<Result<AppMessage, Error>> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        widget::column()
            .spacing(cosmic.space_s())
            .push(
                widget::row()
                    .push(widget::text::heading("Erase Mode"))
                    .push(widget::horizontal_space())
                    .push(widget::dropdown(
                        &[EraseMode::Full, EraseMode::Quick],
                        self.erase_option,
                        |option| Ok(AppMessage::ActionSelection(0, option)),
                    )),
            )
            .push(
                widget::row()
                    .align_y(Alignment::Center)
                    .push(widget::text::heading("Partition Table Type"))
                    .push(widget::horizontal_space())
                    .push(widget::dropdown(
                        &[TableType::Gpt, TableType::Mbr, TableType::Empty],
                        self.ptable_option,
                        |index| Ok(AppMessage::ActionSelection(1, index)),
                    )),
            )
            .into()
    }

    pub fn on_option_select(&mut self, option_number: usize, option_index: usize) {
        match option_number {
            0 => self.erase_option = Some(option_index),
            1 => self.ptable_option = Some(option_index),
            _ => unreachable!(),
        }
    }

    pub fn on_confirm(
        &self,
        block: udisks2::block::BlockProxy<'static>,
    ) -> cosmic::app::Task<Result<AppMessage, Error>> {
        let erase = self.erase_option.unwrap();
        let ptable = self.ptable_option.unwrap();
        cosmic::task::future(async move {
            let mut options = HashMap::new();
            if erase == 0 {
                use udisks2::zbus::zvariant;
                options.insert(
                    "erase",
                    zvariant::Value::Str(zvariant::Str::from_static("zero")),
                );
            }
            let ptype = match ptable {
                0 => "gpt",
                1 => "dos",
                _ => "empty",
            };

            block.format(ptype, options).await?;

            Ok(AppMessage::ActionDone)
        })
    }
}
