// #![cfg(not(target_arch = "wasm32"))]

// A demonstration using winit to provide events and glow for drawing the Ui.

use conrod_floatwin_demo_glow::{conrod_glow, set_widgets, Ids, UiState, WinIds};

use conrod_floatwin::WindowingState;
use conrod_glow::Renderer;
use glow::HasContext;
use glutin::{event, event_loop::ControlFlow};

#[allow(dead_code)]
mod conversion_fns {
    // Conversion functions for converting between types from `winit` and `conrod_core`.
    conrod_floatwin_demo_glow::v023_conversion_fns!();
}
use conversion_fns::*;

const WIN_W: u32 = 800;
const WIN_H: u32 = 600;

fn main() {
    // Build the window.
    let event_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new()
        .with_title("Conrod with glow!")
        .with_inner_size(glutin::dpi::LogicalSize::new(WIN_W, WIN_H));
    let context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_multisampling(4)
        .build_windowed(window, &event_loop)
        .unwrap();
    let context = unsafe { context.make_current() }.unwrap();
    let gl = glow::Context::from_loader_function(|s| context.get_proc_address(s) as *const _);

    let mut current_hidpi_factor = context.window().scale_factor();

    // Construct our `Ui`.
    let mut ui = conrod_core::UiBuilder::new([WIN_W as f64, WIN_H as f64])
        .theme(conrod_example_shared::theme())
        .build();

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    let font_collection = conrod_core::text::FontCollection::from_bytes(include_bytes!(
        "../../assets/fonts/NotoSans/NotoSans-Regular.ttf"
    ) as &[u8])
    .unwrap();
    for font in font_collection.into_fonts() {
        ui.fonts.insert(font.unwrap());
    }

    // Load the Rust logo from our assets folder to use as an example image.
    fn load_rust_logo(gl: &glow::Context) -> conrod_glow::Texture {
        let rgba_image = image::load_from_memory_with_format(
            include_bytes!("../../assets/images/rust.png"),
            image::ImageFormat::PNG,
        )
        .unwrap()
        .to_rgba();
        let image_dimensions = rgba_image.dimensions();

        let pixels: Vec<_> = rgba_image
            .into_raw()
            .chunks(image_dimensions.0 as usize * 4)
            .rev()
            .flat_map(|row| row.iter())
            .map(|p| p.clone())
            .collect();

        let texture;
        unsafe {
            texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                image_dimensions.0 as i32,
                image_dimensions.1 as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&pixels),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
        }

        conrod_glow::Texture {
            texture,
            width: image_dimensions.0,
            height: image_dimensions.1,
        }
    }

    let mut image_map = conrod_core::image::Map::new();
    let rust_logo = image_map.insert(load_rust_logo(&gl));

    // A type used for converting `conrod_core::render::Primitives` into `Command`s that can be used
    // for drawing to the glium `Surface`.
    //
    // Internally, the `Renderer` maintains:
    // - a `backend::glium::GlyphCache` for caching text onto a `glium::texture::Texture2d`.
    // - a `glium::Program` to use as the shader program when drawing to the `glium::Surface`.
    // - a `Vec` for collecting `backend::glium::Vertex`s generated when translating the
    // `conrod_core::render::Primitive`s.
    // - a `Vec` of commands that describe how to draw the vertices.
    let mut renderer = Renderer::new(&gl, true).unwrap();

    let mut ids = Ids::new(ui.widget_id_generator());

    let mut win_state = WindowingState::new();
    let win_ids = WinIds {
        conrod_example: win_state.next_id(),
    };

    let mut ui_state = UiState {
        enable_debug: false,
        win_state,
        win_ids,
        conrod_example_app: conrod_example_shared::DemoApp::new(rust_logo),
    };

    macro_rules! verify {
        () => {{
            let err = gl.get_error();
            if err != 0 {
                panic!("gl error {}", err);
            }
        }};
    }

    unsafe {
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        verify!();
        gl.enable(glow::BLEND);
        verify!();
        gl.blend_func_separate(
            glow::SRC_ALPHA,
            glow::ONE_MINUS_SRC_ALPHA,
            glow::ONE,
            glow::ONE_MINUS_SRC_ALPHA,
        );
        verify!();
    }

    let sixteen_ms = std::time::Duration::from_millis(16);
    let mut next_update = None;
    let mut ui_update_needed = false;
    event_loop.run(move |event, _, control_flow| {
        // *control_flow = glutin::event_loop::ControlFlow::Wait;

        match &event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                // Break from the loop upon `Escape`.
                glutin::event::WindowEvent::CloseRequested
                | glutin::event::WindowEvent::KeyboardInput {
                    input:
                        glutin::event::KeyboardInput {
                            virtual_keycode: Some(glutin::event::VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
                glutin::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    current_hidpi_factor = *scale_factor;
                }
                // Toggle fullscreen on `F11`.
                winit::event::WindowEvent::KeyboardInput {
                    input:
                        winit::event::KeyboardInput {
                            virtual_keycode: Some(winit::event::VirtualKeyCode::F11),
                            state: winit::event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => match context.window().fullscreen() {
                    Some(_) => context.window().set_fullscreen(None),
                    None => context.window().set_fullscreen(Some(
                        winit::window::Fullscreen::Borderless(context.window().current_monitor()),
                    )),
                },
                glutin::event::WindowEvent::Resized(size) => {
                    context.resize(*size);
                }
                // // mouse grab
                // winit::event::WindowEvent::MouseInput {
                //     button: winit::event::MouseButton::Left,
                //     state: winit::event::ElementState::Pressed,
                //     ..
                // } => {
                //     eprintln!("{:?}", context.window().set_cursor_grab(true));
                // }
                // winit::event::WindowEvent::MouseInput {
                //     button: winit::event::MouseButton::Left,
                //     state: winit::event::ElementState::Released,
                //     ..
                // } => {
                //     eprintln!("{:?}", context.window().set_cursor_grab(false));
                // }
                // winit::event::WindowEvent::MouseWheel { delta, .. } => {
                //     eprintln!("{:?}", delta);
                // }
                _ => {}
            },
            glutin::event::Event::RedrawRequested(_) => {
                // This is needed because `v022_conversion_fns` does not convert it
                // to a `Redraw` event.
                ui.needs_redraw();
                ui_update_needed = true;
            }
            _ => {}
        }

        // Use the `winit` backend feature to convert the winit event to a conrod one.
        if let Some(event) = convert_event(&event, &context.window()) {
            ui.handle_event(event);
            ui_update_needed = true;
        }

        // We don't want to draw any faster than 60 FPS, so set the UI only on every 16ms, unless:
        // - this is the very first event, or
        // - we didn't request update on the last event and new events have arrived since then.
        let should_set_ui_on_main_events_cleared = next_update.is_none() && ui_update_needed;
        match (&event, should_set_ui_on_main_events_cleared) {
            (event::Event::NewEvents(event::StartCause::Init { .. }), _)
            | (event::Event::NewEvents(event::StartCause::ResumeTimeReached { .. }), _)
            | (event::Event::MainEventsCleared, true) => {
                next_update = Some(std::time::Instant::now() + sixteen_ms);
                ui_update_needed = false;

                // Instantiate a GUI demonstrating every widget type provided by conrod.
                // conrod_example_shared::gui(&mut ui.set_widgets(), &ids, &mut app);
                set_widgets(
                    ui.set_widgets(),
                    &mut ids,
                    current_hidpi_factor,
                    &mut ui_state,
                );

                // Get the underlying winit window and update the mouse cursor as set by conrod.
                context
                    .window()
                    .set_cursor_icon(convert_mouse_cursor(ui.mouse_cursor()));

                // Draw the `Ui` if it has changed.
                if let Some(primitives) = ui.draw_if_changed() {
                    renderer.fill(&context, &gl, primitives, &image_map);
                    unsafe {
                        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                        gl.enable(glow::FRAMEBUFFER_SRGB);
                        gl.viewport(
                            0,
                            0,
                            context.window().inner_size().width as i32,
                            context.window().inner_size().height as i32,
                        );
                    }
                    renderer.draw(&gl, &image_map).unwrap();
                    context.swap_buffers().unwrap();
                } else {
                    // We don't need to update the UI anymore until more events arrives.
                    next_update = None;
                }
            }
            _ => {}
        }
        if let Some(next_update) = next_update {
            *control_flow = ControlFlow::WaitUntil(next_update);
        } else {
            *control_flow = ControlFlow::Wait;
        }
    })
}
