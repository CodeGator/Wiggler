use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::System::SystemInformation::GetLocalTime;

use crate::input::{idle_ms, nudge_mouse, screen_is_locked_or_saver};
use crate::settings::Settings;

#[derive(Clone)]
pub struct Engine {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Clone)]
pub struct Snapshot {
    pub running: bool,
    pub interval_secs: u32,
    pub idle_only: bool,
    pub skip_when_locked: bool,
    pub auto_start: bool,
    pub quiet_after: bool,
    pub quiet_after_minutes: u32,
    pub skip_weekends: bool,
    pub blocked_by_lock: bool,
    pub blocked_by_weekend: bool,
    pub blocked_by_quiet: bool,
    pub last_nudge: Option<Instant>,
    pub next_nudge: Option<Instant>,
    pub flash_until: Option<Instant>,
    pub breath_started: Option<Instant>,
}

struct Inner {
    running: bool,
    interval_secs: u32,
    idle_only: bool,
    skip_when_locked: bool,
    auto_start: bool,
    quiet_after: bool,
    quiet_after_minutes: u32,
    skip_weekends: bool,
    blocked_by_lock: bool,
    blocked_by_weekend: bool,
    blocked_by_quiet: bool,
    last_nudge: Option<Instant>,
    next_nudge: Option<Instant>,
    flash_until: Option<Instant>,
    breath_started: Option<Instant>,
    quit: bool,
}

impl Engine {
    pub fn spawn(settings: Settings) -> Self {
        let mut inner = Inner {
            running: false,
            interval_secs: settings.interval_seconds,
            idle_only: settings.idle_only,
            skip_when_locked: settings.skip_when_locked,
            auto_start: settings.auto_start,
            quiet_after: settings.quiet_after,
            quiet_after_minutes: settings.quiet_after_minutes,
            skip_weekends: settings.skip_weekends,
            blocked_by_lock: false,
            blocked_by_weekend: false,
            blocked_by_quiet: false,
            last_nudge: None,
            next_nudge: None,
            flash_until: None,
            breath_started: None,
            quit: false,
        };
        if settings.running {
            inner.set_running(true);
        }
        let inner = Arc::new(Mutex::new(inner));
        let thread_inner = Arc::clone(&inner);
        let _ = thread::Builder::new()
            .name("wiggler-engine".into())
            .spawn(move || {
                loop {
                    {
                        let mut guard = thread_inner.lock().expect("engine lock");
                        if guard.quit {
                            break;
                        }
                        guard.tick();
                    }
                    thread::sleep(Duration::from_millis(200));
                }
            });
        Self { inner }
    }

    pub fn snapshot(&self) -> Snapshot {
        let guard = self.inner.lock().expect("engine lock");
        Snapshot {
            running: guard.running,
            interval_secs: guard.interval_secs,
            idle_only: guard.idle_only,
            skip_when_locked: guard.skip_when_locked,
            auto_start: guard.auto_start,
            quiet_after: guard.quiet_after,
            quiet_after_minutes: guard.quiet_after_minutes,
            skip_weekends: guard.skip_weekends,
            blocked_by_lock: guard.blocked_by_lock,
            blocked_by_weekend: guard.blocked_by_weekend,
            blocked_by_quiet: guard.blocked_by_quiet,
            last_nudge: guard.last_nudge,
            next_nudge: guard.next_nudge,
            flash_until: guard.flash_until,
            breath_started: guard.breath_started,
        }
    }

    pub fn settings(&self) -> Settings {
        let guard = self.inner.lock().expect("engine lock");
        Settings {
            interval_seconds: guard.interval_secs,
            idle_only: guard.idle_only,
            skip_when_locked: guard.skip_when_locked,
            running: guard.running,
            auto_start: guard.auto_start,
            quiet_after: guard.quiet_after,
            quiet_after_minutes: guard.quiet_after_minutes,
            skip_weekends: guard.skip_weekends,
        }
    }

    pub fn toggle(&self) {
        let mut guard = self.inner.lock().expect("engine lock");
        let running = !guard.running;
        guard.set_running(running);
    }

    pub fn set_interval(&self, seconds: u32) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.interval_secs = seconds;
        if guard.running {
            guard.schedule_next();
        }
    }

    pub fn set_idle_only(&self, idle_only: bool) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.idle_only = idle_only;
        if guard.running {
            guard.schedule_next();
        }
    }

    pub fn set_skip_when_locked(&self, skip_when_locked: bool) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.skip_when_locked = skip_when_locked;
    }

    pub fn set_auto_start(&self, auto_start: bool) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.auto_start = auto_start;
    }

    pub fn set_quiet_after(&self, quiet_after: bool) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.quiet_after = quiet_after;
    }

    pub fn set_quiet_after_minutes(&self, minutes: u32) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.quiet_after_minutes = minutes;
    }

    pub fn set_skip_weekends(&self, skip_weekends: bool) {
        let mut guard = self.inner.lock().expect("engine lock");
        guard.skip_weekends = skip_weekends;
    }

    pub fn quit(&self) {
        self.inner.lock().expect("engine lock").quit = true;
    }
}

impl Inner {
    fn set_running(&mut self, running: bool) {
        self.running = running;
        if running {
            self.breath_started = Some(Instant::now());
            self.schedule_next();
        } else {
            self.next_nudge = None;
            self.breath_started = None;
        }
    }

    fn schedule_next(&mut self) {
        self.next_nudge = Some(Instant::now() + Duration::from_secs(self.interval_secs as u64));
    }

    fn tick(&mut self) {
        let local = unsafe { GetLocalTime() };
        let minutes = u32::from(local.wHour) * 60 + u32::from(local.wMinute);
        self.blocked_by_weekend =
            self.skip_weekends && (local.wDayOfWeek == 0 || local.wDayOfWeek == 6);
        self.blocked_by_quiet = self.quiet_after && minutes >= self.quiet_after_minutes;
        self.blocked_by_lock = self.skip_when_locked && screen_is_locked_or_saver();
        if !self.running {
            return;
        }
        if self.blocked_by_weekend || self.blocked_by_quiet || self.blocked_by_lock {
            return;
        }
        let now = Instant::now();
        if self
            .last_nudge
            .is_some_and(|last| now.duration_since(last) < Duration::from_secs(1))
        {
            return;
        }
        if self.idle_only {
            if idle_ms() < u64::from(self.interval_secs) * 1000 {
                return;
            }
        } else if self.next_nudge.is_none_or(|at| now < at) {
            return;
        }
        if nudge_mouse().is_err() {
            return;
        }
        self.last_nudge = Some(now);
        self.flash_until = Some(now + Duration::from_millis(120));
        self.schedule_next();
    }
}
