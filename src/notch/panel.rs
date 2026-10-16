use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSAutoresizingMaskOptions, NSBackingStoreType,
    NSBezierPath, NSColor, NSEvent, NSFont, NSFontWeight, NSFontWeightBold, NSFontWeightMedium, NSFontWeightRegular,
    NSFontWeightSemibold, NSLineBreakMode, NSPanel, NSScreen, NSStatusWindowLevel, NSTextAlignment, NSTextField,
    NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer};

use super::card::{self, Expand, Outcome, Press};
use super::motion::{Frame, Motion, SETTLE, Size};
use super::rinco::{self, Mood};
use super::{Action, Board, Card};
use crate::board::Built;
use crate::watch::Shared;
use crate::{act, looking};

const WIDTH: f64 = 340.0;
const PILL_MIN: f64 = 200.0;
const PILL_WINGS: f64 = 26.0;
const PILL_ROW: f64 = 24.0;
const PILL_SIDE: f64 = 14.0;
const PILL_MASCOT: f64 = 16.0;
const PILL_GAP: f64 = 8.0;
const SIDE: f64 = 10.0;
const BOTTOM: f64 = 10.0;
const BELOW_NOTCH: f64 = 6.0;
const NO_NOTCH_TOP: f64 = 4.0;
const GAP: f64 = 6.0;
const RADIUS: f64 = 20.0;
const CARD_RADIUS: f64 = 11.0;
const CARD_V: f64 = 10.0;
const CARD_H: f64 = 12.0;
const MASCOT: f64 = 32.0;
const MASCOT_GAP: f64 = 10.0;
const LINE_GAP: f64 = 2.0;
const TITLE: f64 = 17.0;
const LINE: f64 = 15.0;
const PLACE: f64 = 13.0;
const MORE: f64 = 14.0;
const PILLS_TOP: f64 = 6.0;
const PILL: f64 = 20.0;
const PILL_PAD: f64 = 12.0;
const OPEN_PAD: f64 = 10.0;
const PILLS_GAP: f64 = 6.0;
const META_GAP: f64 = 6.0;
const TIME: f64 = 26.0;
const NO_NOTCH_WIDTH: f64 = 120.0;
const NO_NOTCH_HEIGHT: f64 = 1.0;
const MAX_SHOWN: usize = 3;
const TEXT_WIDTH: f64 = WIDTH - 2.0 * SIDE - CARD_H - (CARD_H + MASCOT + MASCOT_GAP);
const COMMAND_SIZE: f64 = 10.5;
const PENDING_FOR: Duration = Duration::from_secs(10);
const RESULT_FOR: Duration = Duration::from_millis(1200);
const POLL_EVERY: f64 = 0.1;
const FRAME_EVERY: f64 = 1.0 / 60.0;
const MASCOT_EVERY: f64 = 1.0 / 30.0;
const CARD_FILL: u32 = 0x1c_1b_20;
const TIME_TEXT: u32 = 0xc4_c0_d6;
const WHITE: u32 = 0xff_ff_ff;
const SUBTLE: u32 = 0x95_91_ab;

#[derive(Clone, Copy)]
enum Weight {
    Regular,
    Medium,
    Semibold,
    Bold,
}

impl Weight {
    fn value(self) -> NSFontWeight {
        unsafe {
            match self {
                Self::Regular => NSFontWeightRegular,
                Self::Medium => NSFontWeightMedium,
                Self::Semibold => NSFontWeightSemibold,
                Self::Bold => NSFontWeightBold,
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Place {
    screen: NSRect,
    top: f64,
    notch: Size,
}

type Click = (Press, Box<Card>);

#[derive(Debug, Clone, PartialEq)]
struct Item {
    card: Card,
    outcome: Option<Outcome>,
}

impl Item {
    fn mood(&self) -> Mood {
        self.outcome.as_ref().map_or_else(|| card::mood(&self.card), Outcome::mood)
    }
}

#[derive(PartialEq)]
struct Shown {
    items: Vec<Item>,
    expanded: bool,
}

struct Ivars {
    shared: Arc<Shared>,
    panel: OnceCell<Retained<NSPanel>>,
    terminal: Option<i32>,
    built: RefCell<Built>,
    looking: Cell<bool>,
    board: RefCell<Board>,
    dismissed: RefCell<Vec<Card>>,
    answered: RefCell<Vec<(Card, Outcome, Instant)>>,
    appeared: RefCell<Vec<(Card, Instant)>>,
    expand: Cell<Expand>,
    shown: RefCell<Option<Shown>>,
    generation: Cell<u64>,
    pending: RefCell<Vec<(Card, usize, Instant)>>,
    clocks: RefCell<Vec<(Retained<NSTextField>, u64)>>,
    mascots: RefCell<Vec<Retained<Sprite>>>,
    moods: RefCell<HashMap<(u64, Mood), Instant>>,
    content: RefCell<Option<Retained<NSView>>>,
    target: Cell<Size>,
    size: Cell<Option<Size>>,
    place: Cell<Option<Place>>,
    motion: Cell<Option<(Motion, Instant)>>,
    animating: RefCell<Option<Retained<NSTimer>>>,
    blinking: RefCell<Option<Retained<NSTimer>>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct Notch;

    unsafe impl NSObjectProtocol for Notch {}

    unsafe impl NSApplicationDelegate for Notch {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            let panel = make_panel(self.mtm());
            let _ = self.ivars().panel.set(panel);
            unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    POLL_EVERY,
                    self,
                    sel!(tick:),
                    None,
                    true,
                );
            }
        }
    }

    impl Notch {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: &NSTimer) {
            let ivars = self.ivars();
            let built = ivars.shared.take();
            let now = Instant::now();
            let looking = looking::looking(ivars.terminal);
            ivars.shared.looking.store(looking, Ordering::Relaxed);
            if ivars.shared.forward.swap(false, Ordering::Relaxed) {
                looking::bring_forward(ivars.terminal);
            }
            let flipped = ivars.looking.replace(looking) != looking;
            if flipped || built.is_some() {
                if let Some(built) = built {
                    *ivars.built.borrow_mut() = built;
                }
                let board = ivars.built.borrow().board(looking);
                self.take(board, now);
            }
            self.settle(now);
            self.expire(now);
            self.hover(now);
            self.render();
            self.update_clocks();
        }

        #[unsafe(method(frame:))]
        fn frame(&self, _timer: &NSTimer) {
            self.step();
        }

        #[unsafe(method(blink:))]
        fn blink(&self, _timer: &NSTimer) {
            for sprite in self.ivars().mascots.borrow().iter() {
                sprite.advance();
            }
        }
    }
);

#[derive(Default)]
struct ShapeIvars {
    fill: Cell<Option<u32>>,
    hover: Cell<Option<u32>>,
    hovered: Cell<bool>,
    radius: Cell<f64>,
    click: RefCell<Option<Click>>,
    generation: Cell<u64>,
    notch: OnceCell<Retained<Notch>>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ShapeIvars]
    struct Shape;

    impl Shape {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {}

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            let ivars = self.ivars();
            let click = ivars.click.borrow().clone();
            if let (Some(click), Some(notch)) = (click, ivars.notch.get()) {
                notch.clicked(click, ivars.generation.get());
            }
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.ivars().hovered.set(true);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().hovered.set(false);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let ivars = self.ivars();
            let hovered = ivars.hovered.get().then(|| ivars.hover.get()).flatten();
            if let Some(fill) = hovered.or(ivars.fill.get()) {
                colour(fill, 1.0).setFill();
                let radius = ivars.radius.get();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(self.bounds(), radius, radius).fill();
            }
        }
    }
);

struct SpriteIvars {
    mood: Cell<Mood>,
    since: Cell<Instant>,
    frame: Cell<Option<usize>>,
    pixel: Cell<f64>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SpriteIvars]
    struct Sprite;

    impl Sprite {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let (pixel, height) = (self.ivars().pixel.get(), self.bounds().size.height);
            let Some(rows) = rinco::frame(self.ivars().mood.get(), self.ivars().since.get().elapsed()) else { return };
            for (y, row) in (0_u32..).zip(rows) {
                for (x, dot) in (0_u32..).zip(row.chars()) {
                    let Some(rgb) = rinco::colour(dot) else { continue };
                    colour(rgb, 1.0).setFill();
                    let top = height - f64::from(y + 1) * pixel;
                    NSBezierPath::fillRect(rect(f64::from(x) * pixel, top, pixel, pixel));
                }
            }
        }
    }
);

impl Sprite {
    fn advance(&self) {
        let frame = rinco::index(self.ivars().mood.get(), self.ivars().since.get().elapsed());
        if self.ivars().frame.replace(frame) != frame {
            self.setNeedsDisplay(true);
        }
    }
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    struct Background;

    impl Background {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            NSColor::blackColor().setFill();
            NSBezierPath::fillRect(self.bounds());
        }
    }
);

impl Notch {
    fn new(mtm: MainThreadMarker, shared: Arc<Shared>, terminal: Option<i32>) -> Retained<Self> {
        let ivars = Ivars {
            shared,
            panel: OnceCell::new(),
            terminal,
            built: RefCell::new(Built::default()),
            looking: Cell::new(true),
            board: RefCell::new(Board::default()),
            dismissed: RefCell::new(Vec::new()),
            answered: RefCell::new(Vec::new()),
            appeared: RefCell::new(Vec::new()),
            expand: Cell::new(Expand::default()),
            shown: RefCell::new(None),
            generation: Cell::new(0),
            pending: RefCell::new(Vec::new()),
            clocks: RefCell::new(Vec::new()),
            mascots: RefCell::new(Vec::new()),
            moods: RefCell::new(HashMap::new()),
            content: RefCell::new(None),
            target: Cell::new(Size { width: WIDTH, height: 0.0 }),
            size: Cell::new(None),
            place: Cell::new(None),
            motion: Cell::new(None),
            animating: RefCell::new(None),
            blinking: RefCell::new(None),
        };
        let this = Self::alloc(mtm).set_ivars(ivars);
        unsafe { msg_send![super(this), init] }
    }

    fn clicked(&self, (press, card): Click, generation: u64) {
        if generation != self.ivars().generation.get() || card::answering(&self.ivars().pending.borrow(), card.pane) {
            return;
        }
        match press {
            Press::Open(pane) => self.send(Action::Open(pane)),
            Press::Answer(action) => {
                if let Action::Answer { choice, .. } = &action {
                    self.ivars().pending.borrow_mut().push((*card, *choice, Instant::now()));
                }
                self.send(action);
            }
        }
        self.render();
    }

    fn settle(&self, now: Instant) {
        let mut pending = self.ivars().pending.borrow_mut();
        let mut dismissed = self.ivars().dismissed.borrow_mut();
        for (pane, choice, sent) in self.ivars().shared.take_answers() {
            let Some(at) = pending.iter().position(|(card, k, _)| card.pane == pane && *k == choice) else { continue };
            let (card, choice, _) = pending.remove(at);
            match card::outcome(&card, choice).filter(|_| sent) {
                Some(outcome) => self.ivars().answered.borrow_mut().push((card, outcome, now)),
                None => dismissed.push(card),
            }
        }
        let (stale, kept): (Vec<_>, Vec<_>) =
            pending.drain(..).partition(|(_, _, at)| now.saturating_duration_since(*at) >= PENDING_FOR);
        *pending = kept;
        dismissed.extend(stale.into_iter().map(|(card, _, _)| card));
    }

    fn take(&self, board: Board, now: Instant) {
        let before: Vec<Card> = self.items().into_iter().map(|item| item.card).collect();
        if card::arrived(&before, &board.cards) {
            let mut expand = self.ivars().expand.get();
            expand.arrive(now);
            self.ivars().expand.set(expand);
        }
        self.ivars().dismissed.borrow_mut().retain(|c| board.cards.contains(c));
        let mut appeared = self.ivars().appeared.borrow_mut();
        appeared.retain(|(c, _)| board.cards.contains(c));
        for c in &board.cards {
            if !appeared.iter().any(|(seen, _)| seen == c) {
                appeared.push((c.clone(), now));
            }
        }
        drop(appeared);
        *self.ivars().board.borrow_mut() = board;
    }

    fn expire(&self, now: Instant) {
        let mut answered = self.ivars().answered.borrow_mut();
        let (gone, kept): (Vec<_>, Vec<_>) =
            answered.drain(..).partition(|(_, _, at)| now.saturating_duration_since(*at) >= RESULT_FOR);
        *answered = kept;
        let mut dismissed = self.ivars().dismissed.borrow_mut();
        dismissed.extend(gone.into_iter().map(|(card, _, _)| card));
        if self.ivars().expand.get().at(now) {
            return;
        }
        for (c, at) in self.ivars().appeared.borrow().iter() {
            let over = card::lasts(c).is_some_and(|lasts| now.saturating_duration_since(*at) >= lasts);
            if over && !dismissed.contains(c) {
                dismissed.push(c.clone());
            }
        }
    }

    fn hover(&self, now: Instant) {
        let inside = self.ivars().size.get().is_some()
            && self.ivars().panel.get().is_some_and(|panel| {
                let (at, frame) = (NSEvent::mouseLocation(), panel.frame());
                at.x >= frame.origin.x
                    && at.x <= frame.origin.x + frame.size.width
                    && at.y >= frame.origin.y
                    && at.y <= frame.origin.y + frame.size.height
            });
        let mut expand = self.ivars().expand.get();
        expand.hover(inside, now);
        self.ivars().expand.set(expand);
    }

    fn send(&self, action: Action) {
        let (forward, report) = (Arc::clone(&self.ivars().shared), Arc::clone(&self.ivars().shared));
        act::spawn(
            action,
            move || forward.forward.store(true, Ordering::Relaxed),
            move |action, result| {
                if let Action::Answer { pane, choice, .. } = action {
                    report.answered(*pane, *choice, result.sent);
                }
            },
        );
    }

    fn items(&self) -> Vec<Item> {
        let (board, dismissed, answered, pending) = (
            self.ivars().board.borrow(),
            self.ivars().dismissed.borrow(),
            self.ivars().answered.borrow(),
            self.ivars().pending.borrow(),
        );
        let waiting = board
            .cards
            .iter()
            .filter(|c| !dismissed.contains(c) && !answered.iter().any(|(a, _, _)| a.pane == c.pane))
            .filter(|c| !pending.iter().any(|(p, _, _)| p.pane == c.pane))
            .map(|c| Item { card: c.clone(), outcome: None });
        let results =
            answered.iter().rev().map(|(card, outcome, _)| Item { card: card.clone(), outcome: Some(outcome.clone()) });
        results.chain(waiting).collect()
    }

    fn render(&self) {
        let Some(panel) = self.ivars().panel.get() else { return };
        let mtm = self.mtm();
        let items = self.items();
        let expanded = self.ivars().expand.get().at(Instant::now());
        let shown = Shown { items: items.clone(), expanded };
        if self.ivars().shown.borrow().as_ref() == Some(&shown) {
            return;
        }
        *self.ivars().shown.borrow_mut() = Some(shown);
        self.ivars().generation.set(self.ivars().generation.get() + 1);
        let place = place(mtm);
        self.ivars().place.set(Some(place));
        let from = self.ivars().size.get();
        if items.is_empty() {
            if let Some(from) = from {
                self.animate(Motion { from, to: place.notch, opening: false, fresh: false });
            }
            return;
        }
        let (content, target) =
            if expanded { self.cards(mtm, &items, place.top) } else { self.pill(mtm, &items, place) };
        if let Some(old) = self.ivars().content.replace(Some(content.clone())) {
            old.removeFromSuperview();
        }
        if let Some(background) = panel.contentView() {
            background.addSubview(&content);
        }
        self.ivars().target.set(target);
        panel.orderFrontRegardless();
        self.blink_on();
        self.animate(Motion { from: from.unwrap_or(place.notch), to: target, opening: true, fresh: from.is_none() });
    }

    fn fresh_content(&self, mtm: MainThreadMarker, target: Size) -> Retained<NSView> {
        self.ivars().generation.set(self.ivars().generation.get() + 1);
        self.ivars().clocks.borrow_mut().clear();
        self.ivars().mascots.borrow_mut().clear();
        unsafe { msg_send![NSView::alloc(mtm), initWithFrame: rect(0.0, 0.0, target.width, target.height)] }
    }

    fn pill(&self, mtm: MainThreadMarker, items: &[Item], place: Place) -> (Retained<NSView>, Size) {
        let below = if place.top > 0.0 { place.top } else { NO_NOTCH_TOP };
        let width = (place.notch.width + 2.0 * PILL_WINGS).clamp(PILL_MIN, WIDTH);
        let target = Size { width, height: below + PILL_ROW };
        let content = self.fresh_content(mtm, target);
        let first = &items[0];
        let view = shape(mtm, rect(0.0, 0.0, width, PILL_ROW), None, 0.0);
        self.clickable(&view, (Press::Open(first.card.pane), Box::new(first.card.clone())), None);
        let mut used = Vec::new();
        let sprite = self.sprite(mtm, (first.card.pane, first.mood()), 1.0, &mut used);
        let middle = (PILL_ROW - PILL_MASCOT) / 2.0;
        sprite.setFrameOrigin(NSPoint::new(PILL_SIDE, middle));
        view.addSubview(&sprite);
        let mut right = width - PILL_SIDE;
        if let Some(more) = card::more(items.len()) {
            let field = label(mtm, &more, 11.0, Weight::Semibold, SUBTLE);
            let room = field.fittingSize().width;
            right -= room;
            field.setFrame(rect(right, (PILL_ROW - LINE) / 2.0, room, LINE));
            view.addSubview(&field);
            right -= PILL_GAP;
        }
        let x = PILL_SIDE + PILL_MASCOT + PILL_GAP;
        let line = first.outcome.as_ref().map_or_else(|| card::summary(&first.card), |outcome| outcome.line().0);
        let field = label(mtm, &line, 11.5, Weight::Medium, WHITE);
        field.setFrame(rect(x, (PILL_ROW - LINE) / 2.0, right - x, LINE));
        view.addSubview(&field);
        content.addSubview(&view);
        self.ivars().moods.borrow_mut().retain(|key, _| used.contains(key));
        (content, target)
    }

    fn cards(&self, mtm: MainThreadMarker, items: &[Item], top: f64) -> (Retained<NSView>, Size) {
        let shown = &items[..items.len().min(MAX_SHOWN)];
        let more = items.len() - shown.len();
        let heights: Vec<f64> = shown.iter().map(|item| card_height(mtm, item)).collect();
        let below = if top > 0.0 { top + BELOW_NOTCH } else { NO_NOTCH_TOP };
        let gaps = GAP * f64::from(u32::try_from(heights.len().saturating_sub(1)).unwrap_or(0));
        let extra = if more > 0 { GAP + MORE } else { 0.0 };
        let target = Size { width: WIDTH, height: below + gaps + heights.iter().sum::<f64>() + extra + BOTTOM };
        let content = self.fresh_content(mtm, target);
        let mut used = Vec::new();
        let mut y = target.height - below + GAP;
        for (item, height) in shown.iter().zip(heights) {
            y -= GAP + height;
            let view = self.full(mtm, item, rect(SIDE, y, WIDTH - 2.0 * SIDE, height), &mut used);
            content.addSubview(&view);
        }
        if more > 0 {
            let field = label(mtm, &format!("+{more} more"), 10.0, Weight::Medium, SUBTLE);
            field.setAlignment(NSTextAlignment::Center);
            field.setFrame(rect(SIDE, y - GAP - MORE, WIDTH - 2.0 * SIDE, MORE));
            content.addSubview(&field);
        }
        self.ivars().moods.borrow_mut().retain(|key, _| used.contains(key));
        self.update_clocks();
        (content, target)
    }

    fn sprite(
        &self,
        mtm: MainThreadMarker,
        key: (u64, Mood),
        pixel: f64,
        used: &mut Vec<(u64, Mood)>,
    ) -> Retained<Sprite> {
        let since = *self.ivars().moods.borrow_mut().entry(key).or_insert_with(Instant::now);
        used.push(key);
        let side = pixel * f64::from(u32::try_from(rinco::SIZE).unwrap_or(0));
        let ivars = SpriteIvars {
            mood: Cell::new(key.1),
            since: Cell::new(since),
            frame: Cell::new(None),
            pixel: Cell::new(pixel),
        };
        let view = Sprite::alloc(mtm).set_ivars(ivars);
        let view: Retained<Sprite> = unsafe { msg_send![super(view), initWithFrame: rect(0.0, 0.0, side, side)] };
        self.ivars().mascots.borrow_mut().push(view.clone());
        view
    }

    fn clickable(&self, shape: &Shape, click: Click, hover: Option<u32>) {
        shape.ivars().click.replace(Some(click));
        shape.ivars().generation.set(self.ivars().generation.get());
        let _ = shape.ivars().notch.set(self.retain());
        if let Some(hover) = hover {
            shape.ivars().hover.set(Some(hover));
            let options = NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect;
            let area = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    shape.bounds(),
                    options,
                    Some(shape),
                    None,
                )
            };
            shape.addTrackingArea(&area);
        }
    }

    fn full(&self, mtm: MainThreadMarker, item: &Item, frame: NSRect, used: &mut Vec<(u64, Mood)>) -> Retained<Shape> {
        let card = &item.card;
        let view = shape(mtm, frame, Some(CARD_FILL), CARD_RADIUS);
        self.clickable(&view, (Press::Open(card.pane), Box::new(card.clone())), None);
        let width = frame.size.width;
        let top = frame.size.height - CARD_V;
        let mut x = CARD_H;
        let sprite = self.sprite(mtm, (card.pane, item.mood()), 2.0, used);
        sprite.setFrameOrigin(NSPoint::new(x, top - MASCOT));
        view.addSubview(&sprite);
        x += MASCOT + MASCOT_GAP;
        let right = self.time(mtm, &view, card, width - CARD_H, top - TITLE);
        let title = label(mtm, &card.title, 13.0, Weight::Bold, WHITE);
        title.setFrame(rect(x, top - TITLE, right - x, TITLE));
        view.addSubview(&title);
        let (line, tint, mono) = item.outcome.as_ref().map_or_else(
            || {
                let (line, tint) = card::status(card);
                (line, tint, false)
            },
            |outcome| {
                let (line, tint) = outcome.line();
                (line, tint, outcome.monospaced())
            },
        );
        let mut y = top - TITLE - LINE_GAP - LINE;
        let field =
            if mono { mono_label(mtm, &line, 11.0, tint) } else { label(mtm, &line, 11.5, Weight::Regular, tint) };
        field.setFrame(rect(x, y, width - CARD_H - x, LINE));
        view.addSubview(&field);
        let mut fits = true;
        if let Some(command) = card::command(card).filter(|_| item.outcome.is_none()) {
            let block = command_block(mtm, command);
            y -= LINE_GAP + block.height;
            block.field.setFrame(rect(x, y, TEXT_WIDTH, block.height));
            view.addSubview(&block.field);
            fits = block.fits;
        }
        y -= LINE_GAP + PLACE;
        let place = label(mtm, &card::subtitle(card), 10.0, Weight::Regular, SUBTLE);
        place.setFrame(rect(x, y, width - CARD_H - x, PLACE));
        view.addSubview(&place);
        if item.outcome.is_none() {
            self.pills(mtm, &view, card, (x, width - CARD_H, fits), y - PILLS_TOP - PILL);
        }
        view
    }

    fn pills(
        &self,
        mtm: MainThreadMarker,
        parent: &NSView,
        card: &Card,
        (from, right, fits): (f64, f64, bool),
        y: f64,
    ) {
        let mut x = from;
        for button in card::buttons(card, fits) {
            let field = label(mtm, &button.label, 10.0, Weight::Bold, button.text);
            let pad = if matches!(button.press, Press::Answer(_)) { PILL_PAD } else { OPEN_PAD };
            let width = (field.fittingSize().width + 2.0 * pad).min(right - x);
            if width <= 2.0 * pad {
                break;
            }
            let pill = shape(mtm, rect(x, y, width, PILL), Some(button.fill), PILL / 2.0);
            self.clickable(&pill, (button.press, Box::new(card.clone())), Some(lighter(button.fill)));
            field.setAlignment(NSTextAlignment::Center);
            field.setFrame(rect(0.0, (PILL - PLACE) / 2.0, width, PLACE));
            pill.addSubview(&field);
            parent.addSubview(&pill);
            x += width + PILLS_GAP;
        }
    }

    fn time(&self, mtm: MainThreadMarker, parent: &NSView, card: &Card, right: f64, y: f64) -> f64 {
        let Some(since) = card.since else { return right };
        let field = label(mtm, &card::ago(elapsed(since)), 10.0, Weight::Regular, TIME_TEXT);
        field.setAlignment(NSTextAlignment::Right);
        field.setFrame(rect(right - TIME, y + (TITLE - PLACE) / 2.0, TIME, PLACE));
        parent.addSubview(&field);
        self.ivars().clocks.borrow_mut().push((field, since));
        right - TIME - META_GAP
    }

    fn update_clocks(&self) {
        for (field, since) in self.ivars().clocks.borrow().iter() {
            let shown = card::ago(elapsed(*since));
            if field.stringValue().to_string() != shown {
                field.setStringValue(&NSString::from_str(&shown));
            }
        }
    }

    fn blink_on(&self) {
        let mut blinking = self.ivars().blinking.borrow_mut();
        if blinking.is_none() {
            *blinking = Some(unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    MASCOT_EVERY,
                    self,
                    sel!(blink:),
                    None,
                    true,
                )
            });
        }
    }

    fn animate(&self, motion: Motion) {
        let now = Instant::now();
        let reduce = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        let started = if reduce { now.checked_sub(SETTLE).unwrap_or(now) } else { now };
        self.ivars().motion.set(Some((motion, started)));
        self.step();
        let mut animating = self.ivars().animating.borrow_mut();
        if animating.is_none() && self.ivars().motion.get().is_some() {
            *animating = Some(unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    FRAME_EVERY,
                    self,
                    sel!(frame:),
                    None,
                    true,
                )
            });
        }
    }

    fn step(&self) {
        let Some((motion, started)) = self.ivars().motion.get() else { return };
        let frame = motion.at(started.elapsed());
        self.apply(frame);
        if !frame.done {
            return;
        }
        self.ivars().motion.set(None);
        if let Some(timer) = self.ivars().animating.borrow_mut().take() {
            timer.invalidate();
        }
        let empty = self.ivars().shown.borrow().as_ref().is_none_or(|shown| shown.items.is_empty());
        if !motion.opening && empty {
            self.ivars().size.set(None);
            self.ivars().mascots.borrow_mut().clear();
            if let Some(timer) = self.ivars().blinking.borrow_mut().take() {
                timer.invalidate();
            }
            if let Some(content) = self.ivars().content.take() {
                content.removeFromSuperview();
            }
            if let Some(panel) = self.ivars().panel.get() {
                panel.orderOut(None);
            }
        }
    }

    fn apply(&self, frame: Frame) {
        let (Some(panel), Some(place)) = (self.ivars().panel.get(), self.ivars().place.get()) else { return };
        let Frame { size, alpha, .. } = frame;
        let screen = place.screen;
        let x = screen.origin.x + (screen.size.width - size.width) / 2.0;
        let y = screen.origin.y + screen.size.height - size.height;
        panel.setFrame_display(rect(x, y, size.width, size.height + RADIUS), true);
        panel.invalidateShadow();
        if let Some(layer) = panel.contentView().and_then(|view| view.layer()) {
            layer.setCornerRadius(RADIUS.min(f64::midpoint(size.height, RADIUS)));
        }
        self.ivars().size.set(Some(size));
        if let Some(content) = self.ivars().content.borrow().as_ref() {
            let target = self.ivars().target.get();
            content.setFrameOrigin(NSPoint::new((size.width - target.width) / 2.0, size.height - target.height));
            content.setAlphaValue(alpha);
        }
    }
}

struct Command {
    field: Retained<NSTextField>,
    height: f64,
    fits: bool,
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a small, positive count of lines")]
fn command_block(mtm: MainThreadMarker, text: &str) -> Command {
    let font = NSFont::monospacedSystemFontOfSize_weight(COMMAND_SIZE, Weight::Regular.value());
    let wrapping = |shown: &str| {
        let field = NSTextField::wrappingLabelWithString(&NSString::from_str(shown), mtm);
        field.setFont(Some(&font));
        field.setTextColor(Some(&colour(WHITE, 1.0)));
        field.setLineBreakMode(NSLineBreakMode::ByWordWrapping);
        field.setPreferredMaxLayoutWidth(TEXT_WIDTH);
        field
    };
    let line = wrapping("A").fittingSize().height.max(1.0);
    let field = wrapping(text);
    let lines = (field.fittingSize().height / line).round().clamp(1.0, 99.0) as usize;
    let shown = lines.min(card::COMMAND_LINES);
    field.setMaximumNumberOfLines(isize::try_from(shown).unwrap_or(1));
    field.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    Command { field, height: line * f64::from(u32::try_from(shown).unwrap_or(1)), fits: lines <= card::COMMAND_LINES }
}

fn card_height(mtm: MainThreadMarker, item: &Item) -> f64 {
    let place = LINE_GAP + PLACE;
    let command = card::command(&item.card).filter(|_| item.outcome.is_none());
    let place = place + command.map_or(0.0, |command| LINE_GAP + command_block(mtm, command).height);
    let pills = if item.outcome.is_none() { PILLS_TOP + PILL } else { 0.0 };
    let column = TITLE + LINE_GAP + LINE + place + pills;
    2.0 * CARD_V + column.max(MASCOT)
}

fn lighter(rgb: u32) -> u32 {
    let lift = |shift: u32| (((rgb >> shift) & 0xff) + 0x14).min(0xff) << shift;
    lift(16) | lift(8) | lift(0)
}

fn colour(rgb: u32, alpha: f64) -> Retained<NSColor> {
    let part = |shift: u32| f64::from((rgb >> shift) & 0xff) / 255.0;
    NSColor::colorWithSRGBRed_green_blue_alpha(part(16), part(8), part(0), alpha)
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn elapsed(since: u64) -> u64 {
    u64::try_from(now()).unwrap_or(0).saturating_sub(since)
}

fn shape(mtm: MainThreadMarker, frame: NSRect, fill: Option<u32>, radius: f64) -> Retained<Shape> {
    let ivars = ShapeIvars { fill: Cell::new(fill), radius: Cell::new(radius), ..ShapeIvars::default() };
    let view = Shape::alloc(mtm).set_ivars(ivars);
    unsafe { msg_send![super(view), initWithFrame: frame] }
}

fn label(mtm: MainThreadMarker, shown: &str, size: f64, weight: Weight, rgb: u32) -> Retained<NSTextField> {
    styled(mtm, shown, &NSFont::systemFontOfSize_weight(size, weight.value()), rgb)
}

fn mono_label(mtm: MainThreadMarker, shown: &str, size: f64, rgb: u32) -> Retained<NSTextField> {
    styled(mtm, shown, &NSFont::monospacedSystemFontOfSize_weight(size, Weight::Regular.value()), rgb)
}

fn styled(mtm: MainThreadMarker, shown: &str, font: &NSFont, rgb: u32) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(shown), mtm);
    field.setFont(Some(font));
    field.setTextColor(Some(&colour(rgb, 1.0)));
    field.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    field
}

fn make_panel(mtm: MainThreadMarker) -> Retained<NSPanel> {
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        rect(0.0, 0.0, WIDTH, SIDE),
        style,
        NSBackingStoreType::Buffered,
        false,
    );
    unsafe { panel.setReleasedWhenClosed(false) };
    panel.setFloatingPanel(true);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setHidesOnDeactivate(false);
    panel.setLevel(NSStatusWindowLevel + 1);
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(true);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    let background: Retained<Background> =
        unsafe { msg_send![Background::alloc(mtm), initWithFrame: rect(0.0, 0.0, WIDTH, SIDE)] };
    background.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    background.setWantsLayer(true);
    if let Some(layer) = background.layer() {
        layer.setMasksToBounds(true);
        layer.setCornerRadius(RADIUS);
    }
    panel.setContentView(Some(&background));
    panel
}

fn place(mtm: MainThreadMarker) -> Place {
    let small = Size { width: NO_NOTCH_WIDTH, height: NO_NOTCH_HEIGHT };
    let screens = NSScreen::screens(mtm);
    let screen = screens.iter().find(|s| s.safeAreaInsets().top > 0.0).or_else(|| NSScreen::mainScreen(mtm));
    let Some(screen) = screen else {
        return Place { screen: rect(0.0, 0.0, WIDTH, WIDTH), top: 0.0, notch: small };
    };
    let frame = screen.frame();
    let top = screen.safeAreaInsets().top;
    let sides = screen.auxiliaryTopLeftArea().size.width + screen.auxiliaryTopRightArea().size.width;
    let notch = if top > 0.0 && sides > 0.0 {
        Size { width: (frame.size.width - sides).clamp(NO_NOTCH_WIDTH, WIDTH), height: top }
    } else {
        small
    };
    Place { screen: frame, top, notch }
}

pub(crate) fn run(shared: Arc<Shared>, terminal: Option<i32>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let notch = Notch::new(mtm, shared, terminal);
    app.setDelegate(Some(ProtocolObject::from_ref(&*notch)));
    app.run();
}
