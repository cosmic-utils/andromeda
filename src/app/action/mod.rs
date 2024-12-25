//pub mod add_partition;
pub mod drive_format;

use cosmic::prelude::*;
use udisks2::block::BlockProxy;

use super::{error::Error, message::AppMessage};

#[derive(Debug, Clone)]
pub enum Action {
    DriveFormat(drive_format::DriveFormatOptions),
    //AddPartition(add_partition::AddPartitionOptions),
}

impl Action {
    pub fn dialog(&self) -> Element<Result<AppMessage, Error>> {
        cosmic::widget::dialog()
            .title(format!("{} Options", self))
            .primary_action(
                cosmic::widget::button::suggested("Confirm")
                    .on_press(Ok(AppMessage::ConfirmAction)),
            )
            .secondary_action(
                cosmic::widget::button::standard("Cancel").on_press(Ok(AppMessage::CancelAction)),
            )
            .control(match self {
                Action::DriveFormat(options) => options.control(),
            })
            .into()
    }

    pub fn on_option_select(&mut self, menu: usize, option: usize) {
        match self {
            Self::DriveFormat(options) => options.on_option_select(menu, option),
        }
    }

    pub fn on_action(
        &mut self,
        block: &super::drive::Drive,
    ) -> cosmic::app::Task<Result<AppMessage, Error>> {
        match self {
            Self::DriveFormat(options) => options.on_confirm(block),
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Action::DriveFormat(_) => "Drive Format",
        })
    }
}
