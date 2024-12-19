#[derive(Clone, Debug)]
pub struct DriveID {
    pub model: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct DriveData {}

impl DriveData {
    pub fn populate(drive_id: &DriveID, conn: zbus::Connection) -> Self {
        todo!()
    }
}
