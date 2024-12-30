//pub mod action;
pub mod drive;
pub mod error;
pub mod message;
pub mod operation;

use error::Error;
use message::AppMessage;

use cosmic::{cosmic_theme, iced, prelude::*, widget};
use udisks2::zbus::zvariant::OwnedObjectPath;

pub struct App {
    core: cosmic::app::Core,
    nav_model: cosmic::widget::nav_bar::Model,

    active_drive: Option<OwnedObjectPath>,
    client: Option<udisks2::Client>,
    current_operation: Option<Box<dyn operation::OperationDialog>>,
    pending: bool,

    errors: Vec<Error>,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::multi::Executor;
    type Flags = ();
    type Message = Result<AppMessage, Error>;

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
        self.active_drive = if let Some(drive) = self.nav_model.active_data::<drive::Drive>() {
            Some(drive.block_path.clone())
        } else {
            None
        };
        cosmic::Task::none()
    }

    fn init(
        core: cosmic::app::Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::app::Task<Self::Message>) {
        let mut tasks: Vec<cosmic::app::Task<Self::Message>> = Vec::new();
        let nav_model = cosmic::widget::nav_bar::Model::default();

        tasks.push(cosmic::task::message(Ok(AppMessage::InitClient)));
        (
            Self {
                core,
                nav_model,
                active_drive: None,
                client: None,
                current_operation: None,
                pending: false,
                errors: Vec::new(),
            },
            cosmic::task::batch(tasks),
        )
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        let mut tasks = Vec::new();
        match message {
            Ok(message) => {
                if let Some(operation) = &mut self.current_operation {
                    tasks.push(operation.update(message.clone()))
                }
                match message {
                    AppMessage::NoOp => {}
                    AppMessage::DismissLastError => std::mem::drop(self.errors.pop()),
                    AppMessage::Quit => std::process::exit(0),

                    AppMessage::InitClient => {
                        tasks.push(cosmic::task::future(async move {
                            Ok(AppMessage::InitClientDone(udisks2::Client::new().await?))
                        }));
                    }
                    AppMessage::InitClientDone(client) => {
                        self.client = Some(client);
                        tasks.push(cosmic::task::message(Ok(AppMessage::ReadDevices)));
                    }

                    AppMessage::InsertDrive(block_path) => {
                        let entity = self.nav_model.insert().id();
                        tasks.push(cosmic::task::message(Ok(AppMessage::LoadDrive(
                            entity, block_path,
                        ))));
                    }

                    AppMessage::LoadDrive(id, block_path) => {
                        if let Some(client) = self.client.clone() {
                            tasks.push(cosmic::task::future(drive::Drive::load(
                                client, id, block_path,
                            )));
                        }
                    }

                    AppMessage::DriveRead(id, drive) => {
                        self.nav_model.text_set(id, drive.model.clone());
                        self.nav_model.icon_set(
                            id,
                            widget::icon::from_name("drive-harddisk-system-symbolic").icon(),
                        );
                        self.nav_model.data_set(id, drive);
                    }

                    AppMessage::ReadDevices => {
                        let client = self.client.clone();

                        tasks.push(cosmic::task::future(async move {
                            match client {
                                Some(client) => Ok(AppMessage::ReadDevicesDone(
                                    client
                                        .manager()
                                        .get_block_devices(udisks2::standard_options(false))
                                        .await?,
                                )),
                                None => Err(Error::new(
                                    "Client not initialized, this is a bug, please report it!",
                                    false,
                                )),
                            }
                        }))
                    }

                    AppMessage::ReadDevicesDone(blocks) => {
                        for block_path in blocks {
                            if let Some(client) = self.client.clone() {
                                tasks.push(cosmic::task::future(async move {
                                    if client
                                        .object(block_path.clone())
                                        .unwrap()
                                        .partition()
                                        .await
                                        .is_ok()
                                        || client
                                            .object(block_path.clone())
                                            .unwrap()
                                            .r#loop()
                                            .await
                                            .is_ok()
                                        || client
                                            .object(block_path.clone())
                                            .unwrap()
                                            .swapspace()
                                            .await
                                            .is_ok()
                                    {
                                        Ok(AppMessage::NoOp)
                                    } else {
                                        Ok(AppMessage::InsertDrive(block_path))
                                    }
                                }));
                            };
                        }
                    }
                    AppMessage::OpenOperationDialog(operation_type) => {
                        self.current_operation = Some(operation_type.into());
                    }
                    AppMessage::PerformOperation(_) => self.current_operation = None,
                    AppMessage::ConfirmOperation => {
                        if let Some(drive) = self.nav_model.active_data::<drive::Drive>() {
                            tasks.push(cosmic::task::message(Ok(AppMessage::PerformOperation(
                                drive.clone(),
                            ))));
                            self.pending = true;
                        }
                    }
                    AppMessage::CancelOperation => {
                        self.current_operation = None;
                    }
                    AppMessage::OperationFinish => {
                        self.pending = false;
                        self.current_operation = None;
                        if let Some(drive) = self.nav_model.active_data::<drive::Drive>() {
                            tasks.push(cosmic::task::message(Ok(AppMessage::LoadDrive(
                                self.nav_model.active(),
                                drive.block_path.clone(),
                            ))))
                        }
                    }

                    _ => {}
                }
            }
            Err(err) => self.errors.push(err),
        }
        cosmic::Task::batch(tasks)
    }

    fn header_start(&self) -> Vec<Element<Self::Message>> {
        if let Some(active_drive) = self.nav_model.active_data::<drive::Drive>() {
            vec![active_drive.menu_bar()]
        } else {
            Vec::new()
        }
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
                                .on_press(Ok(AppMessage::DismissLastError)),
                        )
                        .into(),
                )
            } else {
                Some(
                    cosmic::widget::dialog()
                        .title("Critical")
                        .body(&error.description)
                        .primary_action(
                            cosmic::widget::button::destructive("Quit")
                                .on_press(Ok(AppMessage::Quit)),
                        )
                        .into(),
                )
            }
        } else if let Some(action) = &self.current_operation {
            Some(action.dialog())
        } else if self.pending {
            Some(
                cosmic::widget::dialog()
                    .title("Please Wait...")
                    .body("Writing to disk.")
                    .into(),
            )
        } else {
            None
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        match self.nav_model.active_data::<drive::Drive>() {
            Some(drive) => widget::layer_container(drive.view())
                .width(iced::Length::Fill)
                .layer(cosmic_theme::Layer::Background)
                .into(),
            None => widget::text::title4("Please select a drive")
                .apply(widget::layer_container)
                .align_x(iced::Alignment::Center)
                .padding([cosmic.space_l(), cosmic.space_xs()])
                .width(iced::Length::Fill)
                .layer(cosmic_theme::Layer::Primary)
                .into(),
        }
    }
}
