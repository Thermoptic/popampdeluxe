//! X11: window types through winit, shapes, struts and work areas through
//! a connection of our own. Under Wayland (see `wayland.rs`) none of that
//! exists, and the calls for it do nothing.

use super::wayland::{self, Wayland};
use super::{shape_rects, Mask, Surface, Win, WindowRole};
use crate::bar::{Screen, Strip};
use popamp_core::config::BarEdge;
use popamp_skin::cursors::CursorImage;
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::platform::wayland::ActiveEventLoopExtWayland;
use winit::platform::x11::{WindowAttributesExtX11, WindowType};
use winit::window::{Window, WindowAttributes};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::shape::{self, ConnectionExt as _, SK, SO};
use x11rb::protocol::xproto::{AtomEnum, ClipOrdering, ConfigureWindowAux, ConnectionExt as _, PropMode, Rectangle, StackMode};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// Fixed-size window attributes for `role`, borderless but for dialogs.
pub fn attributes(role: WindowRole) -> WindowAttributes {
    let attrs = Window::default_attributes().with_decorations(false).with_resizable(false).with_name("popamp", "Popamp");
    match role {
        WindowRole::Main => attrs,
        WindowRole::Docked => attrs.with_x11_window_type(vec![WindowType::Utility]),
        WindowRole::Bar => attrs.with_x11_window_type(vec![WindowType::Dock]),
        WindowRole::Popup => attrs.with_override_redirect(true).with_x11_window_type(vec![WindowType::PopupMenu]),
        WindowRole::Tooltip => attrs.with_override_redirect(true).with_active(false).with_x11_window_type(vec![WindowType::Tooltip]),
        WindowRole::Dialog => attrs.with_decorations(true).with_x11_window_type(vec![WindowType::Dialog]),
    }
}

/// Picks the window system for the event loop: Wayland where the desktop
/// is one and either the user asked for it (`wanted`), X11 programs
/// cannot run there, or its XWayland does not let them place their
/// windows (COSMIC); X11 otherwise, where Popamp can place its windows.
pub fn choose_window_system<T>(builder: &mut winit::event_loop::EventLoopBuilder<T>, wanted: bool) {
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::platform::x11::EventLoopBuilderExtX11 as _;
    let has = |var: &str| std::env::var_os(var).is_some_and(|v| !v.is_empty());
    // cosmic-comp keeps a mapped X11 window where it is, whatever it asks
    // for, so under XWayland the windows could not be dragged.
    let cosmic = std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.split(':').any(|d| d.eq_ignore_ascii_case("COSMIC")));
    if has("WAYLAND_DISPLAY") && (wanted || cosmic || !has("DISPLAY")) {
        builder.with_wayland();
    } else {
        builder.with_x11();
    }
}

/// Window managers differ in whether they minimize transient windows with
/// their owner, so every window is minimized and restored by itself.
pub const OWNED_FOLLOW_OWNER: bool = false;

/// Each window is minimized as the owner rule above says.
pub const MINIMIZE_HIDES_COMPANIONS: bool = false;

/// A connection to the X server for what winit does not cover.
pub struct Platform {
    x11: Option<RustConnection>,
    /// The windows are Wayland's, not X11's.
    on_wayland: bool,
    /// There from the main window's making on.
    wayland: Option<Wayland>,
    /// The Wayland compositor makes bars for programs.
    layer_shell: bool,
    /// winit's window the Wayland connection was joined with, kept while
    /// the bar stands in for it.
    host: Option<Arc<Window>>,
}

impl Platform {
    pub fn connect() -> Platform {
        Platform { x11: None, on_wayland: false, wayland: None, layer_shell: false, host: None }
    }

    /// Finds which window system the event loop is on.
    pub fn start(&mut self, el: &ActiveEventLoop) {
        self.on_wayland = el.is_wayland();
        self.layer_shell = self.on_wayland && wayland::has_layer_shell(el);
        if !self.on_wayland && self.x11.is_none() {
            self.x11 = x11rb::connect(None).map(|(c, _)| c).map_err(|e| log::warn!("no X11 connection for window shapes: {e}")).ok();
        }
    }

    pub fn on_wayland(&self) -> bool {
        self.on_wayland
    }

    /// Whether the main window can be a bar: always but as a Wayland
    /// program, where the compositor makes it.
    pub fn can_bar(&self) -> bool {
        !self.on_wayland || self.layer_shell
    }

    /// Whether a Wayland main window has been made.
    pub fn joined(&self) -> bool {
        self.wayland.is_some()
    }

    /// What to draw `window` into. Under Wayland it is the main window,
    /// at `pos`, which the others are then placed against.
    pub fn surface(&mut self, window: &Arc<Window>, pos: (i32, i32)) -> Result<Surface, String> {
        if let Some(wayland) = &mut self.wayland {
            wayland.rejoin(window, pos)?;
            self.host = Some(window.clone());
            return Ok(Surface::Shm(Box::new(wayland.main_surface())));
        }
        self.host = Some(window.clone());
        match Wayland::join(window, pos) {
            Some(joined) => {
                let wayland = joined?;
                let surface = wayland.main_surface();
                self.wayland = Some(wayland);
                Ok(Surface::Shm(Box::new(surface)))
            }
            None => Surface::soft(window),
        }
    }

    /// Under Wayland, a window at `pos` that is a part of the main one.
    /// Without `input` the mouse goes through it.
    pub fn child(&self, pos: (i32, i32), input: bool) -> Option<(Win, Surface)> {
        let (sub, shm) = self.wayland.as_ref()?.sub(pos, input);
        Some((Win::Sub(Box::new(sub)), Surface::Shm(Box::new(shm))))
    }

    /// Under Wayland, the bar at `edge`, `size` screen pixels big at `pos`,
    /// for the main window made before, which goes off the screen. The
    /// windows made by `child` from now on are parts of the bar.
    pub fn bar(&mut self, edge: BarEdge, pos: (i32, i32), size: (i32, i32)) -> Option<(Win, Surface)> {
        let (sub, shm) = self.wayland.as_mut().filter(|w| w.has_bar())?.bar(edge, pos, size)?;
        Some((Win::Sub(Box::new(sub)), Surface::Shm(Box::new(shm))))
    }

    /// The width of the bar made by `bar`, once the compositor has given it.
    pub fn bar_width(&self) -> Option<i32> {
        self.wayland.as_ref()?.bar_width()
    }

    /// Under Wayland, the monitors placed against the main window, where
    /// the compositor tells where it is.
    pub fn wayland_monitors(&self) -> Vec<Screen> {
        self.wayland.as_ref().map(Wayland::monitors).unwrap_or_default()
    }

    /// Under Wayland, has the compositor move the main window as a button
    /// pressed over one of its parts is held; false where it cannot.
    pub fn drag_main(&self) -> bool {
        self.wayland.as_ref().is_some_and(Wayland::drag_main)
    }

    /// What happened to the windows made by `child`.
    pub fn events(&mut self) -> Vec<wayland::Event> {
        self.wayland.as_mut().map(Wayland::events).unwrap_or_default()
    }

    /// The screen pixels in one of the Wayland compositor's units.
    pub fn set_scale(&self, scale: f64) {
        if let Some(wayland) = &self.wayland {
            wayland.set_scale(scale);
        }
    }

    /// The mouse's picture over the windows made by `child`: a skin's
    /// cursor or the desktop's arrow.
    pub fn set_cursor(&mut self, cursor: Option<&CursorImage>) {
        if let Some(wayland) = &mut self.wayland {
            wayland.set_cursor(cursor);
        }
    }

    /// Cuts `window` to the `region.txt` polygons (`None` restores the
    /// rectangle). `size` is in skin pixels, drawn at `scale`.
    /// Shows only the pixels `mask` covers of a window `size` screen pixels
    /// big; all of it without a mask.
    pub fn set_shape(&self, window: &Window, mask: Option<&Mask>, size: (i32, i32)) -> Result<(), String> {
        let (Some(conn), Some(xid)) = (&self.x11, x11_window_id(window)) else { return Ok(()) };
        let err = |e: &dyn std::fmt::Display| e.to_string();
        if conn.extension_information(shape::X11_EXTENSION_NAME).map_err(|e| err(&e))?.is_none() {
            return Err("X server has no SHAPE extension".into());
        }
        let rects: Vec<Rectangle> =
            shape_rects(mask, size).into_iter().map(|(x, y, width, height)| Rectangle { x, y, width, height }).collect();
        conn.shape_rectangles(SO::SET, SK::BOUNDING, ClipOrdering::UNSORTED, xid, 0, 0, &rects).map_err(|e| err(&e))?;
        conn.flush().map_err(|e| err(&e))?;
        Ok(())
    }

    /// Reserves `strip` for the bar `window` (`_NET_WM_STRUT_PARTIAL`).
    pub fn reserve(&self, window: &Window, strip: Strip) -> Result<(), String> {
        let (Some(conn), Some(xid), Some(root_h)) = (&self.x11, x11_window_id(window), self.root_height()) else { return Ok(()) };
        let strut = strut(strip, root_h);
        let err = |e: &dyn std::fmt::Display| e.to_string();
        let atom = |name: &str| -> Result<u32, String> {
            Ok(conn.intern_atom(false, name.as_bytes()).map_err(|e| err(&e))?.reply().map_err(|e| err(&e))?.atom)
        };
        // The old four-number form for window managers without the partial one.
        for (name, values) in [("_NET_WM_STRUT_PARTIAL", &strut[..]), ("_NET_WM_STRUT", &strut[..4])] {
            conn.change_property32(PropMode::REPLACE, xid, atom(name)?, AtomEnum::CARDINAL, values).map_err(|e| err(&e))?;
        }
        conn.flush().map_err(|e| err(&e))?;
        Ok(())
    }

    /// Makes `window` a companion of `owner` (`WM_TRANSIENT_FOR`): the
    /// window manager then keeps it with the owner, as when the owner's
    /// taskbar icon is clicked (see `raise_together` for a click on the
    /// owner). `None` makes it a window of its own again.
    pub fn attach(&self, window: &Window, owner: Option<&Window>) -> Result<(), String> {
        let (Some(conn), Some(xid)) = (&self.x11, x11_window_id(window)) else { return Ok(()) };
        let err = |e: &dyn std::fmt::Display| e.to_string();
        match owner.and_then(x11_window_id) {
            Some(owner) => conn.change_property32(PropMode::REPLACE, xid, AtomEnum::WM_TRANSIENT_FOR, AtomEnum::WINDOW, &[owner]),
            None => conn.delete_property(xid, AtomEnum::WM_TRANSIENT_FOR.into()),
        }
        .map_err(|e| err(&e))?;
        conn.flush().map_err(|e| err(&e))
    }

    /// Nothing to do: menus and tooltips are override-redirect windows,
    /// which the window manager never focuses.
    pub fn no_activate(&self, _window: &Window) {}

    /// Brings `windows` up, back to front. Not every window manager brings
    /// a window's companions up with it: xfwm4 raises only the one clicked,
    /// and another program's window then cuts the docked group in two.
    /// The top one, just clicked, the window manager has brought up; the
    /// others go right under it. Raised outright they would take the focus
    /// from it, as xfwm4 focuses a window a program brings up.
    pub fn raise_together(&self, windows: &[&Window]) {
        let Some(conn) = &self.x11 else { return };
        let xids: Vec<u32> = windows.iter().filter_map(|w| x11_window_id(w)).collect();
        let Some((&top, under)) = xids.split_last() else { return };
        let below_top = ConfigureWindowAux::new().sibling(top).stack_mode(StackMode::BELOW);
        for &xid in under {
            if let Err(e) = conn.configure_window(xid, &below_top) {
                log::debug!("raise: {e}");
            }
        }
        let _ = conn.flush();
    }

    /// Where the mouse is on the X screen. A window's own mouse position
    /// is from where the window was when the event was sent; the window
    /// manager moves a dragged window later than asked, so that adds up
    /// to a point off the mouse.
    pub fn pointer(&self) -> Option<(i32, i32)> {
        let conn = self.x11.as_ref()?;
        let root = conn.setup().roots.first()?.root;
        let reply = conn.query_pointer(root).ok()?.reply().ok()?;
        Some((reply.root_x.into(), reply.root_y.into()))
    }

    /// Nothing to give back: a strut goes with its window.
    pub fn release(&self, _window: &Window) {}

    /// The work areas the window manager publishes for the current desktop:
    /// one per monitor where it says so (`_GTK_WORKAREAS_Dn`), else one
    /// for all of them (`_NET_WORKAREA`).
    pub fn work_areas(&self) -> Vec<Screen> {
        let Some(conn) = &self.x11 else { return Vec::new() };
        let Some(root) = conn.setup().roots.first().map(|r| r.root) else { return Vec::new() };
        let cardinals = |name: &str| -> Option<Vec<u32>> {
            let atom = conn.intern_atom(true, name.as_bytes()).ok()?.reply().ok()?.atom;
            let reply = conn.get_property(false, root, atom, AtomEnum::CARDINAL, 0, 1024).ok()?.reply().ok()?;
            let values = reply.value32()?.collect();
            Some(values)
        };
        let desktop = cardinals("_NET_CURRENT_DESKTOP").and_then(|d| d.first().copied()).unwrap_or(0) as usize;
        let rects = |v: &[u32]| -> Vec<Screen> {
            v.as_chunks::<4>().0.iter().map(|r| Screen { x: r[0] as i32, y: r[1] as i32, w: r[2] as i32, h: r[3] as i32 }).collect()
        };
        match cardinals(&format!("_GTK_WORKAREAS_D{desktop}")) {
            Some(per_monitor) if !per_monitor.is_empty() => rects(&per_monitor),
            _ => cardinals("_NET_WORKAREA").map(|all| rects(&all).into_iter().skip(desktop).take(1).collect()).unwrap_or_default(),
        }
    }

    /// The height of the X screen all monitors are part of.
    fn root_height(&self) -> Option<i32> {
        Some(self.x11.as_ref()?.setup().roots.first()?.height_in_pixels as i32)
    }
}

fn x11_window_id(w: &Window) -> Option<u32> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match w.window_handle().ok()?.as_raw() {
        RawWindowHandle::Xlib(h) => Some(h.window as u32),
        RawWindowHandle::Xcb(h) => Some(h.window.get()),
        _ => None,
    }
}

/// `_NET_WM_STRUT_PARTIAL` for `strip`: what it takes from the root window,
/// `root_h` pixels high.
fn strut(strip: Strip, root_h: i32) -> [u32; 12] {
    let Screen { x, y, w, h } = strip.area();
    let (start, end) = (x as u32, (x + w - 1) as u32);
    match strip.edge {
        BarEdge::Top => [0, 0, (y + h) as u32, 0, 0, 0, 0, 0, start, end, 0, 0],
        BarEdge::Bottom => [0, 0, 0, (root_h - y) as u32, 0, 0, 0, 0, 0, 0, start, end],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struts_count_from_the_root_window_edges() {
        // The right monitor is shorter than the 1080-pixel root window.
        let screen = Screen { x: 1920, y: 0, w: 1280, h: 1024 };
        let bottom = strut(Strip { edge: BarEdge::Bottom, screen, height: 28 }, 1080);
        assert_eq!(bottom, [0, 0, 0, 56 + 28, 0, 0, 0, 0, 0, 0, 1920, 3199]);
        let top = strut(Strip { edge: BarEdge::Top, screen, height: 28 }, 1080);
        assert_eq!(top, [0, 0, 28, 0, 0, 0, 0, 0, 1920, 3199, 0, 0]);
    }
}
