use std::collections::HashMap;

use cosmic::prelude::*;
use cosmic::widget::nav_bar::Id;
use cosmic::{iced, theme, widget};

use udisks2::{
    block::BlockProxy, drive::DriveProxy, filesystem::FilesystemProxy, partition::PartitionProxy,
    partitiontable::PartitionTableProxy, zbus::zvariant::OwnedObjectPath, Client,
};

use super::action;
use super::{error::Error, message::AppMessage};
use crate::widget::Ring;

#[derive(Clone, Debug)]
pub struct Drive {
    pub block: BlockProxy<'static>,
    pub block_path: OwnedObjectPath,
    pub drive: DriveProxy<'static>,
    pub ptable: Option<PartitionTableProxy<'static>>,

    pub ring: Ring,

    pub model: String,
    pub size: String,
    pub serial: String,
    pub revision: String,
    pub partitioning: String,

    partitions: Vec<Block>,
}

#[derive(std::cmp::Eq, PartialEq, Clone, Copy)]
enum DriveAction {
    Format,
    MakeImg,
    RestoreImg,
}

impl widget::menu::Action for DriveAction {
    type Message = Result<AppMessage, Error>;

    fn message(&self) -> Self::Message {
        match self {
            Self::Format => Ok(AppMessage::Action(action::Action::DriveFormat(
                action::drive_format::DriveFormatOptions::new(),
            ))),
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
                partitions.push(Block {
                    size: part.size().await?,
                    offset: part.offset().await?,
                    size_for_display: client.size_for_display(part.size().await?, true, false),
                    offset_for_display: client.size_for_display(part.offset().await?, true, false),
                    partition: Some(Partition {
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
                        part,
                        fs: fs.ok(),
                    }),
                });
            }

            partitions.sort_by(|a, b| a.offset.cmp(&b.offset));
            let mut partitions_result = Vec::new();
            if partitions.len() > 0 {
                // Leading empty space
                let start = 0;
                let end = partitions[0].offset;
                // TODO: Block size checking for drives
                if end - start > 4 {
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
                if end - start > 4 {
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
                    Ok(ptable) => ptable.type_().await?,
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
                drive,
                ptable: ptable.ok(),
            },
        ))
    }

    pub fn view(&self) -> Element<Result<AppMessage, Error>> {
        let theme = theme::active();
        let cosmic = theme.cosmic();

        use widget::menu;
        widget::column()
            .spacing(cosmic.space_s())
            // Drive view
            .push(
                widget::column()
                    .spacing(cosmic.space_xs())
                    // Header row
                    .push(
                        widget::row()
                            .align_y(iced::Alignment::Center)
                            .push(widget::text::title3(&self.model))
                            .push(widget::horizontal_space())
                            .push(menu::bar(vec![menu::Tree::with_children(
                                widget::button::standard("Manage")
                                    .trailing_icon(widget::icon::from_name("pan-down-symbolic")),
                                menu::items(
                                    &HashMap::new(),
                                    vec![
                                        menu::Item::Button("Format", None, DriveAction::Format),
                                        menu::Item::Divider,
                                        menu::Item::ButtonDisabled(
                                            "Create Disk Image",
                                            None,
                                            DriveAction::MakeImg,
                                        ),
                                        menu::Item::ButtonDisabled(
                                            "Restore Disk Image",
                                            None,
                                            DriveAction::RestoreImg,
                                        ),
                                    ],
                                ),
                            )])),
                    )
                    // Drive information
                    .push(
                        widget::layer_container(
                            widget::row()
                                .align_y(iced::Alignment::Center)
                                .push(widget::canvas(&self.ring))
                                .push(widget::horizontal_space())
                                .push(
                                    widget::column()
                                        .spacing(cosmic.space_xs())
                                        .push(
                                            widget::row()
                                                .push(widget::text::heading("Size"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.size)),
                                        )
                                        .push(
                                            widget::row()
                                                .push(widget::text::body("Serial"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.serial)),
                                        )
                                        .push(
                                            widget::row()
                                                .push(widget::text::body("Revision"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.revision)),
                                        )
                                        .push(
                                            widget::row()
                                                .push(widget::text::heading("Partition Table"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.partitioning)),
                                        ),
                                ),
                        )
                        .padding([cosmic.space_xs(), cosmic.space_xs()])
                        .layer(cosmic::cosmic_theme::Layer::Primary),
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
                .spacing(cosmic.space_xs()),
            ))
            .into()
    }
}

#[derive(Clone, Debug)]
struct Block {
    offset: u64,
    offset_for_display: String,
    size: u64,
    size_for_display: String,
    partition: Option<Partition>,
}

impl Block {
    fn view(&self) -> Element<Result<AppMessage, Error>> {
        if let Some(partition) = &self.partition {
            partition.view()
        } else {
            let theme = theme::active();
            let cosmic = theme.cosmic();
            widget::column()
                .spacing(cosmic.space_xs())
                .push(
                    widget::row()
                        .align_y(iced::Alignment::Center)
                        .push(widget::text::title4("Empty Space")),
                )
                .push(
                    widget::layer_container(
                        widget::row()
                            .align_y(iced::Alignment::Center)
                            .spacing(cosmic.space_s())
                            .push(
                                widget::row()
                                    .push(widget::text::heading("Size"))
                                    .push(widget::horizontal_space())
                                    .push(widget::text::heading(&self.size_for_display)),
                            )
                            .push(
                                widget::row()
                                    .push(widget::text::heading("Offset"))
                                    .push(widget::horizontal_space())
                                    .push(widget::text::heading(&self.offset_for_display)),
                            ),
                    )
                    .padding([cosmic.space_xs(), cosmic.space_xs()])
                    .layer(cosmic::cosmic_theme::Layer::Primary),
                )
                .into()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Partition {
    pub block: BlockProxy<'static>,
    pub part: PartitionProxy<'static>,
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
        let theme = theme::active();
        let cosmic = theme.cosmic();

        widget::column()
            .spacing(cosmic.space_xs())
            // Header row
            .push(
                widget::row()
                    .align_y(iced::Alignment::Center)
                    .push(widget::text::title4(&self.name)),
            )
            // Partition information
            .push(
                widget::layer_container(
                    widget::column()
                        .push(
                            widget::row()
                                .align_y(iced::Alignment::Center)
                                .spacing(cosmic.space_s())
                                .push(
                                    widget::column()
                                        .push(
                                            widget::row()
                                                .push(widget::text::heading("Filesystem"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.partition_id)),
                                        )
                                        .push(
                                            widget::row()
                                                .push(widget::text::heading("Size"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.size)),
                                        )
                                        .push(
                                            widget::row()
                                                .push(widget::text::heading("Offset"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::heading(&self.offset)),
                                        ),
                                )
                                .push(
                                    widget::column()
                                        .push(
                                            widget::row()
                                                .push(widget::text::body("Capacity"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::body(&self.block_size)),
                                        )
                                        .push(
                                            widget::row()
                                                .push(widget::text::body("Type"))
                                                .push(widget::horizontal_space())
                                                .push(widget::text::body(&self.r#type)),
                                        ),
                                ),
                        )
                        .push(
                            widget::row()
                                .push(widget::horizontal_space())
                                .push(widget::text::caption(&self.uuid)),
                        ),
                )
                .padding([cosmic.space_xs(), cosmic.space_xs()])
                .layer(cosmic::cosmic_theme::Layer::Primary),
            )
            .into()
    }
}
