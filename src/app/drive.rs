use cosmic::prelude::*;
use zbus::zvariant;

use super::message::AppMessage;

#[derive(Clone, Debug)]
pub struct DriveID {
    pub model: String,
    pub block_path: String,
    pub drive_path: String,
}

#[derive(Clone, Debug)]
pub struct PartitionData {
    number: u32,
    name: String,
    fs: String,
    offset: u64,
    size: u64,
}

#[derive(Clone, Debug)]
pub struct DriveData {
    partitions: Vec<PartitionData>,
    selected_partition: Option<usize>,

    serial: String,
    revision: String,
    model: String,
    size: u64,
    ptype: String,
}

const KI_B: u64 = 1024;
const MI_B: u64 = 1024 * KI_B;
const GI_B: u64 = 1024 * MI_B;
const TI_B: u64 = 1024 * GI_B;
const PI_B: u64 = 1024 * TI_B;

impl DriveData {
    pub async fn populate(drive_id: DriveID, conn: zbus::Connection) -> Result<Self, zbus::Error> {
        let drive_proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.UDisks2",
            drive_id.drive_path.as_str(),
            "org.freedesktop.UDisks2.Drive",
        )
        .await?;

        let block_proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.UDisks2",
            drive_id.block_path.as_str(),
            "org.freedesktop.UDisks2.Block",
        )
        .await?;

        let ptable_proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.UDisks2",
            drive_id.block_path.as_str(),
            "org.freedesktop.UDisks2.PartitionTable",
        )
        .await?;

        // Read serial
        let serial = drive_proxy
            .get_property::<zvariant::Str>("Serial")
            .await?
            .to_string();
        // Read revision
        let revision = drive_proxy
            .get_property::<zvariant::Str>("Revision")
            .await?
            .to_string();
        // Read size
        let size = block_proxy
            .get_property::<zvariant::Value>("Size")
            .await?
            .downcast()?;
        // Read ptype
        let ptype = ptable_proxy
            .get_property::<zvariant::Str>("Type")
            .await?
            .to_string();

        // Read partitions
        let ptable: Vec<zvariant::Value> = ptable_proxy
            .get_property::<zvariant::Array>("Partitions")
            .await?
            .to_vec();
        let mut partitions = Vec::new();
        for value in ptable {
            let partition_path = value.downcast::<zvariant::ObjectPath>()?;
            let partition_proxy = zbus::Proxy::new(
                &conn,
                "org.freedesktop.UDisks2",
                partition_path.as_str(),
                "org.freedesktop.UDisks2.Partition",
            )
            .await?;
            let block_proxy = zbus::Proxy::new(
                &conn,
                "org.freedesktop.UDisks2",
                partition_path.as_str(),
                "org.freedesktop.UDisks2.Block",
            )
            .await?;
            let _fs_proxy = zbus::Proxy::new(
                &conn,
                "org.freedesktop.UDisks2",
                partition_path.as_str(),
                "org.freedesktop.UDisks2.FileSystem",
            )
            .await?;

            let number = partition_proxy
                .get_property::<zvariant::Value>("Number")
                .await?
                .downcast()?;

            let name = std::path::Path::new(partition_path.as_str())
                .file_name()
                .ok_or(zbus::Error::Failure(
                    "Could not parse partition path!".to_string(),
                ))?
                .to_string_lossy()
                .to_string();

            let fs = block_proxy
                .get_property::<zvariant::Str>("IdType")
                .await?
                .to_string();

            let offset = partition_proxy
                .get_property::<zvariant::Value>("Offset")
                .await?
                .downcast()?;

            let par_size = partition_proxy
                .get_property::<zvariant::Value>("Size")
                .await?
                .downcast()?;

            partitions.push(PartitionData {
                number,
                name,
                fs,
                offset,
                size: par_size,
            })
        }

        partitions.sort_by(|a, b| a.number.cmp(&b.number));

        Ok(Self {
            partitions,
            selected_partition: None,

            model: drive_id.model,
            serial,
            revision,
            size,
            ptype,
        })
    }
    pub fn view(&self) -> Element<super::message::AppMessage> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        cosmic::widget::layer_container(
            cosmic::widget::column()
                .spacing(cosmic.space_xs())
                .push(
                    cosmic::widget::row()
                        .spacing(cosmic.space_xs())
                        .push(self.ring(&cosmic))
                        .push(self.info(&cosmic)),
                )
                .push(self.partition_table(&cosmic)),
        )
        .layer(cosmic::cosmic_theme::Layer::Background)
        .into()
    }

    pub fn selected_partition(&mut self, partition: usize) {
        self.selected_partition = Some(partition);
    }

    fn ring(&self, cosmic: &cosmic::theme::CosmicTheme) -> Element<AppMessage> {
        cosmic::widget::layer_container(
            cosmic::widget::canvas(crate::widget::Ring {
                sections: self
                    .partitions
                    .iter()
                    .enumerate()
                    .map(|(index, partition)| {
                        let size = partition.size as f64 / MI_B as f64;
                        let log_size = size.log(128.0).powf(4.0) as usize;

                        crate::widget::RingSection {
                            index,
                            size: log_size,
                        }
                    })
                    .collect(),
                line_width: 10.,
                selected_par: self.selected_partition,
            })
            .height(cosmic::iced::Length::Fill)
            .width(cosmic::iced::Length::Fill),
        )
        .padding([cosmic.space_l(), cosmic.space_l()])
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .layer(cosmic::cosmic_theme::Layer::Primary)
        .into()
    }

    fn info(&self, cosmic: &cosmic::theme::CosmicTheme) -> Element<AppMessage> {
        if let Some(index) = self.selected_partition {
            let partition = &self.partitions[index];
            cosmic::widget::layer_container(
                cosmic::widget::column()
                    .spacing(cosmic.space_s())
                    .push(cosmic::widget::text::title4(partition.name.clone()))
                    .push(cosmic::iced_widget::horizontal_rule(1))
                    .push(self.info_item("Filesystem".to_string(), partition.fs.clone()))
                    .push(self.info_item("Offset".to_string(), format_bytes(partition.offset)))
                    .push(self.info_item("Size".to_string(), format_bytes(partition.size))),
            )
            .padding([cosmic.space_xs(), cosmic.space_s()])
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .layer(cosmic::cosmic_theme::Layer::Primary)
            .into()
        } else {
            cosmic::widget::layer_container(
                cosmic::widget::column()
                    .spacing(cosmic.space_s())
                    .push(cosmic::widget::text::title4(self.model.clone()))
                    .push(cosmic::iced_widget::horizontal_rule(1))
                    .push(self.info_item("Serial".to_string(), self.serial.clone()))
                    .push(self.info_item("Revision".to_string(), self.revision.clone()))
                    .push(self.info_item("Size".to_string(), format_bytes(self.size.clone())))
                    .push(self.info_item("Partitioning".to_string(), self.ptype.clone())),
            )
            .padding([cosmic.space_xs(), cosmic.space_s()])
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .layer(cosmic::cosmic_theme::Layer::Primary)
            .into()
        }
    }

    fn info_item(&self, field: String, value: String) -> cosmic::widget::Row<AppMessage> {
        cosmic::widget::row()
            .push(cosmic::widget::text::body(field.clone()))
            .push(cosmic::widget::horizontal_space())
            .push(cosmic::widget::text::body(value.clone()))
    }

    fn partition_table(&self, cosmic: &cosmic::theme::CosmicTheme) -> Element<AppMessage> {
        cosmic::widget::layer_container(
            cosmic::widget::column::with_children(
                self.partitions
                    .iter()
                    .enumerate()
                    .map(|(index, partition)| {
                        cosmic::widget::button::standard(partition.name.clone())
                            .width(cosmic::iced::Length::Fill)
                            .on_press(AppMessage::SelectPartition(index))
                            .into()
                    })
                    .collect(),
            )
            .spacing(cosmic.space_s()), // TODO, add partition table
        )
        .padding([cosmic.space_xs(), cosmic.space_s()])
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .layer(cosmic::cosmic_theme::Layer::Primary)
        .into()
    }
}

fn format_bytes(size_unformatted: u64) -> String {
    if size_unformatted >= PI_B {
        format!("{:.1} PiB", (size_unformatted as f64 / PI_B as f64) as f64,)
    } else if size_unformatted >= TI_B {
        format!("{:.1} TiB", (size_unformatted as f64 / TI_B as f64) as f64,)
    } else if size_unformatted >= GI_B {
        format!("{:.1} GiB", (size_unformatted as f64 / GI_B as f64) as f64,)
    } else if size_unformatted >= MI_B {
        format!("{:.1} MiB", (size_unformatted as f64 / MI_B as f64) as f64,)
    } else if size_unformatted >= KI_B {
        format!("{:.1} KiB", (size_unformatted as f64 / KI_B as f64) as f64,)
    } else {
        format!("{} Bytes", size_unformatted)
    }
}
