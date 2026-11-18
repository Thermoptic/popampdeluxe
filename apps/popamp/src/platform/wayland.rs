//! Wayland: a program cannot place its windows there, so only the main
//! window is a window (winit's). The equalizer, the playlist, menus and
//! tooltips are popups of it (xdg_popup), which a program does place, each
//! against the main window. winit knows nothing of them: they are made,
//! drawn and given the mouse here, on winit's connection.
//!
//! They are popups rather than subsurfaces because a compositor may show a
//! window's subsurfaces only within the window's outline: COSMIC cuts every
//! floating window to its rounded rectangle, and the equalizer and playlist
//! were then not seen at all, the menus only where they lay over the main
//! window. A popup is drawn apart from the window. Subsurfaces remain where
//! the compositor has no popups to give, or with `POPAMP_WAYLAND_SUBSURFACES=1`.
//!
//! Positions are the same "screen" pixels the rest of the program counts
//! in. The main window never moves in them, since nobody tells where the
//! compositor has it; a subsurface sits where its position is from the
//! main window's.

use super::{shape_rects, Mask};
use popamp_skin::cursors::CursorImage;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::os::fd::AsFd;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_callback::{self, WlCallback};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_pointer::{self, WlPointer};
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{self, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1;
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1::{self, ZwpRelativePointerV1};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols::xdg::shell::client::xdg_popup::{self, XdgPopup};
use wayland_protocols::xdg::shell::client::xdg_positioner::{self, XdgPositioner};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};
use winit::event::MouseButton;
use winit::platform::wayland::WindowExtWayland as _;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::window::Window;

/// What happened to a subsurface, named by its `Sub::id`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The mouse is at this point, in screen pixels.
    Moved(u32, (i32, i32)),
    Left(u32),
    /// A mouse button went down (`true`) or up.
    Button(u32, MouseButton, bool),
    /// The wheel turned by this many steps, away from the user if positive.
    Wheel(u32, i32),
    /// It is to be drawn again.
    Redraw(u32),
}

type Point = (i32, i32);

/// Where the compositor has a subsurface, which runs behind where it was
/// sent: the mouse is reported from the place the compositor knows.
#[derive(Default)]
struct Place {
    /// The position the compositor has confirmed.
    known: Cell<Point>,
    /// Positions sent and not confirmed yet, oldest first.
    sent: RefCell<VecDeque<Point>>,
}

/// What the main window and its subsurfaces share.
struct Shared {
    /// The main window's position.
    origin: Cell<Point>,
    /// Screen pixels in one of the compositor's units.
    scale: Cell<f64>,
    places: RefCell<HashMap<u32, Rc<Place>>>,
    redraws: RefCell<Vec<u32>>,
    /// Popups: the configure each is to acknowledge before it is next shown.
    configures: RefCell<HashMap<u32, u32>>,
    /// Popups the compositor took off the screen, to be made again.
    dismissed: RefCell<HashSet<u32>>,
    /// Where the compositor is to put each popup once it acknowledges the
    /// configure: until then it is still where it was, and the mouse is
    /// reported from there.
    placed: RefCell<HashMap<u32, Point>>,
    /// Positions asked for with `reposition` and not yet answered, by
    /// popup, with the token each was sent with.
    asked: RefCell<HashMap<u32, Vec<(u32, Point)>>>,
}

impl Shared {
    /// A length in screen pixels in the compositor's units.
    fn units(&self, px: i32) -> i32 {
        (px as f64 / self.scale.get()).round() as i32
    }
}

/// The picture the mouse has over the subsurfaces.
struct Pointer {
    surface: WlSurface,
    hot: Point,
    /// The buffer of a skin's cursor; the theme owns the arrow's.
    _own: Option<Buf>,
}

struct State {
    shared: Rc<Shared>,
    events: Vec<Event>,
    /// The subsurface under the mouse and the serial it entered with.
    over: Option<(u32, u32)>,
    pointer: Option<Pointer>,
    /// Wheel movement not yet a whole step.
    wheel: f64,
    /// Where the mouse last was, in screen pixels.
    last: (f64, f64),
    /// While a button is held over a subsurface, where the mouse is, from
    /// how far it moved rather than from where the subsurface is: one being
    /// dragged is not where the compositor reports the mouse from until a
    /// while after it moves, and the mouse read from it would run ahead of
    /// itself. `None` too where the compositor tells no such movement.
    held: Option<(u32, (f64, f64))>,
    /// The compositor tells how far the mouse moves.
    relative: bool,
}

delegate_noop!(State: WlCompositor);
delegate_noop!(State: WlSubcompositor);
delegate_noop!(State: WlSubsurface);
delegate_noop!(State: WlRegion);
delegate_noop!(State: WlShmPool);
delegate_noop!(State: WpViewporter);
delegate_noop!(State: WpViewport);
delegate_noop!(State: ignore WlSurface);
delegate_noop!(State: ignore WlShm);
delegate_noop!(State: ignore WlSeat);
delegate_noop!(State: XdgPositioner);
delegate_noop!(State: ZwpRelativePointerManagerV1);

/// The mouse moved by this much, wherever its pointer is drawn.
impl Dispatch<ZwpRelativePointerV1, ()> for State {
    fn event(state: &mut State, _: &ZwpRelativePointerV1, event: zwp_relative_pointer_v1::Event, _: &(), _: &Connection, _: &QueueHandle<State>) {
        if let zwp_relative_pointer_v1::Event::RelativeMotion { dx, dy, .. } = event {
            let scale = state.shared.scale.get();
            if let Some((id, at)) = &mut state.held {
                at.0 += dx * scale;
                at.1 += dy * scale;
                let p = (at.0.round() as i32, at.1.round() as i32);
                state.events.push(Event::Moved(*id, p));
            }
        }
    }
}

/// The compositor asks whether the program still answers.
impl Dispatch<XdgWmBase, ()> for State {
    fn event(_: &mut State, wm: &XdgWmBase, event: xdg_wm_base::Event, _: &(), _: &Connection, _: &QueueHandle<State>) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm.pong(serial);
        }
    }
}

/// A popup may be shown, after it acknowledges this; it is drawn again for that.
impl Dispatch<XdgSurface, u32> for State {
    fn event(state: &mut State, _: &XdgSurface, event: xdg_surface::Event, id: &u32, _: &Connection, _: &QueueHandle<State>) {
        if let xdg_surface::Event::Configure { serial } = event {
            state.shared.configures.borrow_mut().insert(*id, serial);
            state.shared.redraws.borrow_mut().push(*id);
        }
    }
}

/// Where the compositor puts a popup, from the main window's corner.
impl Dispatch<XdgPopup, u32> for State {
    fn event(state: &mut State, _: &XdgPopup, event: xdg_popup::Event, id: &u32, _: &Connection, _: &QueueHandle<State>) {
        let shared = &state.shared;
        match event {
            xdg_popup::Event::Configure { x, y, .. } => {
                // The popup goes where it was asked to be, as it may not be
                // moved to fit the screen; the compositor's figures are
                // not used, since they need not count from the same corner.
                log::debug!(target: "moves", "popup {id} configured at {x},{y}, asked {:?}", shared.placed.borrow().get(id));
            }
            xdg_popup::Event::Repositioned { token } => {
                let mut asked = shared.asked.borrow_mut();
                let list = asked.entry(*id).or_default();
                if let Some(i) = list.iter().position(|&(t, _)| t == token) {
                    shared.placed.borrow_mut().insert(*id, list[i].1);
                    list.drain(..=i);
                }
            }
            xdg_popup::Event::PopupDone => {
                log::debug!("Wayland: popup {id} dismissed");
                shared.dismissed.borrow_mut().insert(*id);
                shared.redraws.borrow_mut().push(*id);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(_: &mut State, _: &WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<State>) {}
}

/// A buffer the compositor has let go of is free to draw into again.
impl Dispatch<WlBuffer, Arc<AtomicBool>> for State {
    fn event(_: &mut State, _: &WlBuffer, event: wl_buffer::Event, busy: &Arc<AtomicBool>, _: &Connection, _: &QueueHandle<State>) {
        if let wl_buffer::Event::Release = event {
            busy.store(false, Ordering::Relaxed);
        }
    }
}

/// The compositor has taken everything sent before this, the oldest
/// unconfirmed position of subsurface `id` with it.
impl Dispatch<WlCallback, u32> for State {
    fn event(state: &mut State, _: &WlCallback, _: wl_callback::Event, id: &u32, _: &Connection, _: &QueueHandle<State>) {
        if let Some(place) = state.shared.places.borrow().get(id) {
            if let Some(pos) = place.sent.borrow_mut().pop_front() {
                place.known.set(pos);
            }
        }
    }
}

impl Dispatch<WlPointer, ()> for State {
    fn event(state: &mut State, pointer: &WlPointer, event: wl_pointer::Event, _: &(), _: &Connection, _: &QueueHandle<State>) {
        use wl_pointer::Event as E;
        let at = |state: &mut State, id: u32, x: f64, y: f64| {
            let (shared, scale) = (&state.shared, state.shared.scale.get());
            let known = shared.places.borrow().get(&id).map_or((0, 0), |p| p.known.get());
            state.last = (known.0 as f64 + x * scale, known.1 as f64 + y * scale);
            (known.0 + (x * scale) as i32, known.1 + (y * scale) as i32)
        };
        match event {
            E::Enter { serial, surface, surface_x, surface_y } => {
                let id = surface.id().protocol_id();
                // The main window and the dialogs are winit's.
                if !state.shared.places.borrow().contains_key(&id) {
                    state.over = None;
                    return;
                }
                state.over = Some((id, serial));
                if let Some(p) = &state.pointer {
                    pointer.set_cursor(serial, Some(&p.surface), p.hot.0, p.hot.1);
                }
                let p = at(state, id, surface_x, surface_y);
                state.events.push(Event::Moved(id, p));
            }
            E::Leave { .. } => {
                state.held = None;
                if let Some((id, _)) = state.over.take() {
                    state.events.push(Event::Left(id));
                }
            }
            E::Motion { surface_x, surface_y, .. } => {
                if state.held.is_some() {
                    return;
                }
                if let Some((id, _)) = state.over {
                    let p = at(state, id, surface_x, surface_y);
                    state.events.push(Event::Moved(id, p));
                }
            }
            E::Button { button, state: pressed, .. } => {
                let Some((id, _)) = state.over else { return };
                // The button codes of linux/input-event-codes.h.
                let button = match button {
                    0x110 => MouseButton::Left,
                    0x111 => MouseButton::Right,
                    0x112 => MouseButton::Middle,
                    _ => return,
                };
                let pressed = pressed == WEnum::Value(wl_pointer::ButtonState::Pressed);
                state.held = (pressed && state.relative).then_some((id, state.last));
                state.events.push(Event::Button(id, button, pressed));
            }
            E::Axis { axis: WEnum::Value(wl_pointer::Axis::VerticalScroll), value, .. } => {
                let Some((id, _)) = state.over else { return };
                // A wheel's click is 10 or 15 of these; a touchpad sends many small ones.
                state.wheel -= value / 10.0;
                let steps = state.wheel.trunc();
                if steps != 0.0 {
                    state.wheel -= steps;
                    state.events.push(Event::Wheel(id, steps as i32));
                }
            }
            _ => {}
        }
    }
}

/// A shared-memory picture a surface can show.
struct Buf {
    buffer: WlBuffer,
    pool: WlShmPool,
    map: memmap2::MmapMut,
    size: Point,
    /// The compositor is showing it.
    busy: Arc<AtomicBool>,
}

impl Buf {
    fn new(shm: &WlShm, qh: &QueueHandle<State>, (w, h): Point) -> Result<Buf, String> {
        let err = |e: std::io::Error| format!("no memory for a window picture: {e}");
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, Into::into);
        let path = dir.join(format!("popamp-{}-{:016x}", std::process::id(), fastrand::u64(..)));
        let file = std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(&path).map_err(err)?;
        // Only the handle is needed: the compositor gets it over the socket.
        let _ = std::fs::remove_file(&path);
        let len = w as usize * h as usize * 4;
        file.set_len(len as u64).map_err(err)?;
        // SAFETY: the file is new, unnamed and ours; only the compositor
        // reads it, and it never changes its size.
        let map = unsafe { memmap2::MmapMut::map_mut(&file) }.map_err(err)?;
        let pool = shm.create_pool(file.as_fd(), len as i32, qh, ());
        let busy = Arc::new(AtomicBool::new(false));
        let buffer = pool.create_buffer(0, w, h, w * 4, wl_shm::Format::Argb8888, qh, busy.clone());
        Ok(Buf { buffer, pool, map, size: (w, h), busy })
    }

    fn pixels(&mut self) -> &mut [u32] {
        // SAFETY: a mapping starts on a page, so it is aligned for u32,
        // and its length is a whole number of pixels.
        unsafe { std::slice::from_raw_parts_mut(self.map.as_mut_ptr().cast(), self.map.len() / 4) }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.pool.destroy();
    }
}

/// What a `Sub` and its `Shm` both hold of the connection.
#[derive(Clone)]
struct Link {
    conn: Connection,
    qh: QueueHandle<State>,
    compositor: WlCompositor,
    shm: WlShm,
    shared: Rc<Shared>,
}

/// The connection to the compositor, beside winit's use of it.
pub struct Wayland {
    link: Link,
    queue: EventQueue<State>,
    state: State,
    subcompositor: WlSubcompositor,
    viewporter: Option<WpViewporter>,
    /// What the other windows are popups of; `None` makes them subsurfaces.
    popups: Option<(XdgWmBase, XdgSurface)>,
    /// The main window's surface, which is winit's.
    main: WlSurface,
    _pointer: WlPointer,
    _relative: Option<ZwpRelativePointerV1>,
    /// The theme the arrow comes from: it owns the arrow's picture.
    _theme: Option<wayland_cursor::CursorTheme>,
}

impl Wayland {
    /// Joins winit's connection for `main`, the main window, which is at
    /// `origin`. `None` when the window is not a Wayland one.
    pub fn join(main: &Window, origin: Point) -> Option<Result<Wayland, String>> {
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) =
            (main.display_handle().ok()?.as_raw(), main.window_handle().ok()?.as_raw())
        else {
            return None;
        };
        let toplevel = main.xdg_toplevel().map(|t| t.as_ptr());
        Some(Self::connect(display.display.as_ptr().cast(), window.surface.as_ptr().cast(), toplevel, main.scale_factor(), origin))
    }

    fn connect(
        display: *mut std::ffi::c_void,
        surface: *mut std::ffi::c_void,
        toplevel: Option<*mut std::ffi::c_void>,
        scale: f64,
        origin: Point,
    ) -> Result<Wayland, String> {
        let err = |e: &dyn std::fmt::Display| format!("Wayland: {e}");
        // SAFETY: both pointers come from a living winit window, which the
        // caller keeps for as long as it keeps this.
        let conn = Connection::from_backend(unsafe { Backend::from_foreign_display(display.cast()) });
        let main = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.cast()) }.map_err(|e| err(&e))?;
        let main = WlSurface::from_id(&conn, main).map_err(|e| err(&e))?;
        let (globals, queue) = registry_queue_init::<State>(&conn).map_err(|e| err(&e))?;
        let qh = queue.handle();
        let compositor: WlCompositor = globals.bind(&qh, 1..=4, ()).map_err(|e| err(&e))?;
        let subcompositor: WlSubcompositor = globals.bind(&qh, 1..=1, ()).map_err(|e| err(&e))?;
        let shm: WlShm = globals.bind(&qh, 1..=1, ()).map_err(|e| err(&e))?;
        let seat: WlSeat = globals.bind(&qh, 1..=4, ()).map_err(|e| err(&e))?;
        let viewporter: Option<WpViewporter> = globals.bind(&qh, 1..=1, ()).ok();
        let popups = if std::env::var_os("POPAMP_WAYLAND_SUBSURFACES").is_some_and(|v| v == "1") {
            None
        } else {
            // SAFETY: as above; the toplevel is winit's, of the same window.
            // Popups move with `reposition`, from version 3 on.
            let parent = toplevel.and_then(|t| unsafe { ObjectId::from_ptr(XdgToplevel::interface(), t.cast()) }.ok());
            let parent = parent.and_then(|id| XdgToplevel::from_id(&conn, id).ok());
            let parent = parent.and_then(|t| smithay_client_toolkit::shell::xdg::window::Window::from_xdg_toplevel(&t));
            let wm = globals.bind::<XdgWmBase, _, _>(&qh, 3..=6, ()).ok();
            match (wm, parent) {
                (Some(wm), Some(parent)) => {
                    use smithay_client_toolkit::shell::xdg::XdgSurface as _;
                    Some((wm, parent.xdg_surface().clone()))
                }
                (wm, parent) => {
                    log::info!("Wayland: subsurfaces, not popups (xdg_wm_base 3: {}, main window's: {})", wm.is_some(), parent.is_some());
                    None
                }
            }
        };
        let pointer = seat.get_pointer(&qh, ());
        let relative = globals.bind::<ZwpRelativePointerManagerV1, _, _>(&qh, 1..=1, ()).ok();
        let relative = relative.map(|m| m.get_relative_pointer(&pointer, &qh, ()));
        log::debug!(target: "moves", "relative pointer: {}", relative.is_some());
        let shared = Rc::new(Shared {
            origin: Cell::new(origin),
            scale: Cell::new(scale),
            places: Default::default(),
            redraws: Default::default(),
            configures: Default::default(),
            dismissed: Default::default(),
            placed: Default::default(),
            asked: Default::default(),
        });
        let link = Link { conn, qh, compositor, shm, shared: shared.clone() };
        let state = State {
            shared,
            events: Vec::new(),
            over: None,
            pointer: None,
            wheel: 0.0,
            last: (0.0, 0.0),
            held: None,
            relative: relative.is_some(),
        };
        let mut wayland = Wayland { link, queue, state, subcompositor, viewporter, popups, main, _pointer: pointer, _relative: relative, _theme: None };
        wayland.set_cursor(None);
        Ok(wayland)
    }

    /// The screen pixels in one of the compositor's units changed, as when
    /// the main window goes to another monitor.
    pub fn set_scale(&self, scale: f64) {
        self.link.shared.scale.set(scale);
    }

    /// What to draw the main window into.
    pub fn main_surface(&self) -> Shm {
        Shm::new(self.link.clone(), self.main.clone(), None, false)
    }

    /// A subsurface at `pos`, and what to draw it into. Without `input`
    /// the mouse goes through it.
    pub fn sub(&self, pos: Point, input: bool) -> (Sub, Shm) {
        let (link, qh) = (&self.link, &self.link.qh);
        let surface = link.compositor.create_surface(qh, ());
        let role = match &self.popups {
            Some((wm, parent)) => Role::Popup(Rc::new(Popup {
                wm: wm.clone(),
                parent: parent.clone(),
                shown: RefCell::new(None),
                pos: Cell::new(pos),
                size: Cell::new((1, 1)),
                configured: Cell::new(false),
                token: Cell::new(0),
                moving: Cell::new(None),
            })),
            None => {
                let role = self.subcompositor.get_subsurface(&surface, &self.main, qh, ());
                // Shown when drawn, not when the main window next is.
                role.set_desync();
                Role::Sub(role)
            }
        };
        let popup = match &role {
            Role::Popup(p) => Some(p.clone()),
            Role::Sub(_) => None,
        };
        let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(&surface, qh, ()));
        if !input {
            let nothing = link.compositor.create_region(qh, ());
            surface.set_input_region(Some(&nothing));
            nothing.destroy();
        }
        let place = Rc::new(Place::default());
        place.known.set(pos);
        let id = surface.id().protocol_id();
        if input {
            link.shared.places.borrow_mut().insert(id, place.clone());
        }
        let sub = Sub { link: link.clone(), surface: surface.clone(), role, main: self.main.clone(), place };
        sub.move_to(pos);
        let mut shm = Shm::new(link.clone(), surface, viewport, true);
        shm.popup = popup;
        (sub, shm)
    }

    /// The picture of the mouse over the subsurfaces: a skin's cursor, or
    /// the desktop's arrow.
    pub fn set_cursor(&mut self, cursor: Option<&CursorImage>) {
        let Link { conn, qh, compositor, shm, .. } = &self.link;
        let surface = self.state.pointer.take().map_or_else(|| compositor.create_surface(qh, ()), |p| p.surface);
        let mut own = None;
        let hot;
        match cursor {
            Some(c) => {
                let Ok(mut buf) = Buf::new(shm, qh, (c.w as i32, c.h as i32)) else { return };
                for (px, rgba) in buf.pixels().iter_mut().zip(c.rgba.as_chunks::<4>().0) {
                    // Colours come multiplied by their opacity.
                    let part = |v: u8| v as u32 * rgba[3] as u32 / 255;
                    *px = (rgba[3] as u32) << 24 | part(rgba[0]) << 16 | part(rgba[1]) << 8 | part(rgba[2]);
                }
                surface.attach(Some(&buf.buffer), 0, 0);
                hot = (c.hot_x as i32, c.hot_y as i32);
                own = Some(buf);
            }
            None => {
                if self._theme.is_none() {
                    self._theme = wayland_cursor::CursorTheme::load(conn, shm.clone(), 24).ok();
                }
                let arrow = self
                    ._theme
                    .as_mut()
                    .and_then(|t| ["default", "left_ptr"].into_iter().find_map(|n| t.get_cursor(n).map(|c| c[0].clone())));
                let Some(arrow) = arrow else { return };
                surface.attach(Some(&*arrow), 0, 0);
                let (x, y) = arrow.hotspot();
                hot = (x as i32, y as i32);
            }
        }
        surface.damage(0, 0, i32::MAX, i32::MAX);
        surface.commit();
        if let Some((_, serial)) = self.state.over {
            self._pointer.set_cursor(serial, Some(&surface), hot.0, hot.1);
        }
        self.state.pointer = Some(Pointer { surface, hot, _own: own });
        let _ = conn.flush();
    }

    /// What happened to the subsurfaces since the last call.
    pub fn events(&mut self) -> Vec<Event> {
        if let Err(e) = self.queue.dispatch_pending(&mut self.state) {
            log::warn!("Wayland: {e}");
        }
        let _ = self.link.conn.flush();
        let mut events = std::mem::take(&mut self.state.events);
        events.extend(self.link.shared.redraws.borrow_mut().drain(..).map(Event::Redraw));
        events
    }
}

/// What makes a surface a part of the main window.
enum Role {
    Sub(WlSubsurface),
    Popup(Rc<Popup>),
}

/// A popup of the main window, which its `Sub` places and its `Shm` shows.
/// It is made when it is first shown and unmade when it is hidden: a popup
/// once taken off the screen is not shown again.
struct Popup {
    wm: XdgWmBase,
    /// The main window's.
    parent: XdgSurface,
    shown: RefCell<Option<(XdgSurface, XdgPopup)>>,
    /// Where it is to be, in screen pixels.
    pos: Cell<Point>,
    /// Its size in the compositor's units.
    size: Cell<Point>,
    /// It has acknowledged a configure since it was made.
    configured: Cell<bool>,
    /// The last `reposition`'s token.
    token: Cell<u32>,
    /// Where the next commit puts it.
    moving: Cell<Option<Point>>,
}

impl Popup {
    /// Where the popup goes: `pos` from the main window's corner, never
    /// moved to fit the screen, which would take it off the windows it is
    /// docked to.
    fn positioner(&self, link: &Link) -> XdgPositioner {
        let Link { qh, shared, .. } = link;
        let p = self.wm.create_positioner(qh, ());
        let ((w, h), pos, origin) = (self.size.get(), self.pos.get(), shared.origin.get());
        p.set_size(w.max(1), h.max(1));
        p.set_anchor_rect(0, 0, 1, 1);
        p.set_anchor(xdg_positioner::Anchor::TopLeft);
        p.set_gravity(xdg_positioner::Gravity::BottomRight);
        p.set_constraint_adjustment(xdg_positioner::ConstraintAdjustment::empty());
        p.set_offset(shared.units(pos.0 - origin.0), shared.units(pos.1 - origin.1));
        p
    }

    /// Asks for the popup to be where `pos` and `size` say now.
    fn reposition(&self, link: &Link, id: u32) {
        if let Some((_, popup)) = &*self.shown.borrow() {
            let p = self.positioner(link);
            let token = self.token.get().wrapping_add(1);
            self.token.set(token);
            link.shared.asked.borrow_mut().entry(id).or_default().push((token, self.pos.get()));
            popup.reposition(&p, token);
            p.destroy();
        }
    }

    /// Whether `surface` may be shown, `size` units big. The first time
    /// it makes the popup, which may be shown once the compositor answers.
    fn ready(&self, link: &Link, surface: &WlSurface, size: Point) -> bool {
        let id = surface.id().protocol_id();
        if link.shared.dismissed.borrow_mut().remove(&id) {
            self.hide(link, surface);
        }
        if self.shown.borrow().is_none() {
            self.size.set(size);
            let xdg = self.wm.get_xdg_surface(surface, &link.qh, id);
            let p = self.positioner(link);
            let popup = xdg.get_popup(Some(&self.parent), &p, &link.qh, id);
            link.shared.asked.borrow_mut().remove(&id);
            link.shared.placed.borrow_mut().insert(id, self.pos.get());
            p.destroy();
            self.configured.set(false);
            *self.shown.borrow_mut() = Some((xdg, popup));
            surface.commit();
            let _ = link.conn.flush();
            return false;
        }
        if self.size.get() != size {
            self.size.set(size);
            self.reposition(link, id);
        }
        if let Some(serial) = link.shared.configures.borrow_mut().remove(&id) {
            if let Some((xdg, _)) = &*self.shown.borrow() {
                xdg.ack_configure(serial);
                self.configured.set(true);
                // It moves with the commit that follows.
                if let Some(at) = link.shared.placed.borrow_mut().remove(&id) {
                    self.moving.set(Some(at));
                }
            }
        }
        if let (true, Some((xdg, _))) = (self.configured.get(), &*self.shown.borrow()) {
            xdg.set_window_geometry(0, 0, size.0.max(1), size.1.max(1));
        }
        self.configured.get()
    }

    /// Takes the popup off the screen and unmakes it.
    fn hide(&self, link: &Link, surface: &WlSurface) {
        let Some((xdg, popup)) = self.shown.borrow_mut().take() else { return };
        surface.attach(None, 0, 0);
        surface.commit();
        popup.destroy();
        xdg.destroy();
        link.shared.configures.borrow_mut().remove(&surface.id().protocol_id());
        let _ = link.conn.flush();
    }
}

/// A part of the main window: a popup of it, or a subsurface.
pub struct Sub {
    link: Link,
    surface: WlSurface,
    role: Role,
    main: WlSurface,
    place: Rc<Place>,
}

impl Sub {
    pub fn id(&self) -> u32 {
        self.surface.id().protocol_id()
    }

    pub fn move_to(&self, pos: Point) {
        let Link { conn, qh, shared, .. } = &self.link;
        let role = match &self.role {
            Role::Sub(role) => role,
            Role::Popup(popup) => {
                popup.pos.set(pos);
                popup.reposition(&self.link, self.id());
                let _ = conn.flush();
                return;
            }
        };
        let origin = shared.origin.get();
        role.set_position(shared.units(pos.0 - origin.0), shared.units(pos.1 - origin.1));
        // A subsurface moves when its parent is next shown.
        self.main.commit();
        self.place.sent.borrow_mut().push_back(pos);
        conn.display().sync(qh, self.id());
        let _ = conn.flush();
    }

    pub fn request_redraw(&self) {
        self.link.shared.redraws.borrow_mut().push(self.id());
    }
}

impl Drop for Sub {
    fn drop(&mut self) {
        self.link.shared.places.borrow_mut().remove(&self.id());
        self.link.shared.dismissed.borrow_mut().remove(&self.id());
        self.link.shared.placed.borrow_mut().remove(&self.id());
        self.link.shared.asked.borrow_mut().remove(&self.id());
        match &self.role {
            Role::Sub(role) => role.destroy(),
            Role::Popup(popup) => popup.hide(&self.link, &self.surface),
        }
        self.surface.destroy();
        self.main.commit();
        let _ = self.link.conn.flush();
    }
}

/// Draws a surface from memory shared with the compositor, with see-through
/// pixels where the window's shape has none.
pub struct Shm {
    link: Link,
    surface: WlSurface,
    /// Sizes a subsurface in the compositor's units; winit does that for
    /// the main window.
    viewport: Option<WpViewport>,
    /// A subsurface or popup, not the main window.
    sub: bool,
    /// The popup it is, when it is one.
    popup: Option<Rc<Popup>>,
    bufs: Vec<Buf>,
    /// The pixels the shape shows, for the size they were made for.
    shape: Option<(Point, Vec<bool>)>,
    /// Where the mouse is taken, when not everywhere; in screen pixels.
    input: Option<Vec<(i16, i16, u16, u16)>>,
    input_changed: bool,
    /// The compositor is ready for a picture: a window must wait for the
    /// compositor's first word about it.
    ready: bool,
}

impl Shm {
    fn new(link: Link, surface: WlSurface, viewport: Option<WpViewport>, sub: bool) -> Shm {
        Shm { link, surface, viewport, sub, popup: None, bufs: Vec::new(), shape: None, input: None, input_changed: false, ready: sub }
    }

    /// The main window may be drawn from now on.
    pub fn ready(&mut self) {
        self.ready = true;
    }

    /// Shows only what `mask` covers of a window `size` screen pixels big,
    /// and takes the mouse only there.
    pub fn set_shape(&mut self, mask: Option<&Mask>, size: Point) {
        let rects = mask.map(|m| shape_rects(Some(m), size));
        self.shape = rects.as_ref().map(|rects| {
            let mut on = vec![false; (size.0 * size.1).max(0) as usize];
            for &(x, y, w, h) in rects {
                for row in y as i32..(y as i32 + h as i32).min(size.1) {
                    let start = (row * size.0 + x as i32) as usize;
                    on[start..start + (w as i32).min(size.0 - x as i32) as usize].fill(true);
                }
            }
            (size, on)
        });
        self.input = rects;
        self.input_changed = true;
    }

    /// Shows a picture `w` x `h` screen pixels big, which `draw` paints.
    pub fn present(&mut self, (w, h): Point, draw: impl FnOnce(&mut [u32])) {
        if !self.ready || w <= 0 || h <= 0 {
            return;
        }
        if let Some(popup) = &self.popup {
            let units = (self.link.shared.units(w).max(1), self.link.shared.units(h).max(1));
            if !popup.ready(&self.link, &self.surface, units) {
                return;
            }
        }
        let full = self.whole_units((w, h));
        let Some(i) = self.free_buf(full) else { return };
        let buf = &mut self.bufs[i];
        let shape = self.shape.as_ref().filter(|(size, _)| *size == (w, h)).map(|(_, on)| on);
        let finish = |px: &mut [u32]| match shape {
            Some(on) => px.iter_mut().zip(on).for_each(|(p, on)| *p = if *on { *p | 0xff00_0000 } else { 0 }),
            None => px.iter_mut().for_each(|p| *p |= 0xff00_0000),
        };
        if full == (w, h) {
            draw(buf.pixels());
            finish(buf.pixels());
        } else {
            // Drawn apart, then put into the bigger picture row by row.
            let mut px = vec![0; (w * h) as usize];
            draw(&mut px);
            finish(&mut px);
            let out = buf.pixels();
            out.fill(0);
            for (row, from) in out.chunks_exact_mut(full.0 as usize).zip(px.chunks_exact(w as usize)) {
                row[..w as usize].copy_from_slice(from);
            }
        }
        buf.busy.store(true, Ordering::Relaxed);
        self.surface.attach(Some(&buf.buffer), 0, 0);
        self.commit(full);
    }

    /// `size` made a whole number of the compositor's units, which the
    /// main window's picture must be where a unit is two or three screen
    /// pixels: a compositor may refuse any other.
    fn whole_units(&self, size: Point) -> Point {
        let scale = self.link.shared.scale.get();
        if self.sub || scale.fract() != 0.0 {
            return size;
        }
        let unit = scale as i32;
        let up = |px: i32| (px + unit - 1) / unit * unit;
        (up(size.0), up(size.1))
    }

    /// Takes the window off the screen. The main window stays one for the
    /// compositor, with nothing to see or click: taking its picture away
    /// would be closing it.
    pub fn hide(&mut self, size: Point) {
        if !self.ready {
            return;
        }
        if let Some(popup) = &self.popup {
            popup.hide(&self.link, &self.surface);
            return;
        }
        if self.sub {
            self.surface.attach(None, 0, 0);
            self.surface.commit();
            let _ = self.link.conn.flush();
            return;
        }
        let size = self.whole_units(size);
        let Some(i) = self.free_buf(size) else { return };
        let buf = &mut self.bufs[i];
        buf.pixels().fill(0);
        buf.busy.store(true, Ordering::Relaxed);
        self.surface.attach(Some(&buf.buffer), 0, 0);
        let (input, shape) = (self.input.replace(Vec::new()), self.shape.take());
        self.input_changed = true;
        self.commit(size);
        (self.input, self.shape, self.input_changed) = (input, shape, true);
    }

    /// Sends what was attached, `size` screen pixels big, to the screen.
    fn commit(&mut self, size: Point) {
        let Link { conn, qh, compositor, shared, .. } = &self.link;
        if self.surface.version() >= 4 {
            self.surface.damage_buffer(0, 0, size.0, size.1);
        } else {
            self.surface.damage(0, 0, i32::MAX, i32::MAX);
        }
        if let Some(viewport) = &self.viewport {
            viewport.set_destination(shared.units(size.0).max(1), shared.units(size.1).max(1));
        }
        if std::mem::take(&mut self.input_changed) {
            let region = self.input.as_ref().map(|rects| {
                let region = compositor.create_region(qh, ());
                let scale = shared.scale.get();
                for &(x, y, w, h) in rects {
                    // Whole units that the rectangle covers at least half of.
                    let unit = |px: i32| (px as f64 / scale).round() as i32;
                    let (x0, y0) = (unit(x as i32), unit(y as i32));
                    region.add(x0, y0, unit(x as i32 + w as i32) - x0, unit(y as i32 + h as i32) - y0);
                }
                region
            });
            self.surface.set_input_region(region.as_ref());
            if let Some(region) = region {
                region.destroy();
            }
        }
        self.surface.commit();
        // A popup that moves with this commit is there for the mouse once
        // the compositor has taken it: what the mouse did before comes
        // from where it was.
        if let Some(at) = self.popup.as_ref().and_then(|p| p.moving.take()) {
            let id = self.surface.id().protocol_id();
            if let Some(place) = shared.places.borrow().get(&id) {
                place.sent.borrow_mut().push_back(at);
                conn.display().sync(qh, id);
            }
        }
        let _ = conn.flush();
    }

    /// The index of a buffer of `size` the compositor is not showing.
    fn free_buf(&mut self, size: Point) -> Option<usize> {
        self.bufs.retain(|b| b.size == size || b.busy.load(Ordering::Relaxed));
        if let Some(i) = self.bufs.iter().position(|b| b.size == size && !b.busy.load(Ordering::Relaxed)) {
            return Some(i);
        }
        // The compositor holds them all: it is behind, and this frame is skipped.
        if self.bufs.len() >= 4 {
            return None;
        }
        match Buf::new(&self.link.shm, &self.link.qh, size) {
            Ok(buf) => {
                self.bufs.push(buf);
                Some(self.bufs.len() - 1)
            }
            Err(e) => {
                log::warn!("{e}");
                None
            }
        }
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        if let Some(viewport) = &self.viewport {
            viewport.destroy();
        }
    }
}
