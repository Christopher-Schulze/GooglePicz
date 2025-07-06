use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Program, Stroke, Text};
use iced::{Color, Rectangle, Size, Point};

#[derive(Debug, Clone)]
pub struct FaceRecognizer {
    faces: Vec<face_recognition::Face>,
    width: u32,
    height: u32,
}

impl FaceRecognizer {
    pub fn new(faces: Vec<face_recognition::Face>, width: u32, height: u32) -> Self {
        Self { faces, width, height }
    }

    pub fn view<'a, Message>(self) -> Canvas<'a, Message, Self> {
        Canvas::new(self)
    }
}

impl<Message> Program<Message> for FaceRecognizer {
    type State = ();

    fn draw(&self, _state: &Self::State, bounds: Rectangle, _cursor: iced::mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(bounds.size());
        let sx = bounds.width / self.width.max(1) as f32;
        let sy = bounds.height / self.height.max(1) as f32;
        for face in &self.faces {
            let (x, y, w, h) = face.rect;
            let path = Path::rectangle(
                Point::new(x as f32 * sx, y as f32 * sy),
                Size::new(w as f32 * sx, h as f32 * sy),
            );
            frame.stroke(&path, Stroke { color: Color::from_rgb(1.0, 0.0, 0.0), width: 2.0, ..Stroke::default() });
            if let Some(name) = &face.name {
                frame.fill_text(Text {
                    content: name.clone(),
                    position: Point::new(x as f32 * sx, (y as f32 * sy - 14.0).max(0.0)),
                    color: Color::from_rgb(1.0, 0.0, 0.0),
                    size: 16.0,
                    ..Default::default()
                });
            }
        }
        vec![frame.into_geometry()]
    }
}
