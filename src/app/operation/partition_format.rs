use crate::app::{error::Error, message::AppMessage};
use cosmic::{prelude::*, widget};

pub struct PartitionFormat {
    block_offset: u64,

    name: String,
    erase: bool,
    type_: String,
    type_index: Option<usize>,
}

impl PartitionFormat {
    pub fn new(block_offset: u64) -> Self {
        Self {
            block_offset,
            name: "".to_string(),
            erase: false,
            type_: "ext4".to_string(),
            type_index: Some(0),
        }
    }
}

impl super::OperationDialog for PartitionFormat {
    fn update(&mut self, message: AppMessage) -> cosmic::app::Task<Result<AppMessage, Error>> {
        let mut tasks = Vec::new();
        match message {
            AppMessage::OperationPartitionFormatNameUpdate(name) => self.name = name,
            AppMessage::OperationPartitionFormatToggleErase(toggle) => self.erase = toggle,
            AppMessage::OperationPartitionFormatSelectFS(index) => {
                self.type_ = match index {
                    0 => "ext4".to_string(),
                    1 => "ntfs".to_string(),
                    2 => "vfat".to_string(),
                    _ => unreachable!(),
                };
                self.type_index = Some(index)
            }
            AppMessage::PerformOperation(drive) => {
                let partition = drive
                    .partitions
                    .iter()
                    .find(|partition| partition.offset == self.block_offset);
                let name = self.name.clone();
                let erase = self.erase;
                let type_ = self.type_.clone();
                if let Some(block) = partition {
                    if let Some(partition) = &block.partition {
                        let block = partition.block.clone();
                        let fs = partition.fs.clone();
                        tasks.push(cosmic::task::future(async move {
                            if let Some(fs) = fs {
                                let _ = fs.unmount(udisks2::standard_options(false)).await;
                            }

                            let mut options = udisks2::standard_options(false);
                            options.insert("update-partition-type", true.into());
                            if erase {
                                options.insert("erase", "zeros".into());
                            }
                            if type_ != "vfat" {
                                options.insert("label", name.into());
                            }

                            block.format(type_.as_str(), options).await?;
                            Ok(AppMessage::OperationFinish)
                        }));
                    }
                }
            }
            _ => {}
        }
        cosmic::app::Task::batch(tasks)
    }

    fn dialog(&self) -> Element<Result<AppMessage, Error>> {
        widget::dialog()
            .title("Format Partition")
            .body("Create a filesystem for the selected partition, this erases all data on the volume! Please back up data before you format.")
            .control(
                widget::settings::section()
                    .add(widget::settings::item(
                        "Volume Name",
                        widget::text_input("", &self.name)
                            .on_input(|input| Ok(AppMessage::OperationPartitionFormatNameUpdate(input)))
                    ))
                    .add(widget::settings::item(
                        "Full Erase (Slow)",
                        widget::toggler(self.erase).on_toggle(|toggle| Ok(AppMessage::OperationPartitionFormatToggleErase(toggle)))
                    ))
                    .add(widget::settings::item(
                        "Filesystem Type",
                        widget::dropdown(&["Linux (Ext4)", "Windows (NTFS)", "Universal (FAT)"], self.type_index, |index| Ok(AppMessage::OperationPartitionFormatSelectFS(index)))
                    ))
            )
            .primary_action(widget::button::destructive("Confirm").on_press(Ok(AppMessage::ConfirmOperation)))
            .secondary_action(widget::button::standard("Cancel").on_press(Ok(AppMessage::CancelOperation)))
            .into()
    }
}
