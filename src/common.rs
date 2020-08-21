use conrod_core::{widget, widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon};
use conrod_floatwin::{WinId, WindowBuilder, WindowingArea, WindowingContext, WindowingState};

#[derive(WidgetCommon)]
pub struct ExampleWidget<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    conrod_example_app: &'a mut conrod_example_shared::DemoApp,
}

impl<'a> ExampleWidget<'a> {
    pub fn new(conrod_example_app: &'a mut conrod_example_shared::DemoApp) -> Self {
        ExampleWidget {
            common: widget::CommonBuilder::default(),
            conrod_example_app,
        }
    }
}

impl<'a> Widget for ExampleWidget<'a> {
    type State = conrod_example_shared::Ids;
    type Style = ();
    type Event = ();

    fn init_state(&self, id_gen: conrod_core::widget::id::Generator) -> Self::State {
        conrod_example_shared::Ids::new(id_gen)
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: conrod_core::widget::UpdateArgs<Self>) -> Self::Event {
        conrod_example_shared::gui(args.ui, args.state, self.conrod_example_app);
    }
}

widget_ids! {
    pub struct Ids {
        backdrop,
        windowing_area,
        text,
        button,
        conrod_example,
    }
}

pub struct WinIds {
    pub conrod_example: WinId,
}

pub struct UiState {
    pub enable_debug: bool,
    pub win_state: WindowingState,
    pub win_ids: WinIds,
    pub conrod_example_app: conrod_example_shared::DemoApp,
}

pub fn set_widgets(
    ref mut ui: conrod_core::UiCell,
    ids: &mut Ids,
    hidpi_factor: f64,
    state: &mut UiState,
) {
    widget::Rectangle::fill(ui.window_dim())
        .color(conrod_core::color::BLUE)
        .middle()
        .set(ids.backdrop, ui);
    let mut win_ctx: WindowingContext = WindowingArea::new(&mut state.win_state, hidpi_factor)
        .with_debug(state.enable_debug)
        .middle_of(ids.backdrop)
        .wh_of(ids.backdrop)
        .crop_kids()
        .set(ids.windowing_area, ui);

    let builder = WindowBuilder::new()
        .title("Conrod Example")
        .initial_size([640.0, 480.0])
        .min_size([320.0, 240.0]);
    if let (_, Some(win)) = win_ctx.make_window(builder, state.win_ids.conrod_example, ui) {
        let example = ExampleWidget::new(&mut state.conrod_example_app);
        win.set(example, ui);
    }
}
