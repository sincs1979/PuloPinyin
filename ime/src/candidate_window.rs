//! Two-row × five-column candidate panel: translucent, rounded, no chrome.

use engine::Candidate;
use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSBackingStoreType,
    NSColor, NSFocusRingType, NSFont, NSForegroundColorAttributeName, NSPanel, NSScreen,
    NSTextAlignment, NSTextField, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowStyleMask,
};
use objc2_foundation::{
    NSMutableAttributedString, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use std::cell::Cell;

const COLS: usize = 5;
const ROWS: usize = 2;
const CELL_W: f64 = 108.0;
const CELL_H: f64 = 28.0;
const PAD_X: f64 = 12.0;
const PAD_Y: f64 = 10.0;
const GAP_X: f64 = 2.0;
const GAP_Y: f64 = 2.0;
const RADIUS: f64 = 12.0;
const PANEL_ALPHA: f64 = 0.70;
const INDEX_GRAY: f64 = 0x77 as f64 / 255.0;

const LABELS: [char; 10] = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];

pub struct PanelIvars;

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "BuluoCandidatePanel"]
    #[ivars = PanelIvars]
    struct NonKeyPanel;

    unsafe impl NSObjectProtocol for NonKeyPanel {}

    impl NonKeyPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            false
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            false
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            false
        }
    }
);

pub struct CandidatePanel {
    panel: Retained<NonKeyPanel>,
    glass: Retained<NSVisualEffectView>,
    cells: Vec<Retained<NSTextField>>,
    last_caret: Cell<NSRect>,
}

fn panel_size() -> NSSize {
    NSSize::new(
        PAD_X * 2.0 + COLS as f64 * CELL_W + (COLS as f64 - 1.0) * GAP_X,
        PAD_Y * 2.0 + ROWS as f64 * CELL_H + GAP_Y,
    )
}

fn make_cell(mtm: MainThreadMarker) -> Retained<NSTextField> {
    let field = NSTextField::new(mtm);
    field.setEditable(false);
    field.setSelectable(false);
    field.setBezeled(false);
    field.setBordered(false);
    field.setDrawsBackground(false);
    field.setUsesSingleLineMode(true);
    field.setRefusesFirstResponder(true);
    field.setFocusRingType(NSFocusRingType::None);
    field.setAlignment(NSTextAlignment::Left);
    field.setFont(Some(&NSFont::systemFontOfSize(14.0)));
    field.setTextColor(Some(&NSColor::whiteColor()));
    field
}

impl CandidatePanel {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let size = panel_size();
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), size);
        let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
        let panel: Retained<NonKeyPanel> = unsafe {
            let this = mtm.alloc::<NonKeyPanel>().set_ivars(PanelIvars);
            msg_send![
                super(this),
                initWithContentRect: frame,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ]
        };
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setHidesOnDeactivate(false);
        panel.setIgnoresMouseEvents(true);
        panel.setOpaque(false);
        panel.setHasShadow(true);
        panel.setLevel(110);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        if let Some(dark) = NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }) {
            panel.setAppearance(Some(&dark));
        }
        panel.setCollectionBehavior(
            objc2_app_kit::NSWindowCollectionBehavior::CanJoinAllSpaces
                | objc2_app_kit::NSWindowCollectionBehavior::FullScreenAuxiliary,
        );

        let glass = NSVisualEffectView::initWithFrame(mtm.alloc::<NSVisualEffectView>(), frame);
        glass.setMaterial(NSVisualEffectMaterial::HUDWindow);
        glass.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        frame_glass_corners(&glass);
        glass.setState(NSVisualEffectState::Active);
        glass.setAlphaValue(PANEL_ALPHA);

        let mut cells = Vec::with_capacity(COLS * ROWS);
        for row in 0..ROWS {
            for col in 0..COLS {
                let cell = make_cell(mtm);
                let x = PAD_X + col as f64 * (CELL_W + GAP_X);
                let y = PAD_Y + (ROWS - 1 - row) as f64 * (CELL_H + GAP_Y);
                cell.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(CELL_W, CELL_H)));
                cells.push(cell);
            }
        }

        if let Some(content) = panel.contentView() {
            content.setWantsLayer(true);
            content.setFocusRingType(NSFocusRingType::None);
            content.addSubview(&glass);
            for cell in &cells {
                content.addSubview(cell);
            }
        }

        Self {
            panel,
            glass,
            cells,
            last_caret: Cell::new(NSRect::new(NSPoint::new(80.0, 80.0), NSSize::new(1.0, 16.0))),
        }
    }

    pub fn hide(&self) {
        self.panel.orderOut(None);
        self.last_caret.set(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(0.0, 0.0),
        ));
    }

    pub fn show(&self, candidates: &[Candidate], caret: NSRect) {
        if candidates.is_empty() {
            self.hide();
            return;
        }

        for (i, cell) in self.cells.iter().enumerate() {
            if let Some(c) = candidates.get(i) {
                cell.setAttributedStringValue(&labeled_word(i, &c.word));
                cell.setHidden(false);
            } else {
                cell.setStringValue(&NSString::from_str(""));
                cell.setHidden(true);
            }
        }

        let caret = if usable_anchor(caret) {
            caret
        } else if usable_anchor(self.last_caret.get()) {
            self.last_caret.get()
        } else {
            caret
        };
        if usable_anchor(caret) {
            self.last_caret.set(caret);
        }

        let size = panel_size();
        let origin = place_origin(caret, size);
        let win = NSRect::new(origin, size);
        self.panel.setFrame_display(win, true);
        self.glass
            .setFrame(NSRect::new(NSPoint::new(0.0, 0.0), size));
        frame_glass_corners(&self.glass);
        let _ = self.panel.makeFirstResponder(None);
        self.panel.orderFrontRegardless();
    }
}

fn usable_anchor(rect: NSRect) -> bool {
    rect.origin.x.is_finite()
        && rect.origin.y.is_finite()
        && rect.origin.x.abs() < 20_000.0
        && rect.origin.y.abs() < 20_000.0
        && (rect.size.width.abs() > 0.5
            || rect.size.height.abs() > 0.5
            || rect.origin.x.abs() > 1.0
            || rect.origin.y.abs() > 1.0)
}

fn screen_visible_frame(point: NSPoint) -> NSRect {
    let mtm = MainThreadMarker::new().expect("IME on main thread");
    let screens = NSScreen::screens(mtm);
    let count = screens.count();
    for i in 0..count {
        let screen = screens.objectAtIndex(i);
        let vis = screen.visibleFrame();
        if point_in_rect(point, vis) {
            return vis;
        }
    }
    NSScreen::mainScreen(mtm)
        .map(|s| s.visibleFrame())
        .unwrap_or(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(1440.0, 900.0),
        ))
}

fn point_in_rect(p: NSPoint, r: NSRect) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.width
        && p.y <= r.origin.y + r.size.height
}

fn place_origin(caret: NSRect, size: NSSize) -> NSPoint {
    let vis = screen_visible_frame(NSPoint::new(
        caret.origin.x,
        caret.origin.y + caret.size.height * 0.5,
    ));
    let gap = 4.0;
    let margin = 8.0;
    // Sit just under the blinking caret; flip above only if it would clip.
    let mut x = caret.origin.x;
    let mut y = caret.origin.y - gap - size.height;
    if y < vis.origin.y + margin {
        y = caret.origin.y + caret.size.height.max(16.0) + gap;
    }
    let max_x = vis.origin.x + vis.size.width - size.width - margin;
    let min_x = vis.origin.x + margin;
    if x > max_x {
        x = max_x.max(min_x);
    }
    if x < min_x {
        x = min_x;
    }
    let max_y = vis.origin.y + vis.size.height - size.height - margin;
    let min_y = vis.origin.y + margin;
    if y > max_y {
        y = max_y.max(min_y);
    }
    if y < min_y {
        y = min_y;
    }
    NSPoint::new(x, y)
}

fn labeled_word(index: usize, word: &str) -> Retained<NSMutableAttributedString> {
    let text = format!("{} {}", LABELS[index], word);
    let ns = NSString::from_str(&text);
    let attr = NSMutableAttributedString::initWithString(
        NSMutableAttributedString::alloc(),
        &ns,
    );
    let gray =
        NSColor::colorWithSRGBRed_green_blue_alpha(INDEX_GRAY, INDEX_GRAY, INDEX_GRAY, 1.0);
    let white = NSColor::whiteColor();
    let full = NSRange {
        location: 0,
        length: ns.length(),
    };
    unsafe {
        let _: () = msg_send![
            &*attr,
            addAttribute: NSForegroundColorAttributeName,
            value: &*white,
            range: full
        ];
        let _: () = msg_send![
            &*attr,
            addAttribute: NSForegroundColorAttributeName,
            value: &*gray,
            range: NSRange {
                location: 0,
                length: 1
            }
        ];
    }
    attr
}

fn frame_glass_corners(glass: &NSVisualEffectView) {
    glass.setWantsLayer(true);
    if let Some(layer) = glass.layer() {
        layer.setCornerRadius(RADIUS);
        layer.setMasksToBounds(true);
    }
}
