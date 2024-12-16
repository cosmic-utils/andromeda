pub mod message;
use std::sync::Arc;

use message::AppMessage;

pub struct App {
    core: cosmic::app::Core,
    drives: Vec<Arc<drives::Device>>,
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

    fn init(
        core: cosmic::app::Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::app::Task<Self::Message>) {
        let mut tasks: Vec<cosmic::app::Task<Self::Message>> = Vec::new();

        tasks.push(cosmic::task::future(async {
            match drives::get_devices() {
                Ok(devices) => {
                    let mut drives = Vec::new();
                    for device in devices {
                        drives.push(Arc::new(device));
                    }
                    AppMessage::DrivesLoaded(drives)
                }
                Err(e) => AppMessage::AppError(e.to_string()),
            }
        }));
        (
            Self {
                core,
                drives: Vec::new(),
            },
            cosmic::task::batch(tasks),
        )
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        match message {
            AppMessage::AppError(str) => panic!("{}", str),
            AppMessage::DrivesLoaded(drives) => self.drives = drives,
        }
        cosmic::Task::none()
    }

    fn view(&self) -> cosmic::Element<Self::Message> {
        cosmic::widget::layer_container(cosmic::widget::column::with_children(
            self.drives
                .iter()
                .filter(|drive_arc| drive_arc.model.is_some())
                .map(|drive_arc| {
                    cosmic::widget::row()
                        .push(cosmic::widget::text::heading("Name: "))
                        .push(cosmic::widget::text::heading(
                            drive_arc.model.as_ref().unwrap(),
                        ))
                        .into()
                })
                .collect(),
        ))
        .layer(cosmic::cosmic_theme::Layer::Primary)
        .into()
    }
}
