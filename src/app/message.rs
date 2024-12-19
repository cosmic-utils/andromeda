#[derive(Clone)]
pub enum AppMessage {
    NoOp,
    AppError(super::Error),
    DismissLastError,
    Quit,

    // DBus requests
    RefreshDrives,
    DriveLoad,

    // DBus Results
    ConnectionStarted(zbus::Connection),
    BlockDevicesLoaded(Vec<zbus::zvariant::OwnedObjectPath>),
    DrivesLoaded(Vec<super::drive::DriveID>),
    DriveLoaded(super::drive::DriveData),
}

impl std::fmt::Debug for AppMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "App Message")
    }
}
