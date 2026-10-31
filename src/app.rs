use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui::viewport::SystemTheme;
use eframe::egui::{
    self, Align, Align2, CentralPanel, Color32, ColorImage, Context, CornerRadius, FontData,
    FontDefinitions, FontFamily, FontId, Frame, Label, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, Style, TextureHandle, TextureOptions, Vec2, ViewportBuilder, ViewportCommand, Visuals,
};
use tray_icon::menu::{
    ContextMenu, Menu, MenuEvent, MenuItem, MenuTheme, PredefinedMenuItem, Submenu,
};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

thread_local! {
    static TRAY_POPUP: RefCell<Option<(Menu, isize)>> = RefCell::new(None);
    static MAIN_HWND: Cell<isize> = const { Cell::new(0) };
}

static WANT_SHOW: AtomicBool = AtomicBool::new(false);
static WANT_ABOUT: AtomicBool = AtomicBool::new(false);
static WANT_QUIT: AtomicBool = AtomicBool::new(false);

use crate::engine::{Engine, Snapshot};
use crate::icon::{tray_icon, window_icon};
use crate::input::idle_ms;
use crate::settings::{
    INTERVAL_MAX, INTERVAL_MIN, INTERVAL_STEP, TIME_MAX, TIME_MIN, TIME_STEP, format_clock,
    snap_interval, snap_time,
};
use crate::theme::{
    AMBER, AMBER_DIM, BG, BUTTON_TEXT, GREEN, GREEN_DIM, GREEN_HOVER, LINE, LINE_HOVER, MUTED,
    PAPER, SURFACE, TEXT, WINDOW_H, WINDOW_W,
};

pub fn run(start_hidden: bool) -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Wiggler")
            .with_inner_size([WINDOW_W, WINDOW_H])
            .with_min_inner_size([WINDOW_W, WINDOW_H])
            .with_max_inner_size([WINDOW_W, WINDOW_H])
            .with_resizable(false)
            .with_minimize_button(false)
            .with_maximize_button(false)
            .with_visible(!start_hidden)
            .with_icon(window_icon()),
        centered: true,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Wiggler",
        options,
        Box::new(move |cc| Ok(Box::new(WigglerApp::new(cc, start_hidden)))),
    )?;
    Ok(())
}

struct WigglerApp {
    engine: Engine,
    tray: TrayIcon,
    quit_item: MenuItem,
    exit_item: MenuItem,
    about_item: MenuItem,
    menubar: Menu,
    menubar_attached: bool,
    hidden: bool,
    hide_settle: u8,
    really_quit: bool,
    tray_running: bool,
    show_options: bool,
    show_about: bool,
    gator: TextureHandle,
    tray_events: Receiver<TrayIconEvent>,
    menu_events: Receiver<MenuEvent>,
}

impl WigglerApp {
    fn new(cc: &eframe::CreationContext<'_>, start_hidden: bool) -> Self {
        setup_style(&cc.egui_ctx);
        cc.egui_ctx
            .send_viewport_cmd(ViewportCommand::SetTheme(SystemTheme::Dark));
        if start_hidden {
            cc.egui_ctx
                .send_viewport_cmd(ViewportCommand::Visible(false));
        }
        let (tray_tx, tray_events) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                popup_tray_menu();
            }
            let _ = tray_tx.send(event);
            ctx.request_repaint();
        }));
        let (menu_tx, menu_events) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id = event.id.as_ref();
            if id == "show" {
                WANT_SHOW.store(true, Ordering::SeqCst);
                request_show(&ctx);
            } else if id == "about" {
                WANT_ABOUT.store(true, Ordering::SeqCst);
                WANT_SHOW.store(true, Ordering::SeqCst);
                request_show(&ctx);
            } else if id == "quit" || id == "exit" {
                WANT_QUIT.store(true, Ordering::SeqCst);
                request_show(&ctx);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            let _ = menu_tx.send(event);
            ctx.request_repaint();
        }));

        let settings = crate::settings::Settings::load();
        if settings.auto_start {
            let _ = crate::autostart::set_enabled(true);
        }
        let engine = Engine::spawn(settings);
        let running = engine.snapshot().running;

        let show_item = MenuItem::with_id("show", "Show", true, None);
        let quit_item = MenuItem::with_id("quit", "Quit", true, None);
        let tray_about = MenuItem::with_id("about", "About...", true, None);
        let tray_separator = PredefinedMenuItem::separator();
        let tray_menu = Menu::with_items(&[
            &show_item,
            &tray_about,
            &tray_separator,
            &quit_item,
        ])
        .expect("tray menu");
        let exit_item = MenuItem::with_id("exit", "&Quit", true, None);
        let file = Submenu::with_items("&File", true, &[&exit_item]).expect("file menu");
        let about_item = MenuItem::with_id("about", "About...", true, None);
        let help = Submenu::with_items("&Help", true, &[&about_item]).expect("help menu");
        let menubar = Menu::with_items(&[&file, &help]).expect("menubar");
        let tray = TrayIconBuilder::new()
            .with_tooltip(if running {
                "Wiggler — Still here"
            } else {
                "Wiggler"
            })
            .with_icon(tray_icon(running))
            .build()
            .expect("tray icon");
        let tray_hwnd = tray.window_handle() as isize;
        TRAY_POPUP.with(|slot| {
            *slot.borrow_mut() = Some((tray_menu, tray_hwnd));
        });

        Self {
            engine,
            tray,
            quit_item,
            exit_item,
            about_item,
            menubar,
            menubar_attached: false,
            hidden: start_hidden,
            hide_settle: if start_hidden { 8 } else { 0 },
            really_quit: false,
            tray_running: running,
            show_options: false,
            show_about: false,
            gator: load_gator(&cc.egui_ctx),
            tray_events,
            menu_events,
        }
    }

    fn hide_to_tray(&mut self, ctx: &Context) {
        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        self.hidden = true;
    }

    fn show_from_tray(&mut self, ctx: &Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        self.hidden = false;
        if let Some(hwnd) = app_hwnd() {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
                );
                let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::SW_RESTORE,
                );
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
            }
        }
    }

    fn quit_app(&mut self, ctx: &Context) {
        self.really_quit = true;
        self.engine.quit();
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    fn keep_hidden(&mut self, ctx: &Context) {
        if !self.hidden {
            return;
        }
        hide_native_window();
        if self.hide_settle > 0 {
            self.hide_settle -= 1;
            ctx.request_repaint();
        }
    }

    fn handle_shell_events(&mut self, ctx: &Context) {
        if WANT_QUIT.swap(false, Ordering::SeqCst) {
            self.quit_app(ctx);
            return;
        }
        if WANT_SHOW.swap(false, Ordering::SeqCst) {
            self.show_from_tray(ctx);
        }
        if WANT_ABOUT.swap(false, Ordering::SeqCst) {
            self.show_from_tray(ctx);
            self.show_about = true;
        }

        while let Ok(_event) = self.tray_events.try_recv() {}

        while let Ok(event) = self.menu_events.try_recv() {
            if event.id.as_ref() == "show" {
                self.show_from_tray(ctx);
            } else if event.id == self.quit_item.id() || event.id == self.exit_item.id() {
                self.quit_app(ctx);
            } else if event.id == self.about_item.id() || event.id.as_ref() == "about" {
                self.show_from_tray(ctx);
                self.show_about = true;
            }
        }

        if self.really_quit {
            return;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.hidden {
                self.quit_app(ctx);
            } else {
                self.hide_to_tray(ctx);
            }
        } else if !self.hidden && ctx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            self.hide_to_tray(ctx);
        }
    }

    fn sync_tray(&mut self, snapshot: &Snapshot) {
        if self.tray_running == snapshot.running {
            return;
        }
        self.tray_running = snapshot.running;
        let _ = self.tray.set_icon(Some(tray_icon(snapshot.running)));
        let tooltip = if snapshot.running {
            "Wiggler — Still here"
        } else {
            "Wiggler"
        };
        let _ = self.tray.set_tooltip(Some(tooltip));
    }

    fn attach_menubar(&mut self) {
        if self.menubar_attached {
            return;
        }
        let Some(hwnd) = app_hwnd() else {
            return;
        };
        MAIN_HWND.set(hwnd.0 as isize);
        if unsafe { self.menubar.init_for_hwnd_with_theme(hwnd.0 as isize, MenuTheme::Dark) }.is_ok()
        {
            self.menubar_attached = true;
        }
    }
}

fn popup_tray_menu() {
    TRAY_POPUP.with(|slot| {
        let slot = slot.borrow();
        let Some((menu, hwnd)) = slot.as_ref() else {
            return;
        };
        unsafe {
            let _ = menu.show_context_menu_for_hwnd(*hwnd, None);
        }
    });
}

fn request_show(ctx: &Context) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    restore_main_window();
}

fn restore_main_window() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GA_ROOT, GetAncestor, GetForegroundWindow, GetWindowThreadProcessId,
        HWND_TOP, SW_RESTORE, SW_SHOW, SW_SHOWNORMAL, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        SetForegroundWindow, SetWindowPos, ShowWindow,
    };

    let hwnd = {
        let stored = MAIN_HWND.get();
        if stored != 0 {
            HWND(stored as *mut _)
        } else {
            match app_hwnd() {
                Some(hwnd) => hwnd,
                None => return,
            }
        }
    };
    let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
    let target = if root.0.is_null() { hwnd } else { root };
    unsafe {
        let foreground = GetForegroundWindow();
        let mut foreground_pid = 0_u32;
        let foreground_tid = GetWindowThreadProcessId(foreground, Some(&mut foreground_pid));
        let current_tid = GetCurrentThreadId();
        let attached = foreground_tid != 0
            && foreground_tid != current_tid
            && AttachThreadInput(foreground_tid, current_tid, true).as_bool();
        let _ = ShowWindow(target, SW_SHOW);
        let _ = ShowWindow(target, SW_SHOWNORMAL);
        let _ = ShowWindow(target, SW_RESTORE);
        let _ = SetWindowPos(
            target,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        let _ = SetForegroundWindow(target);
        let _ = BringWindowToTop(target);
        if attached {
            let _ = AttachThreadInput(foreground_tid, current_tid, false);
        }
    }
}

fn hide_native_window() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GA_ROOT, GetAncestor, SW_HIDE, ShowWindow,
    };

    let hwnd = {
        let stored = MAIN_HWND.get();
        if stored != 0 {
            HWND(stored as *mut _)
        } else {
            match app_hwnd() {
                Some(hwnd) => hwnd,
                None => return,
            }
        }
    };
    let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
    let target = if root.0.is_null() { hwnd } else { root };
    unsafe {
        let _ = ShowWindow(target, SW_HIDE);
    }
}

fn app_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    let hwnd = unsafe {
        windows::Win32::UI::WindowsAndMessaging::FindWindowW(None, windows::core::w!("Wiggler"))
    }
    .ok()?;
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd)
    }
}

impl eframe::App for WigglerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_shell_events(ctx);
        self.keep_hidden(ctx);
        self.attach_menubar();
        let snapshot = self.engine.snapshot();
        self.sync_tray(&snapshot);

        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin {
                        left: 28,
                        right: 28,
                        top: 24,
                        bottom: 32,
                    }),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                draw_ui(ui, &self.engine, &snapshot, &mut self.show_options);
            });

        draw_options(ctx, &self.engine, &snapshot, &mut self.show_options);
        draw_about(ctx, &self.gator, &mut self.show_about);

        let delay = if self.hidden {
            Duration::from_millis(250)
        } else if snapshot.running {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(250)
        };
        ctx.request_repaint_after(delay);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        TRAY_POPUP.with(|slot| slot.borrow_mut().take());
        self.engine.settings().save();
        self.engine.quit();
    }
}

fn draw_ui(ui: &mut egui::Ui, engine: &Engine, snapshot: &Snapshot, show_options: &mut bool) {
    let now = Instant::now();
    let (status, meta) = status_copy(snapshot, now);

    ui.vertical_centered(|ui| {
        ui.add(Label::new(
            RichText::new("Wiggler")
                .font(FontId::new(11.0, semibold()))
                .color(MUTED)
                .extra_letter_spacing(2.0),
        ));
        ui.add_space(8.0);
        draw_orb(ui, snapshot, now);
        ui.add_space(8.0);
        ui.add(Label::new(
            RichText::new(status)
                .font(FontId::new(22.0, semibold()))
                .color(TEXT),
        ));
        ui.add(Label::new(RichText::new(meta).size(12.0).color(MUTED)));
    });

    ui.add_space(8.0);
    if primary_button(ui, snapshot.running, "Stop wiggling", "Start wiggling").clicked() {
        engine.toggle();
        engine.settings().save();
    }
    ui.add_space(8.0);
    if outline_button(ui, "Options").clicked() {
        *show_options = true;
    }
}

fn draw_about(ctx: &Context, gator: &TextureHandle, open: &mut bool) {
    let mut window = egui::Window::new("About")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .default_width(308.0)
        .frame(
            Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0_f32, LINE))
                .inner_margin(Margin {
                    left: 18,
                    right: 18,
                    top: 12,
                    bottom: 16,
                })
                .corner_radius(8.0),
        );
    window = window.open(open);
    window.show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            let image_size = {
                let native = gator.size_vec2();
                let scale = if native.x > 0.0 { 120.0 / native.x } else { 1.0 };
                native * scale
            };
            let well_size = image_size + Vec2::splat(16.0);
            ui.allocate_ui(well_size, |ui| {
                ui.set_min_size(well_size);
                ui.set_max_size(well_size);
                Frame::new()
                    .fill(PAPER)
                    .corner_radius(12.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Image::new((gator.id(), image_size))
                                .fit_to_exact_size(image_size)
                                .bg_fill(PAPER),
                        );
                    });
            });
            ui.add_space(8.0);
            ui.add(Label::new(
                RichText::new("Wiggler")
                    .font(FontId::new(11.0, semibold()))
                    .color(MUTED)
                    .extra_letter_spacing(2.0),
            ));
            ui.add(Label::new(
                RichText::new(env!("CARGO_PKG_VERSION"))
                    .size(11.0)
                    .color(MUTED),
            ));
            ui.add_space(8.0);
            ui.add(
                Label::new(
                    RichText::new(copyright_line())
                        .size(11.0)
                        .color(MUTED),
                )
                .wrap(),
            );
        });
    });
}

fn draw_options(ctx: &Context, engine: &Engine, snapshot: &Snapshot, open: &mut bool) {
    let mut window = egui::Window::new("Options")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .default_width(308.0)
        .frame(
            Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0_f32, LINE))
                .inner_margin(Margin {
                    left: 18,
                    right: 18,
                    top: 12,
                    bottom: 16,
                })
                .corner_radius(8.0),
        );
    window = window.open(open);
    window.show(ctx, |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

        ui.horizontal(|ui| {
            ui.add(Label::new(
                RichText::new("Nudge every")
                    .font(FontId::new(11.0, semibold()))
                    .color(MUTED)
                    .extra_letter_spacing(0.5),
            ));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(Label::new(
                    RichText::new(format_interval(snapshot.interval_secs))
                        .size(12.0)
                        .color(TEXT),
                ));
            });
        });
        let mut interval = snapshot.interval_secs;
        if stepped_slider(
            ui,
            &mut interval,
            INTERVAL_MIN,
            INTERVAL_MAX,
            INTERVAL_STEP,
            snapshot.running,
        ) {
            engine.set_interval(snap_interval(interval));
            engine.settings().save();
        }

        ui.add_space(8.0);
        let mut idle_only = snapshot.idle_only;
        if option_check(
            ui,
            &mut idle_only,
            "Only when I go idle",
            "Skip nudges while you are using the mouse or keyboard.",
        ) {
            engine.set_idle_only(idle_only);
            engine.settings().save();
        }

        ui.add_space(4.0);
        let mut skip_when_locked = snapshot.skip_when_locked;
        if option_check(
            ui,
            &mut skip_when_locked,
            "Skip when the screen is locked",
            "Do not nudge if the screen is locked or a screensaver is showing.",
        ) {
            engine.set_skip_when_locked(skip_when_locked);
            engine.settings().save();
        }

        ui.add_space(4.0);
        let mut quiet_after = snapshot.quiet_after;
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::Checkbox::new(
                    &mut quiet_after,
                    RichText::new("No nudges after").size(12.0).color(TEXT),
                )
                .indeterminate(false),
            );
            if quiet_after {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(Label::new(
                        RichText::new(format_clock(snapshot.quiet_after_minutes))
                            .size(12.0)
                            .color(TEXT),
                    ));
                });
            }
            if response.changed() {
                engine.set_quiet_after(quiet_after);
                engine.settings().save();
            }
        });
        ui.add(
            Label::new(
                RichText::new("Pause from this local time until morning.")
                    .size(11.0)
                    .color(MUTED),
            )
            .wrap(),
        );
        if quiet_after {
            let mut quiet_minutes = snapshot.quiet_after_minutes;
            if stepped_slider(
                ui,
                &mut quiet_minutes,
                TIME_MIN,
                TIME_MAX,
                TIME_STEP,
                snapshot.running,
            ) {
                engine.set_quiet_after_minutes(snap_time(quiet_minutes));
                engine.settings().save();
            }
        }

        ui.add_space(4.0);
        let mut skip_weekends = snapshot.skip_weekends;
        if option_check(
            ui,
            &mut skip_weekends,
            "Skip weekends",
            "Do not nudge on Saturday or Sunday.",
        ) {
            engine.set_skip_weekends(skip_weekends);
            engine.settings().save();
        }

        ui.add_space(4.0);
        let mut auto_start = snapshot.auto_start;
        if option_check(
            ui,
            &mut auto_start,
            "Start at sign-in",
            "Open Wiggler when you sign in to Windows.",
        ) {
            if crate::autostart::set_enabled(auto_start).is_ok() {
                engine.set_auto_start(auto_start);
                engine.settings().save();
            }
        }
    });
}

fn option_check(ui: &mut egui::Ui, value: &mut bool, label: &str, helper: &str) -> bool {
    let response = ui.add(
        egui::Checkbox::new(value, RichText::new(label).size(12.0).color(TEXT)).indeterminate(false),
    );
    ui.add(
        Label::new(RichText::new(helper).size(11.0).color(MUTED)).wrap(),
    );
    response.changed()
}

fn status_copy(snapshot: &Snapshot, now: Instant) -> (&'static str, String) {
    if !snapshot.running {
        let meta = match snapshot.last_nudge {
            None => "Has not nudged yet".to_string(),
            Some(at) => format!(
                "Last nudge {} ago",
                format_countdown(now.duration_since(at).as_secs() as u32)
            ),
        };
        return ("Away is allowed", meta);
    }

    if snapshot
        .last_nudge
        .is_some_and(|at| now.duration_since(at) < Duration::from_millis(1500))
    {
        return ("Still here", "Nudged just now".to_string());
    }

    if snapshot.blocked_by_weekend {
        return (
            "Weekend",
            "Nudges pause until Monday".to_string(),
        );
    }

    if snapshot.blocked_by_quiet {
        return (
            "Off the clock",
            "Nudges pause until morning".to_string(),
        );
    }

    if snapshot.blocked_by_lock {
        return (
            "Screen is locked",
            "Nudges pause until you unlock".to_string(),
        );
    }

    if snapshot.idle_only {
        let idle_seconds = idle_ms() as f32 / 1000.0;
        if idle_seconds < 1.0 {
            return (
                "Waiting for idle",
                "Will nudge after you sit still".to_string(),
            );
        }
        let remaining = snapshot.interval_secs.saturating_sub(idle_seconds as u32);
        return (
            "Still here",
            format!("Next nudge in {}", format_countdown(remaining)),
        );
    }

    let remaining = snapshot
        .next_nudge
        .map(|at| at.saturating_duration_since(now).as_secs() as u32)
        .unwrap_or(0);
    (
        "Still here",
        format!("Next nudge in {}", format_countdown(remaining)),
    )
}

fn draw_orb(ui: &mut egui::Ui, snapshot: &Snapshot, now: Instant) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(120.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let active = snapshot.running;
    let flashing = snapshot.flash_until.is_some_and(|until| now < until);
    let fill = if flashing {
        TEXT
    } else if active {
        GREEN
    } else {
        AMBER
    };
    let dim = if active { GREEN_DIM } else { AMBER_DIM };
    let mut ring = 88.0;
    if active {
        if let Some(started) = snapshot.breath_started {
            let t = now.duration_since(started).as_secs_f32() / 2.4;
            ring = 90.0 + 6.0 * (t * std::f32::consts::TAU).sin();
        }
    }
    let glow = Color32::from_rgba_unmultiplied(dim.r(), dim.g(), dim.b(), 10);
    for i in (1..=8).rev() {
        painter.circle_stroke(
            center,
            ring / 2.0 + i as f32 * 3.0,
            Stroke::new(1.0_f32, glow),
        );
    }
    painter.circle_stroke(center, ring / 2.0, Stroke::new(2.0_f32, dim));
    painter.circle_filled(center, 36.0, fill);
}

fn primary_button(ui: &mut egui::Ui, running: bool, on_label: &str, off_label: &str) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
    let rounding = CornerRadius::same(8);
    let hovered = response.hovered();
    if running {
        let fill = if hovered { GREEN_HOVER } else { GREEN };
        ui.painter().rect_filled(rect, rounding, fill);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            on_label,
            FontId::new(13.0, semibold()),
            BUTTON_TEXT,
        );
    } else {
        paint_outline_button(ui, rect, rounding, hovered, off_label);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn outline_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
    paint_outline_button(ui, rect, CornerRadius::same(8), response.hovered(), label);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn paint_outline_button(
    ui: &egui::Ui,
    rect: Rect,
    rounding: CornerRadius,
    hovered: bool,
    label: &str,
) {
    let fill = if hovered { LINE_HOVER } else { SURFACE };
    ui.painter().rect_filled(rect, rounding, fill);
    ui.painter().rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, LINE),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(13.0, semibold()),
        TEXT,
    );
}

fn stepped_slider(
    ui: &mut egui::Ui,
    value: &mut u32,
    min: u32,
    max: u32,
    step: u32,
    running: bool,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::click_and_drag());
    let old = *value;
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let raw = min as f32 + t * (max - min) as f32;
            let clamped = (raw.round() as u32).clamp(min, max);
            *value = min + ((clamped - min) / step) * step;
        }
    }

    let t = (*value - min) as f32 / (max - min) as f32;
    let y = rect.center().y;
    let track = Rect::from_min_max(
        Pos2::new(rect.left(), y - 2.0),
        Pos2::new(rect.right(), y + 2.0),
    );
    let fill_x = rect.left() + rect.width() * t;
    let fill = Rect::from_min_max(track.min, Pos2::new(fill_x, track.max.y));
    let accent = if running { GREEN } else { AMBER };
    ui.painter()
        .rect_filled(track, CornerRadius::same(2), LINE);
    ui.painter()
        .rect_filled(fill, CornerRadius::same(2), accent);
    ui.painter()
        .circle_filled(Pos2::new(fill_x, y), 7.0, TEXT);
    *value != old
}

fn format_interval(seconds: u32) -> String {
    if seconds < 60 {
        format!("{seconds} sec")
    } else if seconds.is_multiple_of(60) {
        format!("{} min", seconds / 60)
    } else {
        format!("{:.1} min", seconds as f32 / 60.0)
    }
}

fn format_countdown(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

fn copyright_line() -> String {
    let year = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }.wYear;
    format!("Copyright (c) 2002 - {year} CodeGator. All rights reserved.")
}

fn load_gator(ctx: &Context) -> TextureHandle {
    const PNG: &[u8] = include_bytes!("../assets/codegator.png");
    let image = image::load_from_memory(PNG)
        .expect("codegator.png")
        .into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    ctx.load_texture("codegator", color, TextureOptions::LINEAR)
}

fn setup_style(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    if let Ok(regular) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe".to_owned(), FontData::from_owned(regular).into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".to_owned());
        fonts
            .families
            .entry(FontFamily::Name("semibold".into()))
            .or_default()
            .insert(0, "segoe".to_owned());
    }
    if let Ok(semibold_font) = std::fs::read(r"C:\Windows\Fonts\segoeuisb.ttf") {
        fonts.font_data.insert(
            "segoe_semibold".to_owned(),
            FontData::from_owned(semibold_font).into(),
        );
        fonts.families.insert(
            FontFamily::Name("semibold".into()),
            vec!["segoe_semibold".to_owned()],
        );
    }
    ctx.set_fonts(fonts);

    let mut style = Style {
        visuals: Visuals::dark(),
        ..Default::default()
    };
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = SURFACE;
    style.visuals.window_stroke = Stroke::new(1.0_f32, LINE);
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.widgets.inactive.bg_fill = SURFACE;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    style.visuals.widgets.hovered.bg_fill = LINE_HOVER;
    style.visuals.widgets.active.bg_fill = GREEN_DIM;
    style.visuals.selection.bg_fill = GREEN;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, BUTTON_TEXT);
    style.visuals.hyperlink_color = GREEN;
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.window_margin = Margin::ZERO;
    ctx.set_style(style);
}
