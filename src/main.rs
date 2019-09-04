#[macro_use]
extern crate lalrpop_util;
lalrpop_mod!(pub grammar); // synthesized by LALRPOP

mod ast;

use regex::Regex;
use std::fs::File;
use std::io::prelude::*;

use glutin::GlContext;
use nanovg::{Alignment, Color, Font, Frame, Gradient, TextOptions};

use std::error::Error;

use glam::{vec2, Vec2};

const INIT_WINDOW_SIZE: (u32, u32) = (500, 300);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct WidgetId(usize);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct WidgetUid(Vec<WidgetId>);

#[derive(Debug)]
enum Widget {
    Button(String),
    Label(String),
    Horizontal,
    Vertical,
}

#[derive(Debug)]
struct UiNode {
    widget: Widget,
    string_uid: Option<String>,
    children: Vec<(WidgetId, UiNode)>,
    next_child_id: WidgetId,
}

impl From<Widget> for UiNode {
    fn from(w: Widget) -> Self {
        UiNode::new(w)
    }
}

struct UiContext<'a> {
    uid: WidgetUid,
    interaction_state: &'a UiInteractionState,
}

impl<'a> UiContext<'a> {
    fn nested(&self, id: WidgetId) -> Self {
        let mut uid = self.uid.clone();
        uid.0.push(id);

        Self {
            uid,
            interaction_state: self.interaction_state,
        }
    }
}

struct Ui<'a, 'b> {
    node: &'a mut UiNode,
    context: UiContext<'b>,
}

#[derive(Debug)]
struct WidgetNotFoundError;

impl Error for WidgetNotFoundError {}
impl std::fmt::Display for WidgetNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not find widget")
    }
}

type UiResult<T> = Result<T, Box<dyn Error>>;

impl UiNode {
    fn new(widget: Widget) -> Self {
        Self {
            widget,
            string_uid: None,
            children: Vec::new(),
            next_child_id: WidgetId(0),
        }
    }

    fn id<'a>(
        &'a mut self,
        label: &str,
        uid_prefix: &WidgetUid,
    ) -> UiResult<(WidgetUid, &'a mut UiNode)> {
        if let Some(ref s) = self.string_uid {
            if s == label {
                return Ok((uid_prefix.clone(), self));
            }
        }

        for (id, ch) in self.children.iter_mut() {
            let mut uid_prefix = uid_prefix.clone();
            uid_prefix.0.push(id.clone());

            if let Ok(res) = ch.id(label, &uid_prefix) {
                return Ok(res);
            }
        }

        Err(Box::new(WidgetNotFoundError))
    }
}

impl<'a, 'b> Ui<'a, 'b> {
    fn new(node: &'a mut UiNode, context: UiContext<'b>) -> Self {
        Self { node, context }
    }

    fn clicked(&self) -> bool {
        self.context.interaction_state.mouse_released
            && Some(&self.context.uid) == self.context.interaction_state.hover_widget.as_ref()
            && Some(&self.context.uid) == self.context.interaction_state.drag_begin_widget.as_ref()
    }

    fn id(&mut self, label: &str) -> UiResult<Ui<'_, '_>> {
        let interaction_state = self.context.interaction_state;

        self.node.id(label, &self.context.uid).map(|(uid, n)| {
            Ui::new(
                n,
                UiContext {
                    interaction_state,
                    uid,
                },
            )
        })
    }

    fn append<'s, 'r>(&'s mut self, child: impl Into<UiNode>) -> Ui<'r, 'b>
    where
        's: 'r,
    {
        let id = self.node.next_child_id;
        self.node.next_child_id.0 += 1;
        self.node.children.push((id, child.into()));

        let len = self.node.children.len();
        let last = &mut self.node.children[len - 1];

        Ui {
            node: &mut last.1,
            context: self.context.nested(last.0),
        }
    }

    // syntax sugar
    fn button(&mut self, label: &str) -> Ui<'_, '_> {
        self.append(Widget::Button(label.to_owned()))
    }

    // syntax sugar
    fn label(&mut self, label: &str) -> Ui<'_, '_> {
        self.append(Widget::Label(label.to_owned()))
    }
}

/*
// syntax sugar
fn button<'b, 's, 'r>(ui: &'s mut Ui<'_, 'b>, label: &str) -> Ui<'r, 'b>
where
    's: 'r,
{
    ui.append(Widget::Button(label.to_owned()))
}

// syntax sugar
fn label<'b, 's, 'r>(ui: &'s mut Ui<'_, 'b>, label: &str) -> Ui<'r, 'b>
where
    's: 'r,
{
    ui.append(Widget::Label(label.to_owned()))
}
*/

#[derive(Debug)]
struct ConfigParseError {
    more: String,
}

impl std::fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Config parse error: {}", self.more)
    }
}

impl Error for ConfigParseError {}

fn do_parse_ui_stuff() -> UiResult<Vec<ast::Item>> {
    let mut f = File::open("hello.grui")?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)?;

    // Remove comments
    let ws_re = Regex::new(r"//[^\n]*").unwrap();
    let contents = ws_re.replace_all(&contents, "");

    match grammar::MainParser::new().parse(&contents) {
        //dbg!(&ast);
        Ok(ast) => Ok(ast),
        Err(e) => Err(Box::new(ConfigParseError {
            more: format!("{}", e),
        })),
    }
}

fn emit_gui_item(ui: &mut Ui, item: &ast::Item) {
    let item_id = ui.node.next_child_id;

    let ctx = match item.ident.as_str() {
        "label" => {
            if let ast::Value::String(ref value) = item.value {
                Some(UiNode::new(Widget::Label(value.to_owned())))
            } else {
                None
            }
        }
        "horizontal" => {
            if let ast::Value::List(ref items) = item.value {
                let mut sub_ctx = UiNode::new(Widget::Horizontal);
                emit_gui_items(
                    &mut Ui::new(&mut sub_ctx, ui.context.nested(item_id)),
                    items,
                );
                Some(sub_ctx)
            } else {
                None
            }
        }
        "vertical" => {
            if let ast::Value::List(ref items) = item.value {
                let mut sub_ctx = UiNode::new(Widget::Vertical);
                emit_gui_items(
                    &mut Ui::new(&mut sub_ctx, ui.context.nested(item_id)),
                    items,
                );
                Some(sub_ctx)
            } else {
                None
            }
        }
        "button" => {
            if let ast::Value::String(ref value) = item.value {
                Some(UiNode::new(Widget::Button(value.to_owned())))
            } else {
                None
            }
        }
        _ => {
            unimplemented!();
        }
    };

    if let Some(mut ctx) = ctx {
        ctx.string_uid = item.uid.clone();
        ui.append(ctx);
    }
}

fn emit_gui_items(ui: &mut Ui, ast: &[ast::Item]) {
    for item in ast {
        emit_gui_item(ui, item);
    }
}

#[derive(Debug)]
struct LayoutTree {
    extent: Vec2,
    offset: Vec2,
    children: Vec<LayoutTree>,
}

impl LayoutTree {
    fn rect(w: f32, h: f32) -> Self {
        Self {
            extent: vec2(w, h),
            offset: vec2(0.0, 0.0),
            children: Default::default(),
        }
    }
}

fn calculate_ui_layout(ctx: &UiNode) -> LayoutTree {
    match &ctx.widget {
        Widget::Button(_s) => LayoutTree::rect(180.0, 25.0),
        Widget::Label(_s) => LayoutTree::rect(180.0, 25.0),
        Widget::Horizontal => {
            let mut node = LayoutTree::rect(0.0, 0.0);
            let mut x = 0f32;
            let mut y = 0f32;
            for item in &ctx.children {
                let mut ch = calculate_ui_layout(&item.1);
                ch.offset = vec2(x, 0.0);
                x += ch.extent.x();
                y = y.max(ch.extent.y());
                node.extent = vec2(x, y);
                node.children.push(ch);
            }
            node
        }
        Widget::Vertical => {
            let mut node = LayoutTree::rect(0.0, 0.0);
            let mut x = 0f32;
            let mut y = 0f32;
            for item in &ctx.children {
                let mut ch = calculate_ui_layout(&item.1);
                ch.offset = vec2(0.0, y);
                y += ch.extent.y();
                x = x.max(ch.extent.x());
                node.extent = vec2(x, y);
                node.children.push(ch);
            }
            node
        }
    }
}

fn flatten_widgets_inner<'a>(ui: &'a UiNode, uid: &WidgetUid) -> Vec<(WidgetUid, &'a Widget)> {
    let mut result = Vec::new();

    result.push((uid.clone(), &ui.widget));

    match &ui.widget {
        Widget::Horizontal | Widget::Vertical => {
            for item in &ui.children {
                let mut uid = uid.clone();
                uid.0.push(item.0);

                result.append(&mut flatten_widgets_inner(&item.1, &uid));
            }
        }
        _ => (),
    }
    result
}

fn flatten_widgets<'a>(ui: &'a UiNode) -> Vec<(WidgetUid, &'a Widget)> {
    flatten_widgets_inner(ui, &WidgetUid(Vec::new()))
}

#[derive(Debug)]
struct FlattenedLayout {
    offset: Vec2,
    extent: Vec2,
}

fn flatten_layout<'a>(base_offset: Vec2, node: &'a LayoutTree) -> Vec<FlattenedLayout> {
    let mut result = Vec::new();
    let offset = base_offset + node.offset;

    result.push(FlattenedLayout {
        offset,
        extent: node.extent,
    });

    for item in &node.children {
        result.append(&mut flatten_layout(offset, &item));
    }

    result
}

#[derive(Default, Debug)]
struct UiInteractionState {
    hover_widget: Option<WidgetUid>,
    drag_begin_widget: Option<WidgetUid>,
    mouse_down: bool,
    mouse_released: bool,
    mouse_pressed: bool,
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("gui proto")
        .with_dimensions(INIT_WINDOW_SIZE.0, INIT_WINDOW_SIZE.1);
    let context = glutin::ContextBuilder::new()
        .with_vsync(false)
        .with_multisampling(4)
        .with_srgb(true);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);
    }

    let context = nanovg::ContextBuilder::new()
        .stencil_strokes()
        .build()
        .expect("Initialization of NanoVG failed!");

    let fonts = DemoFonts {
        sans: Font::from_file(&context, "Roboto-Regular", "resources/Roboto-Regular.ttf")
            .expect("Failed to load font 'Roboto-Regular.ttf'"),
    };

    let mut running = true;
    let mut mouse = vec2(0.0f32, 0.0f32);

    let mut interaction_state = UiInteractionState::default();

    loop {
        let prev_mouse_down = interaction_state.mouse_down;

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => running = false,
                glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    mouse = vec2(position.0 as f32, position.1 as f32)
                }
                glutin::WindowEvent::MouseInput { state, .. } => {
                    interaction_state.mouse_down = state == glutin::ElementState::Pressed;
                }
                _ => {}
            },
            _ => {}
        });

        if !running {
            break;
        }

        let (width, height) = gl_window.get_inner_size().unwrap();
        let (width, height) = (width as i32, height as i32);

        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.3, 0.3, 0.32, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        let (width, height) = (width as f32, height as f32);
        context.frame((width, height), gl_window.hidpi_factor(), |frame| {
            let mut ui_top_level = UiNode::new(Widget::Vertical);
            let mut ui_ctx = Ui::new(
                &mut ui_top_level,
                UiContext {
                    uid: WidgetUid(Vec::new()),
                    interaction_state: &interaction_state,
                },
            );

            //dbg!(&interaction_state);

            let gui_ast = do_parse_ui_stuff();

            if let Err(ref err) = gui_ast {
                println!("{}", err);
            }

            // TODO: cache
            if let Ok(gui_ast) = do_parse_ui_stuff() {
                emit_gui_items(&mut ui_ctx, &gui_ast);

                //dbg!(&ui_ctx);
                let _ = do_ui_stuff(&mut ui_ctx);

                let ui_layout = calculate_ui_layout(&ui_ctx.node);

                //dbg!(&ui_layout);

                let flat_widgets = flatten_widgets(&ui_ctx.node);
                let flat_layout = flatten_layout(vec2(0.0, 0.0), &ui_layout);

                interaction_state.hover_widget = None;
                interaction_state.mouse_released = !interaction_state.mouse_down && prev_mouse_down;
                interaction_state.mouse_pressed = interaction_state.mouse_down && !prev_mouse_down;

                if !interaction_state.mouse_down && !prev_mouse_down {
                    interaction_state.drag_begin_widget = None;
                }

                for ((widget_uid, widget), layout) in flat_widgets.iter().zip(&flat_layout) {
                    let mouse_in_bounds = mouse.cmpge(layout.offset).all()
                        && mouse.cmplt(layout.offset + layout.extent).all();

                    match widget {
                        Widget::Button(_s) => {
                            if mouse_in_bounds {
                                interaction_state.hover_widget = Some(widget_uid.to_owned());

                                if interaction_state.mouse_pressed {
                                    interaction_state.drag_begin_widget =
                                        Some(widget_uid.to_owned());
                                }
                            }
                        }
                        _ => (),
                    }
                }

                for ((widget_uid, widget), layout) in flat_widgets.iter().zip(&flat_layout) {
                    match widget {
                        Widget::Label(s) => draw_label(
                            &frame,
                            &fonts,
                            s,
                            layout.offset.x(),
                            layout.offset.y(),
                            layout.extent.x(),
                            20.0,
                        ),
                        Widget::Button(s) => {
                            let color =
                                if interaction_state.hover_widget.as_ref() == Some(widget_uid) {
                                    Color::from_rgba(16, 112, 144, 255)
                                } else {
                                    Color::from_rgba(0, 96, 128, 255)
                                };

                            draw_button(
                                &frame,
                                &fonts,
                                s,
                                layout.offset.x(),
                                layout.offset.y(),
                                layout.extent.x(),
                                28.0,
                                color,
                                interaction_state.drag_begin_widget.as_ref() == Some(widget_uid),
                            )
                        }
                        _ => (),
                    }
                }
            }
        });

        gl_window.swap_buffers().unwrap();
    }
}

struct DemoFonts<'a> {
    sans: Font<'a>,
}

fn draw_label(frame: &Frame, fonts: &DemoFonts, text: &str, x: f32, y: f32, _w: f32, h: f32) {
    frame.text(
        fonts.sans,
        (x, y + h * 0.5),
        text,
        TextOptions {
            size: 18.0,
            color: Color::from_rgba(255, 255, 255, 128),
            align: Alignment::new().left().middle(),
            ..Default::default()
        },
    );
}

fn is_black(color: Color) -> bool {
    color.red() == 0.0 && color.green() == 0.0 && color.blue() == 0.0 && color.alpha() == 0.0
}

fn draw_button(
    frame: &Frame,
    fonts: &DemoFonts,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    pressed: bool,
) {
    let corner_radius = 4.0;
    let color_is_black = is_black(color);

    // button background
    frame.path(
        |path| {
            path.rounded_rect((x + 1.0, y + 1.0), (w - 2.0, h - 2.0), corner_radius - 0.5);
            if !color_is_black {
                path.fill(color, Default::default());
            }

            let c0 = Color::from_rgba(255, 255, 255, if color_is_black { 16 } else { 32 });
            let c1 = Color::from_rgba(0, 0, 0, if color_is_black { 16 } else { 32 });

            path.fill(
                Gradient::Linear {
                    start: (x, y),
                    end: (x, y + h),
                    start_color: if !pressed { c0 } else { c1 },
                    end_color: if !pressed { c1 } else { c0 },
                },
                Default::default(),
            );
        },
        Default::default(),
    );

    // button border
    frame.path(
        |path| {
            path.rounded_rect((x + 0.5, y + 0.5), (w - 1.0, h - 1.0), corner_radius - 0.5);
            path.stroke(Color::from_rgba(0, 0, 0, 48), Default::default());
        },
        Default::default(),
    );

    let (tw, _) = frame.text_bounds(
        fonts.sans,
        (0.0, 0.0),
        text,
        TextOptions {
            size: 20.0,
            ..Default::default()
        },
    );

    let mut options = TextOptions {
        size: 20.0,
        align: Alignment::new().left().middle(),
        ..Default::default()
    };

    options.color = Color::from_rgba(0, 0, 0, 160);

    frame.text(
        fonts.sans,
        (x + w * 0.5 - tw * 0.5, y + h * 0.5 - 1.0),
        text,
        options,
    );

    options.color = Color::from_rgba(255, 255, 255, 160);

    frame.text(
        fonts.sans,
        (x + w * 0.5 - tw * 0.5, y + h * 0.5),
        text,
        options,
    );
}

// ----

fn do_ui_stuff(ui: &mut Ui) -> UiResult<()> {
    if ui.button("I'm from code").clicked() {
        println!("code button clicked!");
    }

    if ui.id("special_button")?.clicked() {
        println!("special button clicked!");
    }

    let mut append_box = ui.id("append_box")?;
    append_box.label("label 1");
    append_box.label("label 2");
    append_box.label("label 3");

    ui.label(format!("Hover: {:?}", ui.context.interaction_state.hover_widget).as_str());

    Ok(())
}
