#[macro_export]
macro_rules! v023_convert_key {
    ($keycode:expr) => {{
        ::conrod_winit::v021_convert_key!($keycode)
    }};
}

/// Maps winit's mouse button to conrod's mouse button.
///
/// Expects a `winit::MouseButton` as input and returns a `conrod_core::input::MouseButton` as
/// output.
///
/// Requires that both the `conrod_core` and `winit` crates are in the crate root.
#[macro_export]
macro_rules! v023_convert_mouse_button {
    ($mouse_button:expr) => {{
        ::conrod_winit::v021_convert_mouse_button!($mouse_button)
    }};
}

/// A macro for converting a `winit::WindowEvent` to a `Option<conrod_core::event::Input>`.
///
/// Expects a `winit::WindowEvent` and a reference to a window implementing `WinitWindow`.
/// Returns an `Option<conrod_core::event::Input>`.
#[macro_export]
macro_rules! v023_convert_window_event {
    ($event:expr, $window:expr) => {{
        // The window size in points.
        let scale_factor: f64 = $window.scale_factor();
        let (win_w, win_h): (f64, f64) = $window.inner_size().to_logical::<f64>(scale_factor).into();

        // Translate the coordinates from top-left-origin-with-y-down to centre-origin-with-y-up.
        let tx = |x: conrod_core::Scalar| x - win_w / 2.0;
        let ty = |y: conrod_core::Scalar| -(y - win_h / 2.0);

        // Functions for converting keys and mouse buttons.
        let map_key = |key: winit::event::VirtualKeyCode| ::conrod_winit::v021_convert_key!(key);
        let map_mouse = |button: winit::event::MouseButton| ::conrod_winit::v021_convert_mouse_button!(button);

        match $event {
            winit::event::WindowEvent::Resized(physical_size) => {
                let winit::dpi::LogicalSize { width, height } = physical_size.to_logical(scale_factor);
                Some(conrod_core::event::Input::Resize(width, height).into())
            },

            winit::event::WindowEvent::ReceivedCharacter(ch) => {
                let string = match ch {
                    // Ignore control characters and return ascii for Text event (like sdl2).
                    '\u{7f}' | // Delete
                    '\u{1b}' | // Escape
                    '\u{8}'  | // Backspace
                    '\r' | '\n' | '\t' => "".to_string(),
                    _ => ch.to_string()
                };
                Some(conrod_core::event::Input::Text(string).into())
            },

            winit::event::WindowEvent::Focused(focused) =>
                Some(conrod_core::event::Input::Focus(focused.clone()).into()),

            winit::event::WindowEvent::KeyboardInput { input, .. } => {
                input.virtual_keycode.map(|key| {
                    match input.state {
                        winit::event::ElementState::Pressed =>
                            conrod_core::event::Input::Press(conrod_core::input::Button::Keyboard(map_key(key))).into(),
                        winit::event::ElementState::Released =>
                            conrod_core::event::Input::Release(conrod_core::input::Button::Keyboard(map_key(key))).into(),
                    }
                })
            },

            winit::event::WindowEvent::Touch(winit::event::Touch { phase, location, id, .. }) => {
                let winit::dpi::LogicalPosition { x, y } = location.to_logical::<f64>(scale_factor);
                let phase = match phase {
                    winit::event::TouchPhase::Started => conrod_core::input::touch::Phase::Start,
                    winit::event::TouchPhase::Moved => conrod_core::input::touch::Phase::Move,
                    winit::event::TouchPhase::Cancelled => conrod_core::input::touch::Phase::Cancel,
                    winit::event::TouchPhase::Ended => conrod_core::input::touch::Phase::End,
                };
                let xy = [tx(x), ty(y)];
                let id = conrod_core::input::touch::Id::new(id.clone());
                let touch = conrod_core::input::Touch { phase: phase, id: id, xy: xy };
                Some(conrod_core::event::Input::Touch(touch).into())
            }

            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let winit::dpi::LogicalPosition { x, y } = position.to_logical::<f64>(scale_factor);
                let x = tx(x as conrod_core::Scalar);
                let y = ty(y as conrod_core::Scalar);
                let motion = conrod_core::input::Motion::MouseCursor { x: x, y: y };
                Some(conrod_core::event::Input::Motion(motion).into())
            },

            winit::event::WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::PixelDelta(delta) => {
                    let winit::dpi::LogicalPosition { x, y } = delta.to_logical::<f64>(scale_factor);
                    let x = x as conrod_core::Scalar;
                    let y = -y as conrod_core::Scalar;
                    let motion = conrod_core::input::Motion::Scroll { x: x, y: y };
                    Some(conrod_core::event::Input::Motion(motion).into())
                },

                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    // This should be configurable (we should provide a LineDelta event to allow for this).
                    const ARBITRARY_POINTS_PER_LINE_FACTOR: conrod_core::Scalar = 10.0;
                    let x = ARBITRARY_POINTS_PER_LINE_FACTOR * x.clone() as conrod_core::Scalar;
                    let y = ARBITRARY_POINTS_PER_LINE_FACTOR * -y.clone() as conrod_core::Scalar;
                    Some(conrod_core::event::Input::Motion(conrod_core::input::Motion::Scroll { x: x, y: y }).into())
                },
            },

            winit::event::WindowEvent::MouseInput { state, button, .. } => match state {
                winit::event::ElementState::Pressed =>
                    Some(conrod_core::event::Input::Press(conrod_core::input::Button::Mouse(map_mouse(button.clone()))).into()),
                winit::event::ElementState::Released =>
                    Some(conrod_core::event::Input::Release(conrod_core::input::Button::Mouse(map_mouse(button.clone()))).into()),
            },

            _ => None,
        }
    }};
}

/// A macro for converting a `winit::Event` to a `conrod_core::event::Input`.
///
/// Expects a `winit::Event` and a reference to a window implementing `WinitWindow`.
/// Returns an `Option<conrod_core::event::Input>`.
///
/// Invocations of this macro require that a version of the `winit` and `conrod_core` crates are
/// available in the crate root.
#[macro_export]
macro_rules! v023_convert_event {
    ($event:expr, $window:expr) => {{
        match $event {
            winit::event::Event::WindowEvent { event, .. } => $crate::v023_convert_window_event!(event, $window),
            _ => None,
        }
    }};
}

/// Convert a given conrod mouse cursor to the corresponding winit cursor type.
///
/// Expects a `conrod_core::cursor::MouseCursor`, returns a `winit::MouseCursor`.
///
/// Requires that both the `conrod_core` and `winit` crates are in the crate root.
#[macro_export]
macro_rules! v023_convert_mouse_cursor {
    ($cursor:expr) => {{
        ::conrod_winit::v021_convert_mouse_cursor!($cursor)
    }};
}

#[macro_export]
macro_rules! v023_conversion_fns {
    () => {
        /// Generate a set of conversion functions for converting between types of the crate's versions of
        /// `winit` and `conrod_core`.
        /// Maps winit's key to a conrod `Key`.
        ///
        /// Expects a `winit::VirtualKeyCode` as input and returns a `conrod_core::input::keyboard::Key`.
        ///
        /// Requires that both the `winit` and `conrod_core` crates exist within the crate root.
        pub fn convert_key(keycode: winit::event::VirtualKeyCode) -> conrod_core::input::keyboard::Key {
            $crate::v023_convert_key!(keycode)
        }

        /// Convert a `winit::MouseButton` to a `conrod_core::input::MouseButton`.
        pub fn convert_mouse_button(
            mouse_button: winit::event::MouseButton,
        ) -> conrod_core::input::MouseButton {
            $crate::v023_convert_mouse_button!(mouse_button)
        }

        /// Convert a given conrod mouse cursor to the corresponding winit cursor type.
        pub fn convert_mouse_cursor(cursor: conrod_core::cursor::MouseCursor) -> winit::window::CursorIcon {
            $crate::v023_convert_mouse_cursor!(cursor)
        }

        /// A function for converting a `winit::WindowEvent` to a `conrod_core::event::Input`.
        pub fn convert_window_event(
            event: &winit::event::WindowEvent,
            window: &winit::window::Window,
        ) -> Option<conrod_core::event::Input> {
            $crate::v023_convert_window_event!(event, window)
        }

        /// A function for converting a `winit::Event` to a `conrod_core::event::Input`.
        pub fn convert_event<T>(
            event: &winit::event::Event<T>,
            window: &winit::window::Window,
        ) -> Option<conrod_core::event::Input> {
            $crate::v023_convert_event!(event, window)
        }
    };
}
