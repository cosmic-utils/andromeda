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
    LoadDrive(
        cosmic::widget::nav_bar::Id,
        udisks2::zbus::zvariant::OwnedObjectPath,
    ),
    DriveRead(cosmic::widget::nav_bar::Id, Drive),

    // === === === Operations === === ===
    OpenOperationDialog(super::operation::Operation),
    CancelOperation,
    ConfirmOperation,
    PerformOperation(super::drive::Drive),
    OperationFinish,

    // Drive Format
    OperationDriveFormatEraseMode(usize),
    OperationDriveFormatPTableType(usize),

    // Add Partition
    OperationAddPartitionSizeUpdate(String),
    OperationAddPartitionSizeSave,

    // Format Partition
    OperationPartitionFormatNameUpdate(String),
    OperationPartitionFormatToggleErase(bool),
    OperationPartitionFormatSelectFS(usize),
}
