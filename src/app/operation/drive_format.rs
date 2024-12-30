use crate::app::{error::Error, message::AppMessage};
use cosmic::{prelude::*, widget};

#[derive(Debug, Clone)]
pub struct DriveFormat {
    erase: Option<usize>,
    ptable: Option<usize>,
}

impl DriveFormat {
    pub fn new() -> Self {
        Self {
            erase: Some(0),
            ptable: Some(0),
        }
    }
}

impl super::OperationDialog for DriveFormat {
    fn update(&mut self, message: AppMessage) -> cosmic::app::Task<Result<AppMessage, Error>> {
        let mut tasks = Vec::new();
        match message {
            AppMessage::OperationDriveFormatEraseMode(mode) => self.erase = Some(mode),
            AppMessage::OperationDriveFormatPTableType(table_type) => {
                self.ptable = Some(table_type)
            }
            AppMessage::PerformOperation(drive) => {
                let block = drive.block.clone();
                let erase = self.erase.unwrap();
                let ptable = self.ptable.unwrap();
                tasks.push(cosmic::task::future(async move {
                    let mut options = udisks2::standard_options(false);
                    if erase == 1 {
                        use udisks2::zbus::zvariant;
                        options.insert(
                            "erase",
                            zvariant::Value::Str(zvariant::Str::from_static("zero")),
                        );
                    }
                    let ptype = match ptable {
                        0 => "gpt",
                        1 => "dos",
                        _ => "empty",
                    };

                    block.format(ptype, options).await?;

                    Ok(AppMessage::OperationFinish)
                }));
            }
            _ => {}
        }
        cosmic::app::Task::batch(tasks)
    }

    fn dialog(&self) -> Element<Result<AppMessage, crate::app::error::Error>> {
        use widget::settings;

        widget::dialog()
            .title("Format Drive")
            .body(
                "This operation is not reversible, make sure you back up any important user data!",
            )
            .control(
                settings::section()
                    .add(settings::item(
                        "Erase Mode",
                        widget::dropdown(
                            &["Quick (less secure, faster)", "Full (more secure, slower)"],
                            self.erase,
                            |index| Ok(AppMessage::OperationDriveFormatEraseMode(index)),
                        ),
                    ))
                    .add(settings::item(
                        "Partitioning Method",
                        widget::dropdown(
                            &[
                                "GUID Partition Table (Modern)",
                                "Master Boot Record (Legacy)",
                                "Empty",
                            ],
                            self.ptable,
                            |index| Ok(AppMessage::OperationDriveFormatPTableType(index)),
                        ),
                    )),
            )
            .primary_action(
                widget::button::destructive("Confirm").on_press(Ok(AppMessage::ConfirmOperation)),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(Ok(AppMessage::CancelOperation)),
            )
            .into()
    }
}
