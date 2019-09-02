#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[macro_use]
extern crate lalrpop_util;
lalrpop_mod!(pub grammar); // synthesized by LALRPOP

mod ast;

use regex::Regex;
use std::fs::File;
use std::io::prelude::*;

use glutin::GlContext;
use nanovg::{
    Alignment, Clip, Color, Context, Direction, Font, Frame, Gradient, Image, ImagePattern,
    Intersect, LineCap, LineJoin, PathOptions, Scissor, Solidity, StrokeOptions, TextOptions,
    Transform, Winding,
};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::time::Instant;

use glam::{vec2, Vec2};

const INIT_WINDOW_SIZE: (u32, u32) = (300, 300);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct WidgetId(usize);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct WidgetUid(Vec<WidgetId>);

#[derive(Debug)]
enum Widget {
    Button(String),
    Label(String),
    Horizontal(Vec<(WidgetId, Widget)>),
    Vertical(Vec<(WidgetId, Widget)>),
}

#[derive(Debug)]
struct UiContext {
    appended_items: Vec<(WidgetId, Widget)>,
    next_widget_id: WidgetId,
}

impl UiContext {
    fn new() -> Self {
        Self {
            appended_items: Vec::new(),
            next_widget_id: WidgetId(0),
        }
    }

    fn clicked(&self) -> bool {
        unimplemented!();
    }

    fn id(&self, _label: &str) -> UiContext {
        unimplemented!();
    }

    fn append(&mut self, widget: Widget) {
        let id = self.next_widget_id;
        self.next_widget_id.0 += 1;
        self.appended_items.push((id, widget))
    }
}

#[allow(dead_code)]
fn do_ui_stuff(ui: &mut UiContext) {
    ui.append(Widget::Button("I'm from code".to_owned()));

    if ui.id("myspecialbutton").clicked() {
        // do stuff
    }
}

fn do_parse_ui_stuff() -> Vec<ast::Item> {
    let mut f = File::open("hello.grui").expect("file not found");

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    // Remove comments
    let ws_re = Regex::new(r"//[^\n]*").unwrap();
    let contents = ws_re.replace_all(&contents, "");

    let ast = grammar::MainParser::new().parse(&contents).unwrap();

    dbg!(&ast);

    ast
}

fn emit_gui_item(ui: &mut UiContext, item: &ast::Item) {
    match item.ident.as_str() {
        "label" => {
            if let ast::Value::String(ref value) = item.value {
                ui.append(Widget::Label(value.to_owned()));
            }
        }
        "horizontal" => {
            if let ast::Value::List(ref items) = item.value {
                let mut sub_ctx = UiContext::new();
                emit_gui_items(&mut sub_ctx, items);
                ui.append(Widget::Horizontal(sub_ctx.appended_items));
            }
        }
        "vertical" => {
            if let ast::Value::List(ref items) = item.value {
                let mut sub_ctx = UiContext::new();
                emit_gui_items(&mut sub_ctx, items);
                ui.append(Widget::Vertical(sub_ctx.appended_items));
            }
        }
        "button" => {
            if let ast::Value::String(ref value) = item.value {
                ui.append(Widget::Button(value.to_owned()));
            }
        }
        _ => {
            unimplemented!();
        }
    }
}

fn emit_gui_items(ui: &mut UiContext, ast: &[ast::Item]) {
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

fn calculate_vertical_layout(items: &[(WidgetId, Widget)]) -> LayoutTree {
    let mut node = LayoutTree::rect(0.0, 0.0);
    let mut x = 0f32;
    let mut y = 0f32;
    for item in items {
        let mut ch = calculate_widget_layout(&item.1);
        ch.offset = vec2(0.0, y);
        y += ch.extent.y();
        x = x.max(ch.extent.x());
        node.extent = vec2(x, y);
        node.children.push(ch);
    }
    node
}

fn calculate_widget_layout(widget: &Widget) -> LayoutTree {
    match widget {
        Widget::Button(s) => LayoutTree::rect(180.0, 25.0),
        Widget::Label(s) => LayoutTree::rect(180.0, 25.0),
        Widget::Horizontal(ref items) => {
            let mut node = LayoutTree::rect(0.0, 0.0);
            let mut x = 0f32;
            let mut y = 0f32;
            for item in items {
                let mut ch = calculate_widget_layout(&item.1);
                ch.offset = vec2(x, 0.0);
                x += ch.extent.x();
                y = y.max(ch.extent.y());
                node.extent = vec2(x, y);
                node.children.push(ch);
            }
            node
        }
        Widget::Vertical(ref items) => calculate_vertical_layout(items),
    }
}

fn calculate_ui_layout(ui: &UiContext) -> LayoutTree {
    calculate_vertical_layout(&ui.appended_items)
}

fn flatten_widget<'a>(widget: &'a Widget) -> Vec<&'a Widget> {
    let mut result = Vec::new();

    result.push(widget);

    match widget {
        Widget::Horizontal(ref items) | Widget::Vertical(ref items) => {
            for item in items {
                result.append(&mut flatten_widget(&item.1));
            }
        }
        _ => (),
    }
    result
}

fn flatten_widgets<'a>(ui: &'a UiContext) -> Vec<&'a Widget> {
    let mut result = Vec::new();
    for item in &ui.appended_items {
        result.append(&mut flatten_widget(&item.1));
    }
    result
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

fn main() {
    let gui_ast = do_parse_ui_stuff();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("NanoVG Text")
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
    let mut mouse = (0.0f32, 0.0f32);

    loop {
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => running = false,
                glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    mouse = (position.0 as f32, position.1 as f32)
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
            let mut ui_ctx = UiContext::new();

            emit_gui_items(&mut ui_ctx, &gui_ast);
            do_ui_stuff(&mut ui_ctx);

            //dbg!(&ui_ctx);

            let ui_layout = calculate_ui_layout(&ui_ctx);

            //dbg!(&ui_layout);

            let flat_widgets = flatten_widgets(&ui_ctx);
            let flat_layout = flatten_layout(vec2(0.0, 0.0), &ui_layout);
            let flat_layout = flat_layout.iter().skip(1); // skip the root vertical layout node

            for (widget, layout) in flat_widgets.iter().zip(flat_layout) {
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
                    Widget::Button(s) => draw_button(
                        &frame,
                        &fonts,
                        s,
                        layout.offset.x(),
                        layout.offset.y(),
                        layout.extent.x(),
                        28.0,
                        Color::from_rgba(0, 96, 128, 255),
                    ),
                    _ => (),
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

            path.fill(
                Gradient::Linear {
                    start: (x, y),
                    end: (x, y + h),
                    start_color: Color::from_rgba(
                        255,
                        255,
                        255,
                        if color_is_black { 16 } else { 32 },
                    ),
                    end_color: Color::from_rgba(0, 0, 0, if color_is_black { 16 } else { 32 }),
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
