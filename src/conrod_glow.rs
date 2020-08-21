// A glow backend for rendering conrod primitives.

use conrod_core::{color, image, render, text, Rect, Scalar};
use glow::HasContext;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GlRect {
    left: u32,
    bottom: u32,
    width: u32,
    height: u32,
}

/// A `Command` describing a step in the drawing process.
#[derive(Clone, Debug)]
pub enum Command<'a> {
    /// Draw to the target.
    Draw(Draw<'a>),
    /// Update the scissor rect.
    Scizzor(GlRect),
}

/// A `Command` for drawing to the target.
///
/// Each variant describes how to draw the contents of the vertex buffer.
#[derive(Clone, Debug)]
pub enum Draw<'a> {
    /// A range of vertices representing triangles textured with the image in the
    /// image_map at the given `widget::Id`.
    Image(image::Id, &'a [Vertex]),
    /// A range of vertices representing plain triangles.
    Plain(&'a [Vertex]),
}

enum PreparedCommand {
    Image(image::Id, std::ops::Range<usize>),
    Plain(std::ops::Range<usize>),
    Scizzor(GlRect),
}

/// A rusttype `GlyphCache` along with a OpenGL texture handle for caching text on the `GPU`.
pub struct GlyphCache {
    cache: text::GlyphCache<'static>,
    texture: glow::Texture,
}

/// A type used for translating `render::Primitives` into `Command`s that indicate how to draw the
/// conrod GUI using `glow`.
pub struct Renderer {
    program: Program,
    vbo: glow::Buffer,
    vao: glow::VertexArray,
    glyph_cache: GlyphCache,
    commands: Vec<PreparedCommand>,
    vertices: Vec<Vertex>,
}

/// An iterator yielding `Command`s, produced by the `Renderer::commands` method.
pub struct Commands<'a> {
    commands: std::slice::Iter<'a, PreparedCommand>,
    vertices: &'a [Vertex],
}

pub struct Texture {
    pub texture: glow::Texture,
    pub width: u32,
    pub height: u32,
}

/// The `Vertex` type passed to the vertex shader.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vertex {
    /// The mode with which the `Vertex` will be drawn within the fragment shader.
    ///
    /// `0` for rendering text.
    /// `1` for rendering an image.
    /// `2` for rendering non-textured 2D geometry.
    ///
    /// If any other value is given, the fragment shader will not output any color.
    pub mode: u32,
    /// The position of the vertex within vector space.
    ///
    /// [-1.0, -1.0] is the leftmost, bottom position of the display.
    /// [1.0, 1.0] is the rightmost, top position of the display.
    pub position: [f32; 2],
    /// The coordinates of the texture used by this `Vertex`.
    ///
    /// [0.0, 0.0] is the leftmost, bottom position of the texture.
    /// [1.0, 1.0] is the rightmost, top position of the texture.
    pub tex_coords: [f32; 2],
    /// A color associated with the `Vertex`.
    ///
    /// The way that the color is used depends on the `mode`.
    pub color: [f32; 4],
}

/// Draw text from the text cache texture `tex` in the fragment shader.
pub const MODE_TEXT: u32 = 0;
/// Draw an image from the texture at `tex` in the fragment shader.
pub const MODE_IMAGE: u32 = 1;
/// Ignore `tex` and draw simple, colored 2D geometry.
pub const MODE_GEOMETRY: u32 = 2;

/// The vertex shader used for OpenGL.
pub const VERTEX_SHADER_120: &'static str = "
    #version 120

    attribute vec2 position;
    attribute vec2 tex_coords;
    attribute vec4 color;
    attribute float mode;

    varying vec2 v_tex_coords;
    varying vec4 v_color;
    varying float v_mode;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
        v_tex_coords = tex_coords;
        v_color = color;
        v_mode = mode;
    }
";

/// The fragment shader used for OpenGL.
pub const FRAGMENT_SHADER_120: &'static str = "
    #version 120
    uniform sampler2D tex;

    varying vec2 v_tex_coords;
    varying vec4 v_color;
    varying float v_mode;

    void main() {
        // Text
        if (v_mode == 0.0) {
            gl_FragColor = v_color * vec4(1.0, 1.0, 1.0, texture2D(tex, v_tex_coords).r);

        // Image
        } else if (v_mode == 1.0) {
            gl_FragColor = texture2D(tex, v_tex_coords);

        // 2D Geometry
        } else if (v_mode == 2.0) {
            gl_FragColor = v_color;
        }
    }
";

/// The vertex shader used for OpenGL.
pub const VERTEX_SHADER_140: &'static str = "
    #version 140

    in vec2 position;
    in vec2 tex_coords;
    in vec4 color;
    in uint mode;

    out vec2 v_tex_coords;
    out vec4 v_color;
    flat out uint v_mode;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
        v_tex_coords = tex_coords;
        v_color = color;
        v_mode = mode;
    }
";

/// The fragment shader used for OpenGL.
pub const FRAGMENT_SHADER_140: &'static str = "
    #version 140
    uniform sampler2D tex;

    in vec2 v_tex_coords;
    in vec4 v_color;
    flat in uint v_mode;

    out vec4 f_color;

    void main() {
        // Text
        if (v_mode == uint(0)) {
            f_color = v_color * vec4(1.0, 1.0, 1.0, texture(tex, v_tex_coords).r);

        // Image
        } else if (v_mode == uint(1)) {
            f_color = texture(tex, v_tex_coords);

        // 2D Geometry
        } else if (v_mode == uint(2)) {
            f_color = v_color;
        }
    }
";

/// The vertex shader used for OpenGL ES.
pub const VERTEX_SHADER_300_ES: &'static str = "\
    #version 300 es
    precision mediump float;

    in vec2 position;
    in vec2 tex_coords;
    in vec4 color;
    in uint mode;

    out vec2 v_tex_coords;
    out vec4 v_color;
    flat out uint v_mode;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
        v_tex_coords = tex_coords;
        v_color = color;
        v_mode = mode;
    }
";

/// The fragment shader used for OpenGL ES.
pub const FRAGMENT_SHADER_300_ES: &'static str = "\
    #version 300 es
    precision mediump float;
    uniform sampler2D tex;

    in vec2 v_tex_coords;
    in vec4 v_color;
    flat in uint v_mode;

    out vec4 f_color;

    void main() {
        // Text
        if (v_mode == uint(0)) {
            f_color = v_color * vec4(1.0, 1.0, 1.0, texture(tex, v_tex_coords).r);

        // Image
        } else if (v_mode == uint(1)) {
            f_color = texture(tex, v_tex_coords);

        // 2D Geometry
        } else if (v_mode == uint(2)) {
            f_color = v_color;
        }
    }
";

/// The fragment shader with sRGB gamma correction used for OpenGL ES.
pub const FRAGMENT_SHADER_300_ES_LINEAR_TO_SRGB: &'static str = "\
    #version 300 es
    precision mediump float;
    uniform sampler2D tex;

    in vec2 v_tex_coords;
    in vec4 v_color;
    flat in uint v_mode;

    out vec4 f_color;

    vec3 toSrgb(vec3 linearRgb) {
        // Doing it the proper way without branching:
        bvec3 cutoff = lessThan(linearRgb, vec3(0.0031308));
        vec3 higher = vec3(1.055) * pow(linearRgb, vec3(1.0 / 2.4)) - vec3(0.055);
        vec3 lower = linearRgb * vec3(12.92);
        return mix(higher, lower, cutoff);
    }

    vec3 toSrgb_(vec3 linearRgb) {
        // The simple yet inaccurate way:
        return pow(linearRgb, vec3(1.0 / 2.2));
    }

    void main() {
        // Text
        if (v_mode == uint(0)) {
            f_color.rgb = toSrgb(v_color.rgb);
            f_color.a = v_color.a * texture(tex, v_tex_coords).r;

        // Image
        } else if (v_mode == uint(1)) {
            f_color.rgb = toSrgb(texture(tex, v_tex_coords).rgb);
            f_color.a = texture(tex, v_tex_coords).a;

        // 2D Geometry
        } else if (v_mode == uint(2)) {
            f_color.rgb = toSrgb(v_color.rgb);
            f_color.a = v_color.a;
        }
    }
";

pub struct Program {
    program: glow::Program,
    attrib_position: u32,
    attrib_tex_coords: u32,
    attrib_color: u32,
    attrib_mode: u32,
}

/// Construct the OpenGL shader program that can be used to render `Vertex`es.
pub fn program(gl: &glow::Context, is_framebuffer_srgb: bool) -> Result<Program, String> {
    let (vs, fs) = if cfg!(target_arch = "wasm32") {
        if is_framebuffer_srgb {
            (VERTEX_SHADER_300_ES, FRAGMENT_SHADER_300_ES)
        } else {
            (VERTEX_SHADER_300_ES, FRAGMENT_SHADER_300_ES_LINEAR_TO_SRGB)
        }
    } else {
        assert_eq!(is_framebuffer_srgb, true);
        (VERTEX_SHADER_140, FRAGMENT_SHADER_140)
    };
    unsafe {
        let program = gl.create_program().expect("program creation failure");

        let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
        gl.shader_source(vertex_shader, vs);
        gl.compile_shader(vertex_shader);
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!("{}", gl.get_shader_info_log(vertex_shader));
        }
        gl.attach_shader(program, vertex_shader);

        let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
        gl.shader_source(fragment_shader, fs);
        gl.compile_shader(fragment_shader);
        if !gl.get_shader_compile_status(fragment_shader) {
            panic!("{}", gl.get_shader_info_log(fragment_shader));
        }
        gl.attach_shader(program, fragment_shader);

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!(gl.get_program_info_log(program));
        }

        gl.detach_shader(program, vertex_shader);
        gl.delete_shader(vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(fragment_shader);

        let attrib_position = gl.get_attrib_location(program, "position").unwrap();
        let attrib_tex_coords = gl.get_attrib_location(program, "tex_coords").unwrap();
        let attrib_color = gl.get_attrib_location(program, "color").unwrap();
        let attrib_mode = gl.get_attrib_location(program, "mode").unwrap();

        Ok(Program {
            program,
            attrib_position,
            attrib_tex_coords,
            attrib_color,
            attrib_mode,
        })
    }
}

/// Converts gamma (brightness) from sRGB to linear color space.
///
/// sRGB is the default color space for image editors, pictures, internet etc.
/// Linear gamma yields better results when doing math with colors.
pub fn gamma_srgb_to_linear(c: [f32; 4]) -> [f32; 4] {
    fn component(f: f32) -> f32 {
        // Taken from https://github.com/PistonDevelopers/graphics/src/color.rs#L42
        if f <= 0.04045 {
            f / 12.92
        } else {
            ((f + 0.055) / 1.055).powf(2.4)
        }
    }
    [component(c[0]), component(c[1]), component(c[2]), c[3]]
}

// Creating the rusttype glyph cache used within a `GlyphCache`.
fn rusttype_glyph_cache(w: u32, h: u32) -> text::GlyphCache<'static> {
    const SCALE_TOLERANCE: f32 = 0.1;
    const POSITION_TOLERANCE: f32 = 0.1;
    text::GlyphCache::builder()
        .dimensions(w, h)
        .scale_tolerance(SCALE_TOLERANCE)
        .position_tolerance(POSITION_TOLERANCE)
        .build()
}

// Create the texture used within a `GlyphCache` of the given size.
fn glyph_cache_texture(
    gl: &glow::Context,
    width: u32,
    height: u32,
) -> Result<<glow::Context as HasContext>::Texture, String> {
    unsafe {
        let texture = gl.create_texture().unwrap();

        let num_components = 1;
        let data_size = num_components as usize * width as usize * height as usize;
        let data = vec![128u8; data_size];

        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_BASE_LEVEL, 0);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
        gl.tex_storage_2d(glow::TEXTURE_2D, 1, glow::R8, width as i32, height as i32);
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            width as i32,
            height as i32,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(&data),
        );
        Ok(texture)
    }
}

impl GlyphCache {
    /// Construct a **GlyphCache** with the given texture dimensions.
    ///
    /// When calling `GlyphCache::new`, the `get_framebuffer_dimensions` method is used to produce
    /// the width and height. However, often creating a texture the size of the screen might not be
    /// large enough to cache the necessary text for an application. The following constant
    /// multiplier is used to ensure plenty of room in the cache.
    pub fn with_dimensions(gl: &glow::Context, width: u32, height: u32) -> Result<Self, String> {
        // First, the rusttype `Cache` which performs the logic for rendering and laying out glyphs
        // in the cache.
        let cache = rusttype_glyph_cache(width, height);

        // Now the texture to which glyphs will be rendered.
        let texture = glyph_cache_texture(gl, width, height)?;

        Ok(GlyphCache {
            cache: cache,
            texture: texture,
        })
    }

    /// Construct a `GlyphCache` with a size equal to the given `Display`'s current framebuffer
    /// dimensions.
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        Self::with_dimensions(gl, 1200, 900)
    }

    /// The texture used to cache the glyphs on the GPU.
    pub fn texture(&self) -> &glow::Texture {
        &self.texture
    }
}

pub trait Display {
    fn framebuffer_dimensions(&self) -> (u32, u32);
    fn hidpi_factor(&self) -> f64;
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> Display for glutin::WindowedContext<T>
where
    T: glutin::ContextCurrentState,
{
    fn framebuffer_dimensions(&self) -> (u32, u32) {
        (
            self.window().inner_size().width,
            self.window().inner_size().height,
        )
    }

    fn hidpi_factor(&self) -> f64 {
        self.window().scale_factor()
    }
}

#[cfg(target_arch = "wasm32")]
impl Display for (u32, u32, f64) {
    fn framebuffer_dimensions(&self) -> (u32, u32) {
        (self.0, self.1)
    }

    fn hidpi_factor(&self) -> f64 {
        self.2
    }
}

impl Renderer {
    /// Construct a new empty `Renderer`.
    ///
    /// The dimensions of the inner glyph cache will be equal to the dimensions of the given
    /// facade's framebuffer.
    pub fn new(gl: &glow::Context, is_framebuffer_srgb: bool) -> Result<Self, String> {
        let glyph_cache = GlyphCache::new(gl)?;
        Self::with_glyph_cache(gl, glyph_cache, is_framebuffer_srgb)
    }

    /// Construct a new empty `Renderer` with the given glyph cache dimensions.
    pub fn with_glyph_cache_dimensions(
        gl: &glow::Context,
        width: u32,
        height: u32,
        is_framebuffer_srgb: bool,
    ) -> Result<Self, String> {
        let glyph_cache = GlyphCache::with_dimensions(gl, width, height)?;
        Self::with_glyph_cache(gl, glyph_cache, is_framebuffer_srgb)
    }

    // Construct a new **Renderer** that uses the given glyph cache for caching text.
    fn with_glyph_cache(
        gl: &glow::Context,
        gc: GlyphCache,
        is_framebuffer_srgb: bool,
    ) -> Result<Self, String> {
        let program = program(gl, is_framebuffer_srgb)?;
        let vbo;
        let vao;
        unsafe {
            vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            gl.enable_vertex_attrib_array(program.attrib_mode);
            gl.enable_vertex_attrib_array(program.attrib_position);
            gl.enable_vertex_attrib_array(program.attrib_tex_coords);
            gl.enable_vertex_attrib_array(program.attrib_color);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = 9 * 4;
            assert_eq!(std::mem::size_of::<Vertex>(), stride as _);
            gl.vertex_attrib_pointer_i32(program.attrib_mode, 1, glow::UNSIGNED_INT, stride, 0);
            gl.vertex_attrib_pointer_f32(
                program.attrib_position,
                2,
                glow::FLOAT,
                false,
                stride,
                1 * 4,
            );
            gl.vertex_attrib_pointer_f32(
                program.attrib_tex_coords,
                2,
                glow::FLOAT,
                false,
                stride,
                3 * 4,
            );
            gl.vertex_attrib_pointer_f32(
                program.attrib_color,
                4,
                glow::FLOAT,
                false,
                stride,
                5 * 4,
            );
        }
        Ok(Renderer {
            program,
            vbo,
            vao,
            glyph_cache: gc,
            commands: Vec::new(),
            vertices: Vec::new(),
        })
    }

    /// Produce an `Iterator` yielding `Command`s.
    pub fn commands(&self) -> Commands {
        let Renderer {
            ref commands,
            ref vertices,
            ..
        } = *self;
        Commands {
            commands: commands.iter(),
            vertices: vertices,
        }
    }

    /// Fill the inner vertex and command buffers by translating the given `primitives`.
    pub fn fill<D, P>(
        &mut self,
        display: &D,
        gl: &glow::Context,
        mut primitives: P,
        image_map: &image::Map<Texture>,
    ) where
        P: render::PrimitiveWalker,
        D: Display,
    {
        let Renderer {
            ref mut commands,
            ref mut vertices,
            ref mut glyph_cache,
            ..
        } = *self;

        commands.clear();
        vertices.clear();

        enum State {
            Image { image_id: image::Id, start: usize },
            Plain { start: usize },
        }

        let mut current_state = State::Plain { start: 0 };

        // Switches to the `Plain` state and completes the previous `Command` if not already in the
        // `Plain` state.
        macro_rules! switch_to_plain_state {
            () => {
                match current_state {
                    State::Plain { .. } => (),
                    State::Image { image_id, start } => {
                        commands.push(PreparedCommand::Image(image_id, start..vertices.len()));
                        current_state = State::Plain {
                            start: vertices.len(),
                        };
                    }
                }
            };
        }

        // Framebuffer dimensions and the "dots per inch" factor.
        let (screen_w, screen_h) = display.framebuffer_dimensions();
        let (win_w, win_h) = (screen_w as Scalar, screen_h as Scalar);
        let half_win_w = win_w / 2.0;
        let half_win_h = win_h / 2.0;
        let dpi_factor = display.hidpi_factor() as Scalar;

        // Functions for converting for conrod scalar coords to GL vertex coords (-1.0 to 1.0).
        let vx = |x: Scalar| (x * dpi_factor / half_win_w) as f32;
        let vy = |y: Scalar| (y * dpi_factor / half_win_h) as f32;

        let mut current_scizzor = GlRect {
            left: 0,
            width: screen_w,
            bottom: 0,
            height: screen_h,
        };

        let rect_to_gl_rect = |rect: Rect| {
            let (w, h) = rect.w_h();
            let left = (rect.left() * dpi_factor + half_win_w).round() as u32;
            let bottom = (rect.bottom() * dpi_factor + half_win_h).round() as u32;
            let width = (w * dpi_factor).round() as u32;
            let height = (h * dpi_factor).round() as u32;
            GlRect {
                left: std::cmp::max(left, 0),
                bottom: std::cmp::max(bottom, 0),
                width: std::cmp::min(width, screen_w),
                height: std::cmp::min(height, screen_h),
            }
        };

        // Draw each primitive in order of depth.
        while let Some(primitive) = primitives.next_primitive() {
            let render::Primitive {
                kind,
                scizzor,
                rect,
                ..
            } = primitive;

            // Check for a `Scizzor` command.
            let new_scizzor = rect_to_gl_rect(scizzor);
            if new_scizzor != current_scizzor {
                // Finish the current command.
                match current_state {
                    State::Plain { start } => {
                        commands.push(PreparedCommand::Plain(start..vertices.len()))
                    }
                    State::Image { image_id, start } => {
                        commands.push(PreparedCommand::Image(image_id, start..vertices.len()))
                    }
                }

                // Update the scizzor and produce a command.
                current_scizzor = new_scizzor;
                commands.push(PreparedCommand::Scizzor(new_scizzor));

                // Set the state back to plain drawing.
                current_state = State::Plain {
                    start: vertices.len(),
                };
            }

            match kind {
                render::PrimitiveKind::Rectangle { color } => {
                    switch_to_plain_state!();

                    let color = gamma_srgb_to_linear(color.to_fsa());
                    let (l, r, b, t) = rect.l_r_b_t();

                    let v = |x, y| {
                        // Convert from conrod Scalar range to GL range -1.0 to 1.0.
                        Vertex {
                            position: [vx(x), vy(y)],
                            tex_coords: [0.0, 0.0],
                            color: color,
                            mode: MODE_GEOMETRY,
                        }
                    };

                    let mut push_v = |x, y| vertices.push(v(x, y));

                    // Bottom left triangle.
                    push_v(l, t);
                    push_v(r, b);
                    push_v(l, b);

                    // Top right triangle.
                    push_v(l, t);
                    push_v(r, b);
                    push_v(r, t);
                }

                render::PrimitiveKind::TrianglesSingleColor { color, triangles } => {
                    if triangles.is_empty() {
                        continue;
                    }

                    switch_to_plain_state!();

                    let color = gamma_srgb_to_linear(color.into());

                    let v = |p: [Scalar; 2]| Vertex {
                        position: [vx(p[0]), vy(p[1])],
                        tex_coords: [0.0, 0.0],
                        color: color,
                        mode: MODE_GEOMETRY,
                    };

                    for triangle in triangles {
                        vertices.push(v(triangle[0]));
                        vertices.push(v(triangle[1]));
                        vertices.push(v(triangle[2]));
                    }
                }

                render::PrimitiveKind::TrianglesMultiColor { triangles } => {
                    if triangles.is_empty() {
                        continue;
                    }

                    switch_to_plain_state!();

                    let v = |(p, c): ([Scalar; 2], color::Rgba)| Vertex {
                        position: [vx(p[0]), vy(p[1])],
                        tex_coords: [0.0, 0.0],
                        color: gamma_srgb_to_linear(c.into()),
                        mode: MODE_GEOMETRY,
                    };

                    for triangle in triangles {
                        vertices.push(v(triangle[0]));
                        vertices.push(v(triangle[1]));
                        vertices.push(v(triangle[2]));
                    }
                }

                render::PrimitiveKind::Text {
                    color,
                    text,
                    font_id,
                } => {
                    switch_to_plain_state!();

                    let positioned_glyphs = text.positioned_glyphs(dpi_factor as f32);

                    let GlyphCache {
                        ref mut cache,
                        ref mut texture,
                    } = *glyph_cache;

                    // Queue the glyphs to be cached.
                    for glyph in positioned_glyphs.iter() {
                        cache.queue_glyph(font_id.index(), glyph.clone());
                    }

                    // Cache the glyphs on the GPU.
                    cache
                        .cache_queued(|rect, data| {
                            let w = rect.width();
                            let h = rect.height();

                            unsafe {
                                gl.bind_texture(glow::TEXTURE_2D, Some(*texture));
                                assert_eq!(w * h, data.len() as _);
                                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
                                gl.tex_sub_image_2d(
                                    glow::TEXTURE_2D,
                                    0,
                                    rect.min.x as i32,
                                    rect.min.y as i32,
                                    w as i32,
                                    h as i32,
                                    glow::RED,
                                    glow::UNSIGNED_BYTE,
                                    glow::PixelUnpackData::Slice(data),
                                );
                                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
                            }
                        })
                        .unwrap();

                    let color = gamma_srgb_to_linear(color.to_fsa());

                    let cache_id = font_id.index();

                    let origin = text::rt::point(0.0, 0.0);
                    let to_gl_rect = |screen_rect: text::rt::Rect<i32>| text::rt::Rect {
                        min: origin
                            + (text::rt::vector(
                                screen_rect.min.x as f32 / screen_w as f32 - 0.5,
                                1.0 - screen_rect.min.y as f32 / screen_h as f32 - 0.5,
                            )) * 2.0,
                        max: origin
                            + (text::rt::vector(
                                screen_rect.max.x as f32 / screen_w as f32 - 0.5,
                                1.0 - screen_rect.max.y as f32 / screen_h as f32 - 0.5,
                            )) * 2.0,
                    };

                    for g in positioned_glyphs {
                        if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(cache_id, g) {
                            let gl_rect = to_gl_rect(screen_rect);
                            let v = |p, t| Vertex {
                                position: p,
                                tex_coords: t,
                                color: color,
                                mode: MODE_TEXT,
                            };
                            let mut push_v = |p, t| vertices.push(v(p, t));
                            push_v(
                                [gl_rect.min.x, gl_rect.max.y],
                                [uv_rect.min.x, uv_rect.max.y],
                            );
                            push_v(
                                [gl_rect.min.x, gl_rect.min.y],
                                [uv_rect.min.x, uv_rect.min.y],
                            );
                            push_v(
                                [gl_rect.max.x, gl_rect.min.y],
                                [uv_rect.max.x, uv_rect.min.y],
                            );
                            push_v(
                                [gl_rect.max.x, gl_rect.min.y],
                                [uv_rect.max.x, uv_rect.min.y],
                            );
                            push_v(
                                [gl_rect.max.x, gl_rect.max.y],
                                [uv_rect.max.x, uv_rect.max.y],
                            );
                            push_v(
                                [gl_rect.min.x, gl_rect.max.y],
                                [uv_rect.min.x, uv_rect.max.y],
                            );
                        }
                    }
                }

                render::PrimitiveKind::Image {
                    image_id,
                    color,
                    source_rect,
                } => {
                    // Switch to the `Image` state for this image if we're not in it already.
                    let new_image_id = image_id;
                    match current_state {
                        // If we're already in the drawing mode for this image, we're done.
                        State::Image { image_id, .. } if image_id == new_image_id => (),

                        // If we were in the `Plain` drawing state, switch to Image drawing state.
                        State::Plain { start } => {
                            commands.push(PreparedCommand::Plain(start..vertices.len()));
                            current_state = State::Image {
                                image_id: new_image_id,
                                start: vertices.len(),
                            };
                        }

                        // If we were drawing a different image, switch state to draw *this* image.
                        State::Image { image_id, start } => {
                            commands.push(PreparedCommand::Image(image_id, start..vertices.len()));
                            current_state = State::Image {
                                image_id: new_image_id,
                                start: vertices.len(),
                            };
                        }
                    }

                    let color = color.unwrap_or(color::WHITE).to_fsa();

                    if let Some(image) = image_map.get(&image_id) {
                        let (image_w, image_h) = (image.width, image.height);
                        let (image_w, image_h) = (image_w as Scalar, image_h as Scalar);

                        // Get the sides of the source rectangle as uv coordinates.
                        //
                        // Texture coordinates range:
                        // - left to right: 0.0 to 1.0
                        // - bottom to top: 0.0 to 1.0
                        let (uv_l, uv_r, uv_b, uv_t) = match source_rect {
                            Some(src_rect) => {
                                let (l, r, b, t) = src_rect.l_r_b_t();
                                (
                                    (l / image_w) as f32,
                                    (r / image_w) as f32,
                                    (b / image_h) as f32,
                                    (t / image_h) as f32,
                                )
                            }
                            None => (0.0, 1.0, 0.0, 1.0),
                        };

                        let v = |x, y, t| {
                            // Convert from conrod Scalar range to GL range -1.0 to 1.0.
                            let x = (x * dpi_factor as Scalar / half_win_w) as f32;
                            let y = (y * dpi_factor as Scalar / half_win_h) as f32;
                            Vertex {
                                position: [x, y],
                                tex_coords: t,
                                color: color,
                                mode: MODE_IMAGE,
                            }
                        };

                        let mut push_v = |x, y, t| vertices.push(v(x, y, t));

                        let (l, r, b, t) = rect.l_r_b_t();

                        // Bottom left triangle.
                        push_v(l, t, [uv_l, uv_t]);
                        push_v(r, b, [uv_r, uv_b]);
                        push_v(l, b, [uv_l, uv_b]);

                        // Top right triangle.
                        push_v(l, t, [uv_l, uv_t]);
                        push_v(r, b, [uv_r, uv_b]);
                        push_v(r, t, [uv_r, uv_t]);
                    }
                }

                // We have no special case widgets to handle.
                render::PrimitiveKind::Other(_) => (),
            }
        }

        // Enter the final command.
        match current_state {
            State::Plain { start } => commands.push(PreparedCommand::Plain(start..vertices.len())),
            State::Image { image_id, start } => {
                commands.push(PreparedCommand::Image(image_id, start..vertices.len()))
            }
        }
    }

    /// Draws using the inner list of `Command`s to the given `display`.
    ///
    /// Note: If you require more granular control over rendering, you may want to use the `fill`
    /// and `commands` methods separately. This method is simply a convenience wrapper around those
    /// methods for the case that the user does not require accessing or modifying conrod's draw
    /// parameters, uniforms or generated draw commands.
    pub fn draw(&self, gl: &glow::Context, image_map: &image::Map<Texture>) -> Result<(), String> {
        macro_rules! verify {
            () => {{
                let err = gl.get_error();
                if err != 0 {
                    panic!("gl error {}", err);
                }
            }};
        }
        unsafe fn to_raw_bytes<T>(src: &[T]) -> &[u8] {
            std::slice::from_raw_parts(
                src.as_ptr() as *const u8,
                src.len() * std::mem::size_of::<T>(),
            )
        }

        let glyph_texture = *self.glyph_cache.texture();

        const NUM_VERTICES_IN_TRIANGLE: usize = 3;

        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            verify!();
            gl.use_program(Some(self.program.program));
            verify!();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            verify!();
            gl.bind_vertex_array(Some(self.vao));
            verify!();
        }

        for command in self.commands() {
            match command {
                // Update the `scizzor` before continuing to draw.
                Command::Scizzor(scizzor) => {
                    unsafe {
                        gl.enable(glow::SCISSOR_TEST);
                        verify!();
                        gl.scissor(
                            scizzor.left as i32,
                            scizzor.bottom as i32,
                            scizzor.width as i32,
                            scizzor.height as i32,
                        );
                    }
                }

                // Draw to the target with the given `draw` command.
                Command::Draw(draw) => match draw {
                    // Draw text and plain 2D geometry.
                    //
                    // Only submit the vertices if there is enough for at least one triangle.
                    Draw::Plain(slice) => {
                        if slice.len() >= NUM_VERTICES_IN_TRIANGLE {
                            unsafe {
                                gl.bind_texture(glow::TEXTURE_2D, Some(glyph_texture));
                                verify!();
                                let x = to_raw_bytes(slice);
                                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, x, glow::DYNAMIC_DRAW);
                                verify!();
                                gl.draw_arrays(glow::TRIANGLES, 0, slice.len() as i32);
                                verify!();
                            }
                        }
                    }

                    // Draw an image whose texture data lies within the `image_map` at the
                    // given `id`.
                    //
                    // Only submit the vertices if there is enough for at least one triangle.
                    Draw::Image(image_id, slice) => {
                        if slice.len() >= NUM_VERTICES_IN_TRIANGLE {
                            unsafe {
                                if let Some(image) = image_map.get(&image_id) {
                                    gl.bind_texture(glow::TEXTURE_2D, Some(image.texture));
                                    verify!();
                                } else {
                                    gl.bind_texture(glow::TEXTURE_2D, None);
                                    verify!();
                                }
                                let x = to_raw_bytes(slice);
                                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, x, glow::DYNAMIC_DRAW);
                                verify!();
                                gl.draw_arrays(glow::TRIANGLES, 0, slice.len() as i32);
                                verify!();
                            }
                        }
                    }
                },
            }
        }

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, None);
            verify!();
            gl.use_program(None);
            verify!();
            gl.bind_vertex_array(None);
            verify!();
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            verify!();
            gl.disable(glow::SCISSOR_TEST);
            verify!();
        }

        Ok(())
    }
}

impl<'a> Iterator for Commands<'a> {
    type Item = Command<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let Commands {
            ref mut commands,
            ref vertices,
        } = *self;
        commands.next().map(|command| match *command {
            PreparedCommand::Scizzor(scizzor) => Command::Scizzor(scizzor),
            PreparedCommand::Plain(ref range) => {
                Command::Draw(Draw::Plain(&vertices[range.clone()]))
            }
            PreparedCommand::Image(id, ref range) => {
                Command::Draw(Draw::Image(id, &vertices[range.clone()]))
            }
        })
    }
}
