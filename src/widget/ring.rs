use cosmic::prelude::*;
use cosmic::widget::canvas::{self, Stroke};

pub struct RingSection {
    pub size: usize,
    pub index: usize,
}

pub struct Ring {
    pub sections: Vec<RingSection>,
    pub line_width: f32,
    pub selected_par: Option<usize>,
}

impl canvas::Program<crate::app::message::AppMessage, Theme> for Ring {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: cosmic::iced::Rectangle,
        _cursor: cosmic::iced_core::mouse::Cursor,
    ) -> Vec<cosmic::widget::canvas::Geometry<Renderer>> {
        let cosmic = theme.cosmic();
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let radius =
            frame.size().height.min(frame.size().width) * 0.5 - (1.0 + self.line_width * 0.5);
        let bg_circle = canvas::Path::circle(frame.center(), radius);

        frame.stroke(
            &bg_circle,
            Stroke::default()
                .with_color(cosmic.bg_color().into())
                .with_width(self.line_width),
        );

        let mut total = 0.0;
        for section in &self.sections {
            total += section.size as f64;
        }
        let mut offset = 0.0;
        for section in &self.sections {
            let scale = (section.size as f64 / total) as f32;
            let start_angle = cosmic::iced::Degrees(360.0 * offset).into();
            let end_angle = cosmic::iced::Degrees(360.0 * (offset + scale) - 1.0).into();
            let mut arc = canvas::path::Builder::new();
            arc.arc(canvas::path::Arc {
                center: frame.center(),
                radius,
                start_angle,
                end_angle,
            });
            offset += scale;

            frame.stroke(
                &arc.build(),
                Stroke::default()
                    .with_color(cosmic.accent_color().into())
                    .with_width(
                        if self.selected_par.is_some_and(|par| par == section.index) {
                            self.line_width * 1.5
                        } else {
                            self.line_width
                        },
                    ),
            )
        }

        vec![frame.into_geometry()]
    }
}
