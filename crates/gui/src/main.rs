// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{ AtomicU64, Ordering };
use std::sync::{ Arc, Mutex };
use std::time::Duration;

use devices::{ BatteryStatus, ChargingState, ConnectionType, DeviceKind, PollingRate, catalog };
use xengui::{ *, task::{ spawn, spawn_blocking } };
use xenframe::{ App, AppConfig, WindowPosition };
use xengui_icons::codepoints;

fn battery_color(pct: u8) -> Color {
    if pct <= 20 { Color::RED_500 } else if pct <= 50 { Color::AMBER_500 } else { Color::GREEN_500 }
}

fn kind_icon(kind: DeviceKind) -> char {
    match kind {
        DeviceKind::Mouse => codepoints::MOUSE,
        DeviceKind::Keyboard => codepoints::KEYBOARD,
        DeviceKind::Headset => codepoints::HEADSET,
        DeviceKind::Other => codepoints::DEVICES_OTHER,
    }
}

#[derive(Clone)]
struct DeviceEntry {
    id: u64,
    brand: String,
    model: String,
    kind: DeviceKind,
    image_svg: &'static str,
    connected: bool,
    battery: Option<BatteryStatus>,
    error: Option<String>,
    supports_polling_rate: bool,
    polling_rate: Option<PollingRate>,
    // True while a set_polling_rate call is in flight, for UI feedback.
    applying_rate: bool,
    pending_rate: Arc<Mutex<Option<PollingRate>>>,
}

fn next_device_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

// Spawns the poll loop that owns the actual `Box<dyn Device>` handle;
// only the display-facing `DeviceEntry` crosses back into GUI state.
fn add_device(
    descriptor: &'static devices::DeviceDescriptor,
    set_devices: SetState<Vec<DeviceEntry>>,
    set_selected: SetState<Option<u64>>
) {
    let id = next_device_id();
    let pending_rate: Arc<Mutex<Option<PollingRate>>> = Arc::new(Mutex::new(None));

    set_devices.update({
        let pending_rate = pending_rate.clone();
        move |list| {
            list.push(DeviceEntry {
                id,
                brand: descriptor.brand.to_string(),
                model: descriptor.model.to_string(),
                kind: descriptor.kind,
                image_svg: descriptor.image_svg,
                connected: false,
                battery: None,
                error: None,
                supports_polling_rate: false,
                polling_rate: None,
                pending_rate,
                applying_rate: false,
            });
        }
    });
    set_selected.set(Some(id));

    spawn(async move {
        loop {
            let opened = spawn_blocking(descriptor.open).await;

            let mut device = match opened {
                Ok(device) => {
                    let mut device = device;
                    let supports_rate = device.supports_polling_rate();

                    let (returned, initial_rate) = spawn_blocking(move || {
                        let rate = if supports_rate {
                            device.get_polling_rate().ok()
                        } else {
                            None
                        };
                        (device, rate)
                    }).await;
                    device = returned;

                    set_devices.update(move |list| {
                        if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
                            entry.connected = true;
                            entry.error = None;
                            entry.supports_polling_rate = supports_rate;
                            entry.polling_rate = initial_rate;
                        }
                    });
                    device
                }
                Err(error) => {
                    set_devices.update(move |list| {
                        if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
                            entry.connected = false;
                            entry.error = Some(error.to_string());
                        }
                    });
                    spawn_blocking(|| std::thread::sleep(Duration::from_secs(1))).await;
                    continue;
                }
            };

            loop {
                let requested_rate = pending_rate.lock().unwrap().take();
                if let Some(rate) = requested_rate {
                    let (returned, set_result) = spawn_blocking(move || {
                        let result = device.set_polling_rate(rate);
                        (device, result)
                    }).await;
                    device = returned;

                    let outcome = set_result.map_err(|e| e.to_string());
                    set_devices.update(move |list| {
                        if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
                            entry.applying_rate = false;
                            match &outcome {
                                Ok(()) => {
                                    entry.polling_rate = Some(rate);
                                    entry.error = None;
                                }
                                Err(msg) => {
                                    entry.error = Some(msg.clone());
                                }
                            }
                        }
                    });
                }

                let (returned, result) = spawn_blocking(move || {
                    let result = device.poll_battery();
                    (device, result)
                }).await;
                device = returned;

                match result {
                    Ok(status) => {
                        set_devices.update(move |list| {
                            if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
                                entry.connected = true;
                                entry.battery = Some(status);
                                entry.error = None;
                            }
                        });
                    }
                    Err(error) => {
                        set_devices.update(move |list| {
                            if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
                                entry.connected = false;
                                entry.error = Some(error.to_string());
                            }
                        });
                        break;
                    }
                }

                // Slept in small chunks so a polling-rate change requested
                // from the UI is picked up quickly instead of waiting for
                // the full poll interval.
                for _ in 0..10 {
                    spawn_blocking(|| std::thread::sleep(Duration::from_millis(100))).await;
                    if pending_rate.lock().unwrap().is_some() {
                        break;
                    }
                }
            }

            spawn_blocking(|| std::thread::sleep(Duration::from_secs(1))).await;
        }
    });
}

fn m3_card() -> View {
    let theme = current_theme();
    View::new()
        .display(Display::Flex)
        .flex_direction(FlexDirection::Column)
        .background(theme.surface_container_low)
        .border(Border::all(0, Color::TRANSPARENT).radius(theme.radius_xl))
        .box_shadow(BoxShadow::new(0.0, 1.0, 3.0, Color::NEUTRAL_950.with_alpha(20)))
}

fn device_list_item(
    entry: &DeviceEntry,
    selected: bool,
    set_selected: SetState<Option<u64>>
) -> View {
    let theme = current_theme();
    let id = entry.id;

    let (bg, fg) = if selected {
        (theme.secondary_container, theme.on_secondary_container)
    } else {
        (Color::TRANSPARENT, theme.on_surface_variant)
    };

    Row::new()
        .width(pct!(100.0))
        .align_items(Align::Center)
        .gap(12.0, 0.0)
        .padding(Edges::symmetric(14.0, 10.0))
        .background(bg)
        .hover_background(theme.surface_container_high.with_alpha_f32(0.6))
        .border(Border::all(0, Color::TRANSPARENT).radius(theme.radius_lg))
        .cursor(Cursor::Pointer)
        .on_click(move |_ctx| set_selected.set(Some(id)))
        .child(VariableIcon::new(kind_icon(entry.kind)).size(22.0).color(fg))
        .child(
            Column::new()
                .gap(0.0, 2.0)
                .flex_grow(1.0)
                .child(
                    Label::new()
                        .label(entry.brand.clone())
                        .font_size(12)
                        .color(if selected { fg } else { theme.on_surface_variant })
                )
                .child(
                    Label::new()
                        .label(entry.model.clone())
                        .font_size(14)
                        .font_weight(FontWeight::Medium)
                        .color(if selected { fg } else { theme.on_surface })
                )
        )
        .child(
            View::new()
                .width(8)
                .height(8)
                .background(if entry.connected { Color::GREEN_500 } else { theme.outline_variant })
                .border(Border::all(0, Color::TRANSPARENT).radius(64))
        )
}

fn device_picker_row(
    descriptor: &'static devices::DeviceDescriptor,
    set_devices: SetState<Vec<DeviceEntry>>,
    set_selected: SetState<Option<u64>>
) -> View {
    let theme = current_theme();
    Row::new()
        .width(pct!(100.0))
        .align_items(Align::Center)
        .gap(10.0, 0.0)
        .padding(Edges::symmetric(12.0, 8.0))
        .hover_background(theme.surface_container_high)
        .border(Border::all(0, Color::TRANSPARENT).radius(theme.radius_md))
        .cursor(Cursor::Pointer)
        .on_click(move |_ctx| add_device(descriptor, set_devices.clone(), set_selected.clone()))
        .child(
            VariableIcon::new(kind_icon(descriptor.kind)).size(20.0).color(theme.on_surface_variant)
        )
        .child(
            Label::new()
                .label(format!("{} {}", descriptor.brand, descriptor.model))
                .font_size(14)
                .color(theme.on_surface)
        )
}

fn sidebar(
    devices_list: &[DeviceEntry],
    selected: Option<u64>,
    set_selected: SetState<Option<u64>>,
    set_devices: SetState<Vec<DeviceEntry>>,
    picker_open: bool,
    set_picker_open: SetState<bool>
) -> View {
    let theme = current_theme();

    let mut list = View::new()
        .key("device_list_rows")
        .display(Display::Flex)
        .flex_direction(FlexDirection::Column)
        .gap(0.0, 4.0);

    for entry in devices_list {
        list = list.child(
            device_list_item(entry, selected == Some(entry.id), set_selected.clone()).key(
                entry.id.to_string()
            )
        );
    }

    let add_button = Button::new()
        .label("Cihaz ekle")
        .icon(
            r#"<svg viewBox="0 0 24 24"><path d="M11 13H5v-2h6V5h2v6h6v2h-6v6h-2z" fill="currentColor"/></svg>"#
        )
        .icon_size(18.0, 18.0)
        .width(pct!(100.0))
        .justify_content(JustifyContent::Center)
        .padding(Edges::symmetric(16.0, 10.0))
        .background(theme.secondary_container)
        .color(theme.on_secondary_container)
        .border(Border::all(0, Color::TRANSPARENT).radius(theme.radius_4xl))
        .on_click(move |_ctx| set_picker_open.set(!picker_open));

    let mut column = Column::new()
        .width(320)
        .height(pct!(100.0))
        .gap(0.0, 12.0)
        .padding(Edges::all(16.0))
        .background(theme.surface_container)
        .border(Border::right(1, theme.outline_variant))
        .child(
            Label::new()
                .label("Cihazlarım")
                .font_size(20)
                .font_weight(FontWeight::SemiBold)
                .color(theme.on_surface)
        )
        .child(add_button);

    if picker_open {
        let mut picker = m3_card().padding(Edges::all(8.0)).gap(0.0, 2.0);
        for descriptor in catalog() {
            picker = picker.child(
                device_picker_row(descriptor, set_devices.clone(), set_selected.clone())
            );
        }
        column = column.child(picker);
    }

    // Column direction here keeps overflow along the height axis (main
    // axis) and stretches width (cross axis) - a Row wrapper would clip
    // height to the container and never register overflow.
    column = column.child(
        Column::new().flex_grow(1.0).width(pct!(100.0)).overflow_y(Overflow::Auto).child(list)
    );

    column
}

fn device_illustration(entry: &DeviceEntry, breathe_phase: f32) -> View {
    let theme = current_theme();
    let led_opacity = if
        entry.battery.map(|b| matches!(b.state, ChargingState::Charging)).unwrap_or(false)
    {
        0.35 + 0.65 * (0.5 + 0.5 * breathe_phase.sin())
    } else {
        1.0
    };
    let led_color = entry.battery.map(|b| battery_color(b.percentage)).unwrap_or(theme.outline);

    m3_card()
        .width(pct!(100.0))
        .align_items(Align::Center)
        .justify_content(JustifyContent::Center)
        .padding(Edges::all(32.0))
        .child(
            Svg::from_string(entry.image_svg)
                .fill_by_id("mouse_led", led_color)
                .opacity_by_id("mouse_led", led_opacity)
                .width(220)
                .height(220)
        )
}

fn battery_card(status: Option<BatteryStatus>) -> View {
    let theme = current_theme();
    let pct = status.map(|s| s.percentage).unwrap_or(0);

    m3_card()
        .width(pct!(100.0))
        .gap(0.0, 14.0)
        .padding(Edges::all(20.0))
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(8.0, 0.0)
                .child(
                    VariableIcon::new(codepoints::BATTERY_FULL).size(20.0).color(battery_color(pct))
                )
                .child(
                    Row::new()
                        .flex_grow(1.0)
                        .justify_content(JustifyContent::SpaceBetween)
                        .align_items(Align::Center)
                        .child(
                            Label::new().label("Pil").font_size(14).color(theme.on_surface_variant)
                        )
                        .child(
                            Label::new()
                                .label(format!("{pct}%"))
                                .font_size(22)
                                .font_weight(FontWeight::SemiBold)
                                .color(theme.on_surface)
                        )
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

fn status_row(icon: char, label: &str, value: String) -> View {
    let theme = current_theme();
    Row::new()
        .width(pct!(100.0))
        .align_items(Align::Center)
        .gap(10.0, 0.0)
        .justify_content(JustifyContent::SpaceBetween)
        .padding(Edges::symmetric(0.0, 7.0))
        .border(Border::bottom(1, theme.outline_variant.with_alpha(120)))
        .child(
            Row::new()
                .align_items(Align::Center)
                .gap(8.0, 0.0)
                .child(VariableIcon::new(icon).size(16.0).color(theme.on_surface_variant))
                .child(Label::new().label(label).font_size(13).color(theme.on_surface_variant))
        )
        .child(
            Label::new()
                .label(value)
                .font_size(13)
                .font_weight(FontWeight::Medium)
                .color(theme.on_surface)
        )
}

fn connection_icon(connection: ConnectionType) -> char {
    match connection {
        ConnectionType::Usb => codepoints::USB,
        ConnectionType::Wireless2_4Ghz => codepoints::SETTINGS_INPUT_ANTENNA,
        ConnectionType::Bluetooth => codepoints::BLUETOOTH,
        ConnectionType::Unknown => codepoints::LINK_OFF,
    }
}

fn connection_label(connection: ConnectionType) -> &'static str {
    match connection {
        ConnectionType::Usb => "USB",
        ConnectionType::Wireless2_4Ghz => "2.4GHz",
        ConnectionType::Bluetooth => "Bluetooth",
        ConnectionType::Unknown => "Bilinmiyor",
    }
}

fn status_card(entry: &DeviceEntry) -> View {
    let theme = current_theme();
    let charging = entry.battery
        .map(|b| matches!(b.state, ChargingState::Charging))
        .unwrap_or(false);

    let (connection_icon, connection_text) = match entry.battery {
        Some(status) if entry.connected =>
            (
                connection_icon(status.connection),
                format!("Bağlı ({})", connection_label(status.connection)),
            ),
        _ => (codepoints::LINK_OFF, "Bağlı değil".to_string()),
    };

    let mut card = m3_card()
        .width(pct!(100.0))
        .padding(Edges::all(20.0))
        .child(
            Label::new()
                .label("Durum")
                .font_size(14)
                .font_weight(FontWeight::SemiBold)
                .color(theme.on_surface)
                .margin(Edges::only(0, 0, 0, 10))
        )
        .child(
            status_row(
                if charging {
                    codepoints::BATTERY_CHARGING_FULL
                } else {
                    codepoints::BATTERY_FULL
                },
                "Durum",
                (if charging { "Şarj oluyor" } else { "Şarjda değil" }).to_string()
            )
        )
        .child(status_row(connection_icon, "Bağlantı", connection_text));

    if let Some(err) = &entry.error {
        card = card.child(
            View::new()
                .width(pct!(100.0))
                .margin(Edges::only(0, 10, 0, 0))
                .padding(Edges::symmetric(10.0, 8.0))
                .background(theme.error_container)
                .border(Border::all(1, theme.error.with_alpha(80)).radius(theme.radius_sm))
                .child(
                    Label::new().label(err.clone()).font_size(12).color(theme.on_error_container)
                )
        );
    }

    card
}

fn polling_rate_card(entry: &DeviceEntry, set_devices: SetState<Vec<DeviceEntry>>) -> View {
    let theme = current_theme();
    let rates = [
        (PollingRate::Hz125, "125 Hz"),
        (PollingRate::Hz500, "500 Hz"),
        (PollingRate::Hz1000, "1000 Hz"),
    ];

    let applying = entry.applying_rate;
    let mut row = Row::new().width(pct!(100.0)).gap(8.0, 0.0);

    for (rate, label) in rates {
        let selected = entry.polling_rate == Some(rate);
        let (bg, fg) = if selected {
            (theme.secondary_container, theme.on_secondary_container)
        } else {
            (theme.surface_container_high, theme.on_surface_variant)
        };
        let entry_id = entry.id;
        let pending_rate = entry.pending_rate.clone();
        let set_devices = set_devices.clone();

        row = row.child(
            Button::new()
                .label(if applying && selected { "..." } else { label })
                .flex_grow(1.0)
                .justify_content(JustifyContent::Center)
                .padding(Edges::symmetric(0.0, 10.0))
                .background(bg)
                .color(fg)
                .border(Border::all(0, Color::TRANSPARENT).radius(theme.radius_lg))
                .cursor(Cursor::Pointer)
                .enabled(!applying)
                .on_click(move |_ctx| {
                    *pending_rate.lock().unwrap() = Some(rate);
                    // Reflected immediately, before the background task
                    // even picks the request up, so the click never feels
                    // like it did nothing.
                    set_devices.update(move |list| {
                        if let Some(entry) = list.iter_mut().find(|e| e.id == entry_id) {
                            entry.applying_rate = true;
                        }
                    });
                })
        );
    }

    let mut card = m3_card()
        .width(pct!(100.0))
        .gap(0.0, 10.0)
        .padding(Edges::all(20.0))
        .child(
            Label::new()
                .label("Polling Rate")
                .font_size(14)
                .font_weight(FontWeight::SemiBold)
                .color(theme.on_surface)
        )
        .child(row);

    if applying {
        card = card.child(
            Label::new().label("Uygulanıyor...").font_size(12).color(theme.on_surface_variant)
        );
    }

    card
}

fn detail_panel(
    entry: Option<&DeviceEntry>,
    breathe_phase: f32,
    set_devices: SetState<Vec<DeviceEntry>>
) -> View {
    let theme = current_theme();

    let Some(entry) = entry else {
        return View::new()
            .flex_grow(1.0)
            .align_items(Align::Center)
            .justify_content(JustifyContent::Center)
            .child(
                Label::new()
                    .label("Soldan bir cihaz seçin veya yeni cihaz ekleyin")
                    .font_size(15)
                    .color(theme.on_surface_variant)
            );
    };

    let mut column = Column::new()
        .width(pct!(100.0))
        .max_width(560)
        .gap(0.0, 20.0)
        .margin(Edges::only(0, 0, 0, 32))
        .child(
            Column::new()
                .gap(0.0, 4.0)
                .child(
                    Label::new()
                        .label(entry.brand.clone())
                        .font_size(14)
                        .color(theme.on_surface_variant)
                )
                .child(
                    Label::new()
                        .label(entry.model.clone())
                        .font_size(24)
                        .font_weight(FontWeight::SemiBold)
                        .color(theme.on_surface)
                )
        )
        .child(device_illustration(entry, breathe_phase))
        .child(battery_card(entry.battery))
        .child(status_card(entry));

    if entry.supports_polling_rate {
        column = column.child(polling_rate_card(entry, set_devices));
    }

    Column::new()
        .flex_grow(1.0)
        .width(pct!(100.0))
        .height(pct!(100.0))
        .overflow_y(Overflow::Auto)
        .padding(Edges::all(28.0))
        .child(column)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig {
        title: "Device Monitor".into(),
        width: 960,
        height: 600,
        position: WindowPosition::Center,
        ..Default::default()
    };

    let mut app = App::new(config);

    app.render(|| {
        let (devices_list, set_devices) = use_state(Vec::<DeviceEntry>::new());
        let (selected_id, set_selected) = use_state(Option::<u64>::None);
        let (picker_open, set_picker_open) = use_state(false);
        let (breathe_phase, set_breathe_phase) = use_state(0.0f32);

        use_effect(move || {
            spawn(async move {
                let mut phase: f32 = 0.0;
                loop {
                    spawn_blocking(|| std::thread::sleep(Duration::from_millis(33))).await;
                    phase = (phase + (std::f32::consts::TAU / 2.0) * 0.033) % std::f32::consts::TAU;
                    set_breathe_phase.set(phase);
                }
            });
        }, ());

        let theme = current_theme();
        let selected_entry = devices_list.iter().find(|e| Some(e.id) == selected_id);

        Box::new(
            Row::new()
                .width(pct!(100.0))
                .height(pct!(100.0))
                .background(theme.background)
                .child(
                    sidebar(
                        &devices_list,
                        selected_id,
                        set_selected.clone(),
                        set_devices.clone(),
                        picker_open,
                        set_picker_open.clone()
                    )
                )
                .child(detail_panel(selected_entry, breathe_phase, set_devices.clone()))
        )
    });

    app.run()?;

    Ok(())
}
