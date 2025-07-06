use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Program, Stroke};
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
            let path = Path::rectangle(
                Point::new(face.bbox[0] as f32 * sx, face.bbox[1] as f32 * sy),
                Size::new(face.bbox[2] as f32 * sx, face.bbox[3] as f32 * sy),
            );
            frame.stroke(&path, Stroke { color: Color::from_rgb(1.0, 0.0, 0.0), width: 2.0, ..Stroke::default() });
        }
        vec![frame.into_geometry()]
    }
}
