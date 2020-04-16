//! In the dialogue view, the full screen us used to display text and images.
//! It is mainly used to display conversations with paddlers to explain the story of Paddland.

use crate::game::story::scene::*;
use crate::game::Game;
use crate::gui::{
    decoration::draw_leaf_border, gui_components::*, shapes::PadlShapeIndex, sprites::*,
    ui_state::Now, utils::colors::LIGHT_BLUE, utils::*, z::*,
};
use crate::prelude::*;
use crate::view::Frame;
use quicksilver::graphics::{Mesh, ShapeRenderer};
use quicksilver::lyon::{math::point, path::Path, tessellation::*};
use quicksilver::prelude::Window as QuicksilverWindow;
use quicksilver::prelude::{Col, Rectangle, Transform};
use specs::WorldExt;
use std::marker::PhantomData;

pub(crate) struct DialogueFrame<'a, 'b> {
    left_area: Rectangle,
    active_area: Rectangle,
    buttons: UiBox,
    text_lines: Vec<String>,
    text_provider: TableTextProvider,
    text_bubble: Mesh,
    current_scene: Option<Scene>,
    phantom: PhantomData<(&'a (), &'b ())>,
}

impl<'a, 'b> DialogueFrame<'a, 'b> {
    pub fn new(area: &Rectangle) -> PadlResult<Self> {
        let (left_area, bubble_area) = area
            .shrink_to_center(0.875)
            .cut_vertical(0.38195 * area.size.x);

        let mut text_area = bubble_area.shrink_to_center(0.875);
        let d = 0.1875 * bubble_area.size.x;
        text_area.size.x -= d;
        text_area.pos.x += d;

        let text_bubble = build_text_bubble(bubble_area, text_area);
        let text_provider = TableTextProvider::new_styled("dialogue");

        let dialogue = DialogueFrame {
            active_area: text_area,
            buttons: UiBox::new(2, 1, 20.0, 15.0),
            left_area,
            text_lines: Vec::new(),
            text_provider,
            text_bubble,
            current_scene: None,
            phantom: PhantomData,
        };

        Ok(dialogue)
    }

    pub fn load_scene(&mut self, scene: Scene, locale: &Catalog) {
        self.current_scene = Some(scene);
        self.reload(locale);
    }
    #[inline(always)]
    fn scene_mut(&mut self) -> PadlResult<&mut Scene> {
        self.current_scene
            .as_mut()
            .ok_or(PadlError::dev_err(PadlErrorCode::DialogueEmpty))
    }
    #[inline(always)]
    fn scene(&mut self) -> PadlResult<&Scene> {
        self.current_scene
            .as_ref()
            .ok_or(PadlError::dev_err(PadlErrorCode::DialogueEmpty))
    }
    fn load_slide(&mut self, i: usize, locale: &Catalog) -> PadlResult<()> {
        let scene = self.scene_mut()?;
        scene.set_slide(i);
        self.reload(locale);
        Ok(())
    }
    /// Panics if no scene is loaded
    fn reload(&mut self, locale: &Catalog) {
        self.buttons.clear();
        self.text_lines.clear();

        // Write text into text bubble
        let key = self.current_scene.as_ref().unwrap().slide_text_key();
        let text = locale.gettext(key);
        for s in text.split("\n") {
            self.text_lines.push(s.to_owned());
        }

        // Create buttons
        if let Some(i) = self.scene().unwrap().back_button() {
            let back_button = UiElement::new(ClickOutput::Slide(i))
                .with_render_variant(RenderVariant::Shape(PadlShapeIndex::LeftArrow));
            self.buttons.add(back_button);
        } else {
            self.buttons.add(UiElement::empty());
        }
        if let Some(i) = self.scene().unwrap().next_button() {
            let next_button = UiElement::new(ClickOutput::Slide(i))
                .with_render_variant(RenderVariant::Shape(PadlShapeIndex::RightArrow));
            self.buttons.add(next_button);
        } else {
            self.buttons.add(UiElement::empty());
        }
    }

    pub fn draw_active_area(
        &mut self,
        sprites: &mut Sprites,
        now: Timestamp,
        window: &mut QuicksilverWindow,
    ) -> PadlResult<()> {
        let mut table = Vec::new();
        for s in &self.text_lines {
            table.push(TableRow::Text(s.to_owned()));
        }
        table.push(TableRow::InteractiveArea(&mut self.buttons));

        draw_table(
            window,
            sprites,
            &mut table,
            &self.active_area,
            &mut self.text_provider,
            60.0,
            Z_MENU_TEXT,
            now,
        )?;
        Ok(())
    }
    pub fn draw_background(
        &self,
        sprites: &mut Sprites,
        window: &mut QuicksilverWindow,
        main_area: Rectangle,
        resolution: ScreenResolution,
    ) {
        window.draw_ex(&main_area, Col(LIGHT_BLUE), Transform::IDENTITY, Z_TEXTURE);
        let leaf_w = resolution.leaves_border_w();
        let leaf_h = resolution.leaves_border_h();
        let mut leaf_area = main_area.clone();
        let dx = leaf_w / 2.0;
        leaf_area.pos.x += dx;
        leaf_area.size.x -= dx;
        draw_leaf_border(window, sprites, &leaf_area, leaf_w, leaf_h);
    }

    pub fn draw_roger_with_text_bubble(
        &self,
        sprites: &mut Sprites,
        window: &mut QuicksilverWindow,
    ) -> PadlResult<()> {
        draw_static_image(
            sprites,
            window,
            &self.left_area,
            SpriteIndex::Simple(SingleSprite::Roger),
            Z_TEXTURE + 1,
            FitStrategy::Center,
        )?;

        // let t = Transform::translate((x * self.shared.scaling, 0));
        let t = Transform::default();
        extend_transformed(window.mesh(), &self.text_bubble, t);

        Ok(())
    }
}

impl<'a, 'b> Frame for DialogueFrame<'a, 'b> {
    type Error = PadlError;
    type State = Game<'a, 'b>;
    type Graphics = QuicksilverWindow;
    type Event = PadlEvent;
    fn event(&mut self, state: &mut Self::State, e: &Self::Event) -> Result<(), Self::Error> {
        match e {
            PadlEvent::Scene => {
                self.load_scene(load_entry_scene(), &state.locale);
            }
            _ => {}
        }
        Ok(())
    }
    fn draw(
        &mut self,
        state: &mut Self::State,
        window: &mut Self::Graphics,
    ) -> Result<(), Self::Error> {
        self.text_provider.reset();
        let resolution = *state.world.read_resource::<ScreenResolution>();
        let main_area = Rectangle::new_sized(window.project() * window.screen_size());
        self.draw_background(&mut state.sprites, window, main_area, resolution);
        self.draw_roger_with_text_bubble(&mut state.sprites, window)?;
        self.draw_active_area(
            &mut state.sprites,
            state.world.read_resource::<Now>().0,
            window,
        )?;
        self.text_provider.finish_draw();
        Ok(())
    }
    fn left_click(&mut self, state: &mut Self::State, pos: (i32, i32)) -> Result<(), Self::Error> {
        if let Some(output) = self.buttons.click(pos.into())? {
            match output {
                (ClickOutput::Slide(i), None) => self.load_slide(i, &state.locale)?,
                _ => unimplemented!("Only slide transitions implemented"),
            }
        }
        Ok(())
    }
    fn enter(&mut self, _state: &mut Self::State) -> Result<(), Self::Error> {
        Ok(())
    }
    fn leave(&mut self, _state: &mut Self::State) -> Result<(), Self::Error> {
        self.text_provider.hide();
        Ok(())
    }
}

/// Creates a shape for tesselation that forms a left-open text bubble.
/// total_area: Maximum space that text bubble should use
/// text_area: Minimum space that text should have. Must be a subset of total_area.
pub fn build_text_bubble(total_area: Rectangle, text_area: Rectangle) -> Mesh {
    // Define start point
    let x0 = total_area.pos.x;
    let y0 = total_area.pos.y + total_area.size.y / 2.0;
    // Define text corners
    let left = text_area.pos.x;
    let top = text_area.pos.y;
    let right = text_area.pos.x + text_area.size.x;
    let bottom = text_area.pos.y + text_area.size.y;
    // Degree of curvature
    let s = text_area.size.x * 0.125;
    // Define control points for bezier curves
    let ctrl_x0 = text_area.pos.x;
    let ctrl_y0 = y0;
    let ctrl_x1 = text_area.pos.x + text_area.size.x / 2.0;
    let ctrl_y1 = text_area.pos.y - s;
    let ctrl_x2 = text_area.pos.x + text_area.size.x + s;
    let ctrl_y2 = text_area.pos.y + text_area.size.y + s;

    // Create enclosing path
    let mut builder = Path::builder();
    builder.move_to(point(x0, y0));

    builder.quadratic_bezier_to(point(ctrl_x0, ctrl_y0), point(left, top));
    builder.quadratic_bezier_to(point(ctrl_x1, ctrl_y1), point(right, top));
    builder.quadratic_bezier_to(point(ctrl_x2, ctrl_y0), point(right, bottom));
    builder.quadratic_bezier_to(point(ctrl_x1, ctrl_y2), point(left, bottom));
    builder.quadratic_bezier_to(point(ctrl_x0, ctrl_y0), point(x0, y0));
    builder.close();

    let path = builder.build();

    // Tesselate path to mesh
    let mut mesh = Mesh::new();
    let mut tessellator = FillTessellator::new();
    let mut shape = ShapeRenderer::new(&mut mesh, WHITE);
    shape.set_z((Z_TEXTURE + 2) as f32);

    tessellator
        .tessellate_path(path.into_iter(), &FillOptions::default(), &mut shape)
        .unwrap();

    mesh
}
