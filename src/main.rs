mod app;

fn main() -> Result<(), cosmic::iced::Error> {
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
