use std::collections::HashMap;

use cosmic::prelude::*;
use cosmic::widget::nav_bar::Id;
use cosmic::{iced, theme, widget};

use udisks2::{
    block::BlockProxy, drive::DriveProxy, filesystem::FilesystemProxy, partition::PartitionProxy,
    partitiontable::PartitionTableProxy, zbus::zvariant::OwnedObjectPath, Client,
};

use super::operation::Operation;
use super::{error::Error, message::AppMessage};
use crate::widget::Ring;

#[derive(Clone, Debug)]
pub struct Drive {
    pub block: BlockProxy<'static>,
    pub block_path: OwnedObjectPath,
    pub _drive: DriveProxy<'static>,
    pub ptable: Option<PartitionTableProxy<'static>>,

    pub ring: Ring,

    pub model: String,
    pub size: String,
    pub serial: String,
    pub revision: String,
    pub partitioning: String,

    pub partitions: Vec<Block>,
}

#[derive(Eq, PartialEq, Clone, Copy)]
enum DriveAction {
    Format,
    MakeImg,
    RestoreImg,
}

impl widget::menu::Action for DriveAction {
    type Message = Result<AppMessage, Error>;

    fn message(&self) -> Self::Message {
        match self {
            Self::Format => Ok(AppMessage::OpenOperationDialog(Operation::DriveFormat)),
            Self::MakeImg => Ok(AppMessage::NoOp),
            Self::RestoreImg => Ok(AppMessage::NoOp),
        }
    }
}

impl Drive {
    pub async fn load(
        client: Client,
        id: Id,
        block_path: OwnedObjectPath,
    ) -> Result<AppMessage, Error> {
        let block_device = client.object(block_path.clone()).unwrap();
        let block = block_device.block().await?;

        let drive_path = block.drive().await?;
        let drive_device = client.object(drive_path).unwrap();
        let drive = drive_device.drive().await?;

        let ptable = block_device.partition_table().await;

        let mut partitions = Vec::new();
        if let Ok(ptable) = &ptable {
            for partition_path in ptable.partitions().await? {
                let block = client
                    .object(partition_path.clone())
                    .unwrap()
                    .block()
                    .await?;
                let part = client
                    .object(partition_path.clone())
                    .unwrap()
                    .partition()
                    .await?;
                let fs = client
                    .object(partition_path.clone())
                    .unwrap()
                    .filesystem()
                    .await;
                let partition = Partition {
                    name: std::path::Path::new(&partition_path.to_string())
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),

                    block_size: client.size_for_display(block.size().await?, true, false),
                    size: client.size_for_display(part.size().await?, true, false),
                    offset: client.size_for_display(part.offset().await?, true, false),
                    r#type: client
                        .partition_type_for_display(
                            ptable.type_().await?.as_str(),
                            part.type_().await?.as_str(),
                        )
                        .unwrap_or("part-type\u{004}None".to_string())
                        .split('\u{004}')
                        .last()
                        .unwrap_or(part.type_().await?.as_str())
                        .to_string(),
                    uuid: block.id_uuid().await?,

                    partition_id: client
                        .id_for_display(
                            block.id_usage().await?.as_str(),
                            block.id_type().await?.as_str(),
                            block.id_version().await?.as_str(),
                            false,
                        )
                        .split('\u{004}')
                        .last()
                        .unwrap_or(block.id_type().await?.as_str())
                        .to_string(),

                    block,
                    _part: part.clone(),
                    fs: fs.ok(),
                };

                let block = Block {
                    size: part.size().await?,
                    offset: part.offset().await?,
                    size_for_display: client.size_for_display(part.size().await?, true, false),
                    offset_for_display: client.size_for_display(part.offset().await?, true, false),
                    partition: Some(partition.clone()),
                };

                partitions.push(block);
            }

            partitions.sort_by(|a, b| a.offset.cmp(&b.offset));
            let mut partitions_result = Vec::new();
            if partitions.len() > 0 {
                // Leading empty space
                let start = 0;
                let end = partitions[0].offset;
                // TODO: Block size checking for drives
                if end - start > 512 {
                    partitions_result.push(Block {
                        size: end - start,
                        offset: start,
                        size_for_display: client.size_for_display(end - start, true, false),
                        offset_for_display: client.size_for_display(start, true, false),
                        partition: None,
                    })
                }
                // Partition pairs
                for (par_a, par_b) in partitions.iter().zip(partitions.iter().skip(1)) {
                    let start = par_a.offset + par_a.size;
                    let end = par_b.offset;

                    partitions_result.push(par_a.clone());
                    if end - start > 512 {
                        partitions_result.push(Block {
                            size: end - start,
                            offset: start,
                            size_for_display: client.size_for_display(end - start, true, false),
                            offset_for_display: client.size_for_display(start, true, false),
                            partition: None,
                        })
                    }
                }
                // Push last partition
                let last_par = partitions.last().unwrap();
                partitions_result.push(last_par.clone());
                // Trailing empty space
                let start = last_par.offset + last_par.size;
                let end = block.size().await?;
                if end - start > 512 {
                    partitions_result.push(Block {
                        size: end - start,
                        offset: start,
                        size_for_display: client.size_for_display(end - start, true, false),
                        offset_for_display: client.size_for_display(start, true, false),
                        partition: None,
                    })
                }
            } else {
                partitions_result.push(Block {
                    size: block.size().await?,
                    offset: 0,
                    size_for_display: client.size_for_display(block.size().await?, true, false),
                    offset_for_display: client.size_for_display(0, true, false),
                    partition: None,
                });
            }
            partitions = partitions_result;
        }

        Ok(AppMessage::DriveRead(
            id,
            Drive {
                model: drive.model().await?,
                size: client.size_for_display(block.size().await?, true, false),
                serial: drive.serial().await?,
                revision: drive.revision().await?,
                partitioning: match &ptable {
                    Ok(ptable) => match ptable.type_().await?.as_str() {
                        "gpt" => "GUID Partition Table",
                        "mbr" => "Master Boot Record",
                        _ => "Unknown",
                    }
                    .to_string(),
                    Err(_) => "Empty".to_string(),
                },
                ring: Ring {
                    sections: Vec::new(),
                    line_width: 12.0,
                    selected_par: None,
                },

                partitions,

                block,
                block_path,
                _drive: drive,
                ptable: ptable.ok(),
            },
        ))
    }

    pub fn menu_bar(&self) -> Element<Result<AppMessage, Error>> {
        use widget::menu;
        menu::bar(vec![
            menu::Tree::with_children(
                menu::root("Drive"),
                menu::items(
                    &HashMap::new(),
                    vec![
                        menu::Item::Button("Format", None, DriveAction::Format),
                        menu::Item::Divider,
                        menu::Item::ButtonDisabled("Create Disk Image", None, DriveAction::MakeImg),
                        menu::Item::ButtonDisabled(
                            "Restore Disk Image",
                            None,
                            DriveAction::RestoreImg,
                        ),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root("Partitions"),
                menu::items(
                    &HashMap::new(),
                    self.partitions
                        .iter()
                        .map(|partition| partition.menu_folder())
                        .collect(),
                ),
            ),
        ])
        .apply(Element::from)
    }

    pub fn view(&self) -> Element<Result<AppMessage, Error>> {
        let theme = theme::active();
        let cosmic = theme.cosmic();

        widget::column()
            .spacing(cosmic.space_s())
            // Drive view
            .push(
                widget::column()
                    .spacing(cosmic.space_xs())
                    // Header row
                    .push(widget::text::title3(&self.model))
                    // Drive information
                    .push(
                        widget::flex_row(vec![
                            widget::canvas(&self.ring)
                                .width(iced::Length::Fill)
                                .height(iced::Length::Fixed(250.0))
                                .apply(Element::from),
                            widget::container(
                                widget::settings::section()
                                    .add(widget::settings::item(
                                        "Size",
                                        widget::text::heading(&self.size),
                                    ))
                                    .add(widget::settings::item(
                                        "Serial",
                                        widget::text::body(&self.serial),
                                    ))
                                    .add(widget::settings::item(
                                        "Revision",
                                        widget::text::body(&self.revision),
                                    ))
                                    .add(widget::settings::item(
                                        "Partition Table",
                                        widget::text::body(&self.partitioning),
                                    )),
                            )
                            .apply(Element::from),
                        ])
                        .align_items(iced::Alignment::Center),
                    ),
            )
            .push(
                widget::column()
                    .push(widget::text::title3("Partitions"))
                    .push(iced::widget::horizontal_rule(1)),
            )
            .push(widget::scrollable(
                widget::column::with_children(
                    self.partitions
                        .iter()
                        .map(|partition| partition.view())
                        .collect(),
                )
                .padding([0, cosmic.space_xs(), 0, 0])
                .spacing(cosmic.space_m()),
            ))
            .into()
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BlockAction {
    AddPartition(u64, u64),
    FormatPartition(u64),
}

impl widget::menu::Action for BlockAction {
    type Message = Result<AppMessage, Error>;

    fn message(&self) -> Self::Message {
        match self {
            Self::AddPartition(offset, max_size) => Ok(AppMessage::OpenOperationDialog(
                Operation::AddPartition(*offset, *max_size),
            )),
            Self::FormatPartition(offset) => Ok(AppMessage::OpenOperationDialog(
                Operation::PartitionFormat(*offset),
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub offset: u64,
    pub offset_for_display: String,
    pub size: u64,
    pub size_for_display: String,
    pub partition: Option<Partition>,
}

impl Block {
    fn menu_folder(&self) -> widget::menu::Item<BlockAction, String> {
        use widget::menu;
        match &self.partition {
            Some(partition) => menu::Item::Folder(
                partition.name.to_string(),
                vec![menu::Item::Button(
                    "Format".to_string(),
                    None,
                    BlockAction::FormatPartition(self.offset),
                )],
            ),
            None => menu::Item::Folder(
                "Empty Space".to_string(),
                vec![menu::Item::Button(
                    "Add Partition".to_string(),
                    None,
                    BlockAction::AddPartition(self.offset, self.size),
                )],
            ),
        }
    }

    fn view(&self) -> Element<Result<AppMessage, Error>> {
        if let Some(partition) = &self.partition {
            partition.view()
        } else {
            widget::settings::section()
                .title("Empty Space")
                .add(widget::settings::item(
                    "Size",
                    widget::text::body(&self.size_for_display),
                ))
                .add(widget::settings::item(
                    "Offset",
                    widget::text::body(&self.offset_for_display),
                ))
                .apply(Element::from)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Partition {
    pub block: BlockProxy<'static>,
    pub _part: PartitionProxy<'static>,
    pub fs: Option<FilesystemProxy<'static>>,

    pub name: String,
    pub partition_id: String,
    pub size: String,
    pub offset: String,

    pub r#type: String,
    pub block_size: String,

    pub uuid: String,
}

impl Partition {
    pub fn view(&self) -> Element<Result<AppMessage, Error>> {
        widget::settings::section()
            .title(&self.name)
            .add(widget::settings::item(
                "File System",
                widget::text::heading(&self.partition_id),
            ))
            .add(widget::settings::item(
                "Type",
                widget::text::heading(&self.r#type),
            ))
            .add(widget::settings::item(
                "Size",
                widget::text::heading(&self.size),
            ))
            .add(widget::settings::item(
                "Offset",
                widget::text::body(&self.offset),
            ))
            .add(widget::settings::item(
                "Capacity",
                widget::text::body(&self.block_size),
            ))
            .add(widget::settings::item(
                "UUID",
                widget::text::caption(&self.uuid),
            ))
            .into()
    }
}
