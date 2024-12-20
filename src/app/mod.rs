pub mod drive;
pub mod error;
pub mod message;

use error::Error;
use message::AppMessage;

use cosmic::prelude::*;

pub struct App {
    core: cosmic::app::Core,
    nav_model: cosmic::widget::nav_bar::Model,

    udisks2: Option<zbus::Connection>,

    errors: Vec<Error>,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::multi::Executor;
    type Flags = ();
    type Message = AppMessage;

    const APP_ID: &'static str = "io.github.cosmic_utils.andromeda";

    fn core(&self) -> &cosmic::app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::app::Core {
        &mut self.core
    }

    fn nav_model(&self) -> Option<&cosmic::widget::nav_bar::Model> {
        Some(&self.nav_model)
    }

    fn on_nav_select(
        &mut self,
        id: cosmic::widget::nav_bar::Id,
    ) -> cosmic::app::Task<Self::Message> {
        self.nav_model.activate(id);
        cosmic::task::message(AppMessage::DriveLoad)
    }

    fn init(
        core: cosmic::app::Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::app::Task<Self::Message>) {
        let mut tasks: Vec<cosmic::app::Task<Self::Message>> = Vec::new();
        let nav_model = cosmic::widget::nav_bar::Model::default();

        tasks.push(
            cosmic::task::future(async {
                let conn = zbus::connection::Builder::system()?
                    .auth_mechanism(zbus::AuthMechanism::External)
                    .build()
                    .await?;

                Result::<zbus::Connection, zbus::Error>::Ok(conn)
            })
            .map(|result| match result {
                Ok(conn) => cosmic::app::message::app(AppMessage::ConnectionStarted(conn)),
                Err(e) => cosmic::app::message::app(AppMessage::AppError(Error::from_err(
                    Box::new(e),
                    false,
                ))),
            }),
        );

        tasks.push(cosmic::task::future(async { AppMessage::NoOp }));
        (
            Self {
                core,
                nav_model,
                udisks2: None,
                errors: Vec::new(),
            },
            cosmic::task::batch(tasks),
        )
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        let mut tasks = Vec::new();
        match message {
            AppMessage::NoOp => {}
            AppMessage::AppError(err) => self.errors.push(err),
            AppMessage::DismissLastError => std::mem::drop(self.errors.pop()),
            AppMessage::Quit => std::process::exit(0),

            AppMessage::SelectPartition(partition) => {
                if let Some(drive_data) = self.nav_model.active_data_mut::<drive::DriveData>() {
                    drive_data.selected_partition(partition);
                }
            }

            // DBus requests
            AppMessage::RefreshDrives => {
                match self.udisks2.clone() {
                    Some(conn) => {
                        tasks.push(
                            cosmic::task::future(async move {
                                let proxy = zbus::Proxy::new(
                                    &conn,
                                    "org.freedesktop.UDisks2",
                                    "/org/freedesktop/UDisks2/Manager",
                                    "org.freedesktop.UDisks2.Manager",
                                )
                                .await?;

                                let devices: Vec<zbus::zvariant::OwnedObjectPath> =
                                    proxy
                                        .call(
                                            "GetBlockDevices",
                                            &std::collections::HashMap::<
                                                String,
                                                zbus::zvariant::Value,
                                            >::new(),
                                        )
                                        .await?;

                                Result::<Vec<zbus::zvariant::OwnedObjectPath>, zbus::Error>::Ok(
                                    devices,
                                )
                            })
                            .map(|result| match result {
                                Ok(devices) => {
                                    // Filter devices by ones that contain
                                    cosmic::app::message::app(AppMessage::BlockDevicesLoaded(
                                        devices,
                                    ))
                                }
                                Err(e) => cosmic::app::message::app(AppMessage::AppError(
                                    Error::from_err(Box::new(e), false),
                                )),
                            }),
                        )
                    }
                    None => tasks.push(cosmic::task::message(AppMessage::AppError(Error::new(
                        "Could not refresh: DBus Interface to UDisks2 not initialized!",
                        false,
                    )))),
                }
            }
            AppMessage::DriveLoad => {
                let active_id = self.nav_model.active();
                match self.udisks2.clone() {
                    Some(udisks2) => match self.nav_model.active_data::<drive::DriveID>().cloned() {
                        Some(drive_id) => tasks.push(cosmic::task::future(
                            drive::DriveData::populate(drive_id, udisks2),
                        ).map(move |result| match result {
                            Ok(drive_data) =>
                                cosmic::app::message::app(AppMessage::DriveLoaded(active_id, drive_data)),
                            Err(e) => cosmic::app::message::app(AppMessage::AppError(Error::from_err(Box::new(e), false)))
                        }

                        )),
                        None => tasks.push(cosmic::task::message(AppMessage::AppError(Error::new(
                            "Selected drive has no drive ID data, if this is showing, there is a bug in the application",
                            false
                        ))))
                    },
                    None => tasks.push(cosmic::task::message(AppMessage::AppError(Error::new(
                        "Connection not initialized yet!",
                        true,
                    )))),
                }
            }
            // DBus Results
            AppMessage::ConnectionStarted(conn) => {
                self.udisks2 = Some(conn);
                tasks.push(cosmic::task::message(AppMessage::RefreshDrives))
            }

            AppMessage::BlockDevicesLoaded(devices) => match self.udisks2.clone() {
                Some(conn) => tasks.push(
                    cosmic::task::future(async move {
                        let mut drives = Vec::new();
                        for device in devices {
                            let pt_proxy = zbus::Proxy::new(
                                &conn,
                                "org.freedesktop.UDisks2",
                                device.as_str(),
                                "org.freedesktop.UDisks2.PartitionTable",
                            )
                            .await?;
                            if let Ok(_) =
                                pt_proxy.get_property::<zbus::zvariant::Str>("Type").await
                            {
                                let dev_proxy = zbus::Proxy::new(
                                    &conn,
                                    "org.freedesktop.UDisks2",
                                    device.as_str(),
                                    "org.freedesktop.UDisks2.Block",
                                )
                                .await?;
                                let resp: zbus::zvariant::OwnedObjectPath =
                                    dev_proxy.get_property("Drive").await?;

                                let drive_path = resp.as_str();
                                if drive_path != "/" {
                                    let drive_proxy = zbus::Proxy::new(
                                        &conn,
                                        "org.freedesktop.UDisks2",
                                        drive_path,
                                        "org.freedesktop.UDisks2.Drive",
                                    )
                                    .await?;
                                    let drive_model: zbus::zvariant::Str =
                                        drive_proxy.get_property("Model").await?;
                                    drives.push(drive::DriveID {
                                        model: drive_model.to_string(),
                                        block_path: device.to_string(),
                                        drive_path: drive_path.to_string(),
                                    })
                                }
                            }
                        }

                        Result::<Vec<drive::DriveID>, zbus::Error>::Ok(drives)
                    })
                    .map(|res| match res {
                        Ok(drives) => cosmic::app::message::app(AppMessage::DrivesLoaded(drives)),
                        Err(e) => cosmic::app::message::app(AppMessage::AppError(Error::from_err(
                            Box::new(e),
                            false,
                        ))),
                    }),
                ),
                None => tasks.push(cosmic::task::message(AppMessage::AppError(Error::new(
                    "ZBus connection not started yet!",
                    false,
                )))),
            },
            AppMessage::DrivesLoaded(drives) => {
                for drive in drives {
                    self.nav_model
                        .insert()
                        .text(drive.model.clone())
                        .icon(cosmic::widget::icon::from_name(
                            "drive-harddisk-system-symbolic",
                        ))
                        .data(drive);
                }
            }
            AppMessage::DriveLoaded(entity, drive_data) => {
                self.nav_model.data_set(entity, drive_data)
            }
        }
        cosmic::Task::batch(tasks)
    }

    fn dialog(&self) -> Option<Element<Self::Message>> {
        if self.errors.len() > 0 {
            let error = self.errors.last().unwrap();
            if error.recoverable {
                Some(
                    cosmic::widget::dialog()
                        .title("Warning")
                        .body(&error.description)
                        .primary_action(
                            cosmic::widget::button::suggested("Dismiss")
                                .on_press(AppMessage::DismissLastError),
                        )
                        .into(),
                )
            } else {
                Some(
                    cosmic::widget::dialog()
                        .title("Critical")
                        .body(&error.description)
                        .primary_action(
                            cosmic::widget::button::destructive("Quit").on_press(AppMessage::Quit),
                        )
                        .into(),
                )
            }
        } else {
            None
        }
    }

    fn view(&self) -> Element<Self::Message> {
        cosmic::widget::layer_container(
            match self.nav_model.active_data::<drive::DriveID>().cloned() {
                Some(_) => match self.nav_model.active_data::<drive::DriveData>() {
                    Some(drive_data) => cosmic::widget::layer_container(drive_data.view())
                        .layer(cosmic::cosmic_theme::Layer::Background),
                    None => cosmic::widget::layer_container(cosmic::widget::text::title4(
                        "Loading drive data...",
                    ))
                    .layer(cosmic::cosmic_theme::Layer::Background),
                },
                None => cosmic::widget::layer_container(cosmic::widget::text::title1(
                    "Please Select a Drive",
                ))
                .layer(cosmic::cosmic_theme::Layer::Background),
            }
            .width(cosmic::iced::Length::Fill),
        )
        .layer(cosmic::cosmic_theme::Layer::Background)
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .into()
    }
}
