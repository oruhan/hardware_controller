// SPDX-License-Identifier: Apache-2.0

use std::f32::consts::TAU;
use std::time::Duration;

use razer::DaV3HS;
use xengui::{ *, task::{ spawn, spawn_blocking } };
use xenframe::{ App, AppConfig, WindowPosition };

#[derive(Debug, Clone, Default)]
struct MouseState {
    connected: bool,
    battery: u8,
    raw_battery: u8,
    charging: bool,
    error: Option<String>,
}

// Green above 50%, amber between 20-50%, red at or below 20% - mirrors
// the usual OS battery indicator convention.
fn battery_color(pct: u8) -> Color {
    if pct <= 20 { Color::RED_500 } else if pct <= 50 { Color::AMBER_500 } else { Color::GREEN_500 }
}

fn led_opacity(charging: bool, phase: f32) -> f32 {
    if !charging {
        return 1.0;
    }
    0.35 + 0.65 * (0.5 + 0.5 * phase.sin())
}

fn connection_badge(connected: bool) -> View {
    let (bg, dot_color, label) = if connected {
        (Color::GREEN_500.with_alpha(30), Color::GREEN_500, "Connected")
    } else {
        (Color::RED_500.with_alpha(30), Color::RED_500, "Disconnected")
    };

    Row::new()
        .align_items(Align::Center)
        .gap(6.0, 0.0)
        .padding(Edges::only(10, 5, 10, 4))
        .background(bg)
        .border(Border::all(0, Color::TRANSPARENT).radius(64))
        .child(
            View::new()
                .width(8)
                .height(8)
                .background(dot_color)
                .border(Border::all(0, Color::TRANSPARENT).radius(64))
        )
        .child(
            Label::new().label(label).font_size(13).color(dot_color).font_weight(FontWeight::Medium)
        )
}

fn charging_badge(charging: bool) -> View {
    let (bg, text_color, label) = if charging {
        (Color::BLUE_500.with_alpha(30), Color::BLUE_500, "Charging")
    } else {
        (Color::NEUTRAL_500.with_alpha(40), Color::NEUTRAL_300, "Discharging")
    };

    Row::new()
        .align_items(Align::Center)
        .padding(Edges::only(10, 5, 10, 4))
        .background(bg)
        .border(Border::all(0, Color::TRANSPARENT).radius(64))
        .child(
            Label::new()
                .label(label)
                .font_size(13)
                .color(text_color)
                .font_weight(FontWeight::Medium)
        )
}

fn header(state: &MouseState, breathe_phase: f32) -> View {
    let led_color = battery_color(state.battery);
    let opacity = led_opacity(state.charging, breathe_phase);

    Row::new()
        .width(pct!(100.0))
        .align_items(Align::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(14.0, 0.0)
                .child(
                    Svg::from_string(include_str!("../assets/mouse.svg"))
                        .fill_by_id("mouse_led", led_color)
                        .opacity_by_id("mouse_led", opacity)
                        .width(72)
                        .height(72)
                )
                .child(
                    Column::new()
                        .gap(0.0, 8.0)
                        .child(
                            Label::new()
                                .label("Razer DeathAdder V3 HyperSpeed")
                                .font_size(20)
                                .font_weight(FontWeight::SemiBold)
                                .color(|theme: &Theme| theme.on_background)
                        )
                        .child(
                            Row::new()
                                .gap(4.0, 0.0)
                                .child(connection_badge(state.connected))
                                .child(charging_badge(state.charging))
                        )
                )
        )
}

fn battery_card(pct: u8) -> View {
    let theme = current_theme();

    View::new()
        .display(Display::Flex)
        .flex_direction(FlexDirection::Column)
        .width(pct!(100.0))
        .gap(0.0, 14.0)
        .padding(Edges::all(18.0))
        .background(theme.surface)
        .border(Border::all(1, theme.outline_variant).radius(theme.radius_lg))
        .box_shadow(BoxShadow::new(0.0, 2.0, 10.0, Color::NEUTRAL_950.with_alpha(18)))
        .child(
            Row::new()
                .justify_content(JustifyContent::SpaceBetween)
                .align_items(Align::Center)
                .child(Label::new().label("Battery").font_size(14).color(theme.on_surface_variant))
                .child(
                    Label::new()
                        .label(format!("{pct}%"))
                        .font_size(22)
                        .font_weight(FontWeight::SemiBold)
                        .color(theme.on_surface)
                )
        )
        .child(
            View::new()
                .width(pct!(100.0))
                .height(10)
                .background(theme.surface_container_high)
                .border(Border::all(0, Color::TRANSPARENT).radius(64))
                .child(
                    View::new()
                        .height(pct!(100.0))
                        .width(pct!(pct as f32))
                        .background(battery_color(pct))
                        .border(Border::all(0, Color::TRANSPARENT).radius(64))
                        .transition_all(
                            Transition::new(Duration::from_millis(400)).easing(Easing::EaseOut)
                        )
                )
        )
}

fn stat_row(label: &str, value: String) -> View {
    let theme = current_theme();

    Row::new()
        .width(pct!(100.0))
        .justify_content(JustifyContent::SpaceBetween)
        .padding(Edges::symmetric(0.0, 7.0))
        .border(Border::bottom(1, theme.outline_variant.with_alpha(120)))
        .child(Label::new().label(label).font_size(13).color(theme.on_surface_variant))
        .child(
            Label::new()
                .label(value)
                .font_size(13)
                .font_weight(FontWeight::Medium)
                .color(theme.on_surface)
        )
}

fn status_card(state: &MouseState) -> View {
    let theme = current_theme();

    let mut card = View::new()
        .display(Display::Flex)
        .flex_direction(FlexDirection::Column)
        .width(pct!(100.0))
        .padding(Edges::all(18.0))
        .background(theme.surface)
        .border(Border::all(1, theme.outline_variant).radius(theme.radius_lg))
        .box_shadow(BoxShadow::new(0.0, 2.0, 10.0, Color::NEUTRAL_950.with_alpha(18)))
        .child(
            Label::new()
                .label("Status")
                .font_size(14)
                .font_weight(FontWeight::SemiBold)
                .color(theme.on_surface)
                .margin(Edges::only(0, 0, 0, 10))
        )
        .child(
            stat_row("State", (if state.charging { "Charging" } else { "Discharging" }).to_string())
        )
        .child(stat_row("Raw value", format!("0x{:02X}", state.raw_battery)))
        .child(
            stat_row(
                "Connection type",
                (if state.charging { "Wired" } else { "Wireless" }).to_string()
            )
        );

    if let Some(err) = &state.error {
        card = card.child(
            View::new()
                .width(pct!(100.0))
                .margin(Edges::only(0, 10, 0, 0))
                .padding(Edges::symmetric(10.0, 8.0))
                .background(Color::RED_500.with_alpha(24))
                .border(Border::all(1, Color::RED_500.with_alpha(80)).radius(theme.radius_sm))
                .child(Label::new().label(err.clone()).font_size(12).color(Color::RED_500))
        );
    }

    card
}

fn build_ui(state: &MouseState, breathe_phase: f32) -> Box<dyn Widget> {
    let theme = current_theme();

    Box::new(
        View::new()
            .width(pct!(100.0))
            .height(pct!(100.0))
            .background(theme.background)
            .padding(Edges::all(24.0))
            .child(
                Column::new()
                    .width(pct!(100.0))
                    .gap(0.0, 20.0)
                    .child(header(state, breathe_phase))
                    .child(battery_card(state.battery))
                    .child(status_card(state))
            )
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig {
        title: "Razer Monitor".into(),
        width: 480,
        height: 480,
        position: WindowPosition::Center,
        ..Default::default()
    };

    let mut app = App::new(config);

    app.render(|| {
        let (state, set_state) = use_state(MouseState::default());
        let (breathe_phase, set_breathe_phase) = use_state(0.0f32);

        use_effect(move || {
            spawn(async move {
                loop {
                    let mouse = match spawn_blocking(DaV3HS::open).await {
                        Ok(mouse) => {
                            println!("[Razer] Mouse connected.");

                            set_state.update(|state| {
                                state.connected = true;
                                state.error = None;
                            });

                            mouse
                        }

                        Err(error) => {
                            println!("[Razer] Failed to connect: {error}");

                            set_state.update(|state| {
                                state.connected = false;
                                state.error = Some(error.to_string());
                            });

                            spawn_blocking(|| {
                                std::thread::sleep(Duration::from_secs(1));
                            }).await;

                            continue;
                        }
                    };

                    let mut mouse = mouse;

                    loop {
                        let (returned_mouse, result) = spawn_blocking(move || {
                            let result = mouse.battery();
                            (mouse, result)
                        }).await;

                        mouse = returned_mouse;

                        match result {
                            Ok(battery) => {
                                println!(
                                    "[Razer] Battery: {}% | Raw: 0x{:02X} | State: {:?}",
                                    battery.percentage,
                                    battery.raw,
                                    battery.state
                                );

                                set_state.update(|state| {
                                    state.connected = true;
                                    state.battery = battery.percentage;
                                    state.raw_battery = battery.raw;
                                    state.charging = battery.is_charging();
                                    state.error = None;
                                });
                            }

                            Err(error) => {
                                println!("[Razer] Communication error: {error}");

                                set_state.update(|state| {
                                    state.connected = false;
                                    state.error = Some(error.to_string());
                                });

                                println!("[Razer] Mouse disconnected.");

                                break;
                            }
                        }

                        spawn_blocking(|| {
                            std::thread::sleep(Duration::from_millis(1000));
                        }).await;
                    }

                    spawn_blocking(|| {
                        std::thread::sleep(Duration::from_secs(1));
                    }).await;
                }
            });
        }, ());

        // Drives the LED's breathing animation independently of the
        // battery polling loop above - a 2 second sine period, ticked at
        // ~30fps so the fade reads as smooth rather than stepped.
        use_effect(move || {
            spawn(async move {
                let mut phase: f32 = 0.0;
                loop {
                    spawn_blocking(|| {
                        std::thread::sleep(Duration::from_millis(33));
                    }).await;

                    phase = (phase + (TAU / 2.0) * 0.033) % TAU;
                    set_breathe_phase.set(phase);
                }
            });
        }, ());

        build_ui(&state, breathe_phase)
    });

    app.run()?;

    Ok(())
}
