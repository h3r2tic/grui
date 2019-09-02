#![allow(unused_imports)]

#[macro_use]
extern crate lalrpop_util;
lalrpop_mod!(pub grammar); // synthesized by LALRPOP

mod ast;

use regex::Regex;
use std::fs::File;
use std::io::prelude::*;

use glutin::GlContext;
use nanovg::{Direction, Alignment, Color, Font, Frame, Gradient, ImagePattern,
             LineCap, LineJoin, PathOptions, Scissor, Solidity, StrokeOptions,
             TextOptions, Transform, Winding, Image, Context, Clip, Intersect};
use std::f32::consts::PI;
use std::time::Instant;

const INIT_WINDOW_SIZE: (u32, u32) = (300, 300);

trait Widget {}

struct Button;

impl Button {
    fn new() -> Self {
        unimplemented!();
    }

    fn with_label(self, _label: &str) -> Self {
        self
    }
}

impl Widget for Button {}

struct UiContext;

impl UiContext {
    fn clicked(&self) -> bool {
        unimplemented!();
    }

    fn id(&self, _label: &str) -> UiContext {
        unimplemented!();
    }

    fn append(&self, _widget: impl Widget) {
        unimplemented!();
    }
}

#[allow(dead_code)]
fn do_ui_stuff(ui: UiContext) {
    ui.append(Button::new().with_label("Why helo thar"));

    if ui.id("herpderp").clicked() {
        ui.id("herpderp")
            .append(Button::new().with_label("Inline button lulz"));
    }
}

fn do_parse_ui_stuff() -> Vec<Box<ast::Decl>> {
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

fn main() {
    do_parse_ui_stuff();

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
            let mut x = 50.0;
            let mut y = 50.0;

            draw_label(&frame, &fonts, "Login", x, y, 280.0, 20.0);
            y += 25.0;
            draw_label(&frame, &fonts, "Password", x, y, 280.0, 20.0);
            x += 80.0;
            draw_button(&frame, &fonts, "Sign in", x, y, 140.0, 28.0, Color::from_rgba(0, 96, 128, 255));
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

fn draw_button(frame: &Frame, fonts: &DemoFonts, text: &str, x: f32, y: f32, w: f32, h: f32, color: Color) {
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
                    start_color: Color::from_rgba(255, 255, 255, if color_is_black { 16 } else { 32 }),
                    end_color: Color::from_rgba(0, 0, 0, if color_is_black { 16 } else { 32 }),
                },
                Default::default()
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

    let mut iw = 0.0;

    let mut options = TextOptions {
        size: 20.0,
        align: Alignment::new().left().middle(),
        ..Default::default()
    };

    options.color = Color::from_rgba(0, 0, 0, 160);

    frame.text(
        fonts.sans,
        (x + w * 0.5 - tw * 0.5 + iw * 0.25, y + h * 0.5 - 1.0),
        text,
        options,
    );

    options.color = Color::from_rgba(255, 255, 255, 160);

    frame.text(
        fonts.sans,
        (x + w * 0.5 - tw * 0.5 + iw * 0.25, y + h * 0.5),
        text,
        options,
    );
}