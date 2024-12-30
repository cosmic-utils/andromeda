pub mod drive_format;
pub mod partition_create;
pub mod partition_format;

use super::{error::Error, message::AppMessage};
use cosmic::prelude::*;

#[derive(Debug, Clone)]
pub enum Operation {
    DriveFormat,
    AddPartition(u64, u64),
    PartitionFormat(u64),
}

impl Into<Box<dyn OperationDialog>> for Operation {
    fn into(self) -> Box<dyn OperationDialog> {
        match self {
            Self::DriveFormat => Box::new(drive_format::DriveFormat::new()),
            Self::AddPartition(offset, max_size) => {
                Box::new(partition_create::AddPartition::new(offset, max_size))
            }
            Self::PartitionFormat(offset) => {
                Box::new(partition_format::PartitionFormat::new(offset))
            }
        }
    }
}

pub trait OperationDialog {
    fn update(&mut self, message: AppMessage) -> cosmic::app::Task<Result<AppMessage, Error>>;
    fn dialog(&self) -> Element<Result<AppMessage, Error>>;
}
