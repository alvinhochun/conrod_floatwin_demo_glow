use crate::{conrod_glow, set_widgets, Ids, UiState, WinIds};

use conrod_floatwin::WindowingState;
use conrod_glow::Renderer;
use glow::HasContext;
use wasm_bindgen::{prelude::*, JsCast};
use winit::platform::web::WindowBuilderExtWebSys;

#[allow(dead_code)]
mod conversion_fns {
    // Conversion functions for converting between types from `winit` and `conrod_core`.
    crate::v023_conversion_fns!();
}
use conversion_fns::*;

const WIN_W: u32 = 800;
const WIN_H: u32 = 600;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    let webgl2_context = canvas
        .get_context("webgl2")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::WebGl2RenderingContext>()
        .unwrap();
    let gl = glow::Context::from_webgl2_context(webgl2_context);

    let event_loop = winit::event_loop::EventLoop::new();
    let winit_window = winit::window::WindowBuilder::new()
        .with_title("Conrod with glow!")
        // .with_inner_size(winit::dpi::LogicalSize::new(WIN_W, WIN_H))
        .with_auto_parent_size()
        .with_canvas(Some(canvas));
    let winit_window = winit_window.build(&event_loop).unwrap();

    // let mut current_hidpi_factor = window.device_pixel_ratio();
    let mut current_hidpi_factor = winit_window.scale_factor();

    // Construct our `Ui`.
    let mut ui = conrod_core::UiBuilder::new([WIN_W as f64, WIN_H as f64])
        .theme(conrod_example_shared::theme())
        .build();

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    let font_collection = conrod_core::text::FontCollection::from_bytes(include_bytes!(
        "../assets/fonts/NotoSans/NotoSans-Regular.ttf"
    ) as &[u8])
    .unwrap();
    for font in font_collection.into_fonts() {
        ui.fonts.insert(font.unwrap());
    }

    // Load the Rust logo from our assets folder to use as an example image.
    fn load_rust_logo(gl: &glow::Context) -> conrod_glow::Texture {
        let rgba_image = image::load_from_memory_with_format(
            include_bytes!("../assets/images/rust.png"),
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
    let mut renderer = Renderer::new(&gl, false).unwrap();

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

    let mut should_update_ui = true;
    let mut needs_next_update = true;
    event_loop.run(move |event, _, control_flow| {
        // Break from the loop upon `Escape` or closed window.
        match &event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                // Break from the loop upon `Escape`.
                winit::event::WindowEvent::CloseRequested
                | winit::event::WindowEvent::KeyboardInput {
                    input:
                        winit::event::KeyboardInput {
                            virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                    return;
                }
                winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
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
                } => match winit_window.fullscreen() {
                    Some(_) => winit_window.set_fullscreen(None),
                    None => winit_window.set_fullscreen(Some(
                        winit::window::Fullscreen::Borderless(winit_window.current_monitor()),
                    )),
                },
                _ => {}
            },
            winit::event::Event::RedrawRequested(_) => {
                // This is needed because `v022_conversion_fns` does not convert it
                // to a `Redraw` event.
                web_sys::console::log_1(&JsValue::from_str("needs redraw"));
                ui.needs_redraw();
                should_update_ui = true;
            }
            _ => {}
        }

        // Use the `winit` backend feature to convert the winit event to a conrod one.
        if let Some(event) = convert_event(&event, &winit_window) {
            ui.handle_event(event);
            should_update_ui = true;
        }

        match &event {
            winit::event::Event::MainEventsCleared => {
                if should_update_ui || needs_next_update {
                    needs_next_update = true;
                    should_update_ui = false;

                    // Instantiate a GUI demonstrating every widget type provided by conrod.
                    // conrod_example_shared::gui(&mut ui.set_widgets(), &ids, &mut app);
                    set_widgets(
                        ui.set_widgets(),
                        &mut ids,
                        current_hidpi_factor,
                        &mut ui_state,
                    );

                    // Get the underlying winit window and update the mouse cursor as set by conrod.
                    winit_window.set_cursor_icon(convert_mouse_cursor(ui.mouse_cursor()));

                    macro_rules! verify {
                        () => {{
                            let err = gl.get_error();
                            if err != 0 {
                                panic!("gl error {}", err);
                            }
                        }};
                    }

                    // Draw the `Ui` if it has changed.
                    if let Some(primitives) = ui.draw_if_changed() {
                        let display = (
                            winit_window.inner_size().width,
                            winit_window.inner_size().height,
                            winit_window.scale_factor(),
                        );
                        renderer.fill(&display, &gl, primitives, &image_map);
                        unsafe {
                            gl.clear_color(0.0, 0.0, 0.0, 1.0);
                            verify!();
                            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
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
                            gl.viewport(
                                0,
                                0,
                                winit_window.inner_size().width as i32,
                                winit_window.inner_size().height as i32,
                            );
                            verify!();
                        }
                        renderer.draw(&gl, &image_map).unwrap();
                    } else {
                        needs_next_update = false;
                    }
                }
            }
            _ => {}
        }
        if needs_next_update {
            // On WASM, ControlFlow::Poll uses `requestAnimationFrame`, so this
            // is completely fine.
            *control_flow = winit::event_loop::ControlFlow::Poll;
        } else {
            *control_flow = winit::event_loop::ControlFlow::Wait;
        }
    })
}
