use super::drive::Drive;

#[derive(Clone, Debug)]
pub enum AppMessage {
    NoOp,
    DismissLastError,
    Quit,

    InitClient,
    InitClientDone(udisks2::Client),

    ReadDevices,
    ReadDevicesDone(Vec<udisks2::zbus::zvariant::OwnedObjectPath>),

    InsertDrive(udisks2::zbus::zvariant::OwnedObjectPath),
    DriveRead(cosmic::widget::nav_bar::Id, Drive),

    // Actions
    Action(super::action::Action),
    ActionSelection(usize, usize),
    ConfirmAction,
    CancelAction,
    ActionDone,
}
