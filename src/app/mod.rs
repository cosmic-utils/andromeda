pub mod action;
pub mod drive;
pub mod error;
pub mod message;

use error::Error;
use message::AppMessage;

use cosmic::{cosmic_theme, iced, prelude::*, widget};

pub struct App {
    core: cosmic::app::Core,
    nav_model: cosmic::widget::nav_bar::Model,

    client: Option<udisks2::Client>,
    current_action: Option<action::Action>,
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
                client: None,
                current_action: None,
                pending: false,
                errors: Vec::new(),
            },
            cosmic::task::batch(tasks),
        )
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        let mut tasks = Vec::new();
        match message {
            Ok(message) => match message {
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
                    if let Some(client) = self.client.clone() {
                        tasks.push(cosmic::task::future(drive::Drive::load(client, entity, block_path)));
                    }
                }

                AppMessage::DriveRead(id, drive) => {
                    self.nav_model.text_set(id, drive.model.clone());
                    self.nav_model.icon_set(id, widget::icon::from_name("drive-harddisk-system-symbolic").icon());
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
                            tasks.push(
                                cosmic::task::future(async move {
                                    if client.object(block_path.clone()).unwrap().partition().await.is_ok() ||
                                    client.object(block_path.clone()).unwrap().r#loop().await.is_ok() ||
                                    client.object(block_path.clone()).unwrap().swapspace().await.is_ok() {
                                        Ok(AppMessage::NoOp)
                                    } else {
                                        Ok(AppMessage::InsertDrive(block_path))
                                    }
                                })
                            );
                        };
                    }
                }
                AppMessage::Action(action) => {
                    self.current_action = Some(action);
                }
                AppMessage::ActionSelection(a, b) => match self.current_action.as_mut() {
                    Some(action) => action.on_option_select(a, b),
                    None => tasks.push(cosmic::task::message(Err(Error::new("No active action yet user tried to perform an action, this is a bug, please report and post what you did to recreate this error", false))))
                },
                AppMessage::ConfirmAction => {
                    self.pending = true;
                    tasks.push(match self.nav_model.active_data::<drive::Drive>() {
                        Some(drive) => {
                            match self.current_action.take() {
                                Some(mut action) => action.on_action(drive.block.clone()),
                                None => cosmic::task::message(Err(Error::new("Could not get current action even though confirm action message was sent, this is a bug! Please report", false)))
                            }.chain(cosmic::task::future(drive::Drive::load(self.client.clone().unwrap(), self.nav_model.active(), drive.block_path.clone())))
                        }
                        None => cosmic::task::message(Err(Error::new("Could not get active data on current page, this is a bug!", false)))
                    });
                }
                AppMessage::CancelAction => {
                    self.current_action = None;
                }
                AppMessage::ActionDone => {
                    self.pending = false;
                }
            },
            Err(err) => self.errors.push(err),
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
        } else if let Some(action) = &self.current_action {
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
            None => widget::layer_container(widget::text::title4("Please select a drive"))
                .align_x(iced::Alignment::Center)
                .padding([cosmic.space_l(), cosmic.space_xs()])
                .width(iced::Length::Fill)
                .layer(cosmic_theme::Layer::Primary)
                .into(),
        }
    }
}
