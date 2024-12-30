use cosmic::{prelude::*, widget};

use crate::app::{error::Error, message::AppMessage};

pub struct AddPartition {
    offset: u64,
    size: u64,
    max_size: u64,
    size_string: String,
}

impl AddPartition {
    pub fn new(offset: u64, max_size: u64) -> Self {
        Self {
            offset,
            size: 0,
            max_size,
            size_string: "0".to_string(),
        }
    }
}

impl super::OperationDialog for AddPartition {
    fn update(&mut self, message: AppMessage) -> cosmic::app::Task<Result<AppMessage, Error>> {
        let mut tasks = Vec::new();
        match message {
            AppMessage::OperationAddPartitionSizeUpdate(input) => self.size_string = input,
            AppMessage::OperationAddPartitionSizeSave => {
                let mut size: u64 = (if let Ok(size) = self.size_string.parse() {
                    size
                } else {
                    self.size
                } / 512)
                    * 512;
                size = size.max(512).min(self.max_size);
                self.size_string = size.to_string();
            }
            AppMessage::PerformOperation(drive) => {
                if let Some(ptable) = drive.ptable {
                    let size = self.size;
                    let offset = self.offset;
                    tasks.push(cosmic::task::future(async move {
                        ptable
                            .create_partition(
                                offset,
                                size,
                                "",
                                "",
                                udisks2::standard_options(false),
                            )
                            .await?;

                        Ok(AppMessage::OperationFinish)
                    }));
                }
            }
            _ => {}
        }
        cosmic::app::Task::batch(tasks)
    }

    fn dialog(&self) -> Element<Result<AppMessage, Error>> {
        use widget::settings;
        widget::dialog()
            .title("Create Partition")
            .body("Create a partition in the empty space")
            .control(
                settings::section().add(settings::item(
                    "Size",
                    widget::text_input("", &self.size_string)
                        .on_input(|input| Ok(AppMessage::OperationAddPartitionSizeUpdate(input)))
                        .on_submit(Ok(AppMessage::OperationAddPartitionSizeSave)),
                )),
            )
            .primary_action(
                widget::button::suggested("Create").on_press(Ok(AppMessage::ConfirmOperation)),
            )
            .into()
    }
}
