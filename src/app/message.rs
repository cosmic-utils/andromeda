#[derive(Clone)]
pub enum AppMessage {
    NoOp,
    AppError(super::Error),
    DismissLastError,
    Quit,

    SelectPartition(usize),

    // DBus requests
    RefreshDrives,
    DriveLoad,

    // DBus Results
    ConnectionStarted(zbus::Connection),
    BlockDevicesLoaded(Vec<zbus::zvariant::OwnedObjectPath>),
    DrivesLoaded(Vec<super::drive::DriveID>),
    DriveLoaded(cosmic::widget::nav_bar::Id, super::drive::DriveData),
}

impl std::fmt::Debug for AppMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "App Message")
    }
}
