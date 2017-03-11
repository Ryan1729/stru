#![feature(link_args)]
#![feature(drop_types_in_const)]

#![allow(non_upper_case_globals)]

extern crate libc;
use libc::*;

extern crate x11;
use x11::xlib;
use x11::xft;
use x11::xrender;

extern crate fontconfig;
use fontconfig::fontconfig::*;

extern crate errno;
use errno::errno;

use std::ffi::CString;
use std::mem;
use std::ptr;
use std::cmp::max;

mod config;

// "the `link_args` attribute is not portable across platforms" but that's fine,
// I just need it for the purposes of the port and only until I can move everything
// over to the rust X11 bindings
#[link_args = "-L/usr/lib -lc -L/usr/X11R6/lib -lm -lrt -lX11 -lutil -lXft -lfontconfig -lfreetype"]
extern "C" {
    fn st_main(argc: c_int,
               argv: *const *const c_char,
               opt_title: *const c_char,
               opt_class: *const c_char,
               opt_io: *const c_char,
               opt_line: *const c_char,
               opt_name: *const c_char)
               -> c_int;

    fn run_step(ev: xlib::XEvent,
                xfd: libc::c_int,
                xev: libc::c_uint,
                blinkset: libc::c_int,
                dodraw: libc::c_int,
                drawtimeout: libc::timespec,
                tv: *mut libc::timespec,
                now: libc::timespec,
                last: libc::timespec,
                lastblink: libc::timespec,
                deltatime: libc::c_long,
                rfd: libc::fd_set);

    fn xloadfonts(fontstr: *const c_char, fontsize: c_double);

    fn ttynew();
    fn ttyresize();
    fn ttyread() -> size_t;

    fn tsetdirt(top: c_int, bot: c_int);
    fn tresize(col: c_int, row: c_int);
    fn tclearregion(x1: c_int, y1: c_int, x2: c_int, y2: c_int);

    fn cresize(width: c_int, height: c_int);
}

// 8888ba.88ba
// 88  `8b  `8b
// 88   88   88 .d8888b. .d8888b. 88d888b. .d8888b. .d8888b.
// 88   88   88 88'  `88 88'  `"" 88'  `88 88'  `88 Y8ooooo.
// 88   88   88 88.  .88 88.  ... 88       88.  .88       88
// dP   dP   dP `88888P8 `88888P' dP       `88888P' `88888P'


macro_rules! arg_set {
    ( CString $target:ident, $args:ident, $cmd_start:ident, $len:ident, $exe_path:expr) => {{
        $cmd_start += 1;
        if $cmd_start < $len {
            $target = Some(CString::new($args.remove($cmd_start)).unwrap());

            $cmd_start -= 1;
            $args.remove($cmd_start); //remove the flag
            $len = $args.len();
        } else {
            usage($exe_path)
        }
    }};

    ( $target:ident, $args:ident, $cmd_start:ident, $len:ident, $exe_path:expr) => {{
        $cmd_start += 1;
        if $cmd_start < $len {
            $target = Some($args.remove($cmd_start));

            $cmd_start -= 1;
            $args.remove($cmd_start); //remove the flag
            $len = $args.len();
        } else {
            usage($exe_path)
        }
    }}
}

//adapted from https://github.com/rust-lang/rfcs/issues/1078
macro_rules! die {
    ($fmt:expr) => {{ use std::io::Write;
            if let Err(e) = write!(&mut std::io::stderr(), $fmt) {
                panic!("Failed to write to stderr.\
                    \nOriginal error output: {}\
                    \nSecondary error writing to stderr: {}", $fmt, e);
            }
            std::process::exit(1);
        }};
    ($fmt:expr, $($arg:tt)*) => {{ use std::io::Write;
            if let Err(e) = write!(&mut std::io::stderr(), $fmt, $($arg)*) {
                panic!("Failed to write to stderr.\
                    \nOriginal error output: {}\
                    \nSecondary error writing to stderr: {}", format!($fmt, $($arg)*), e);
            }
            std::process::exit(1);
        }};
}

macro_rules! limit {
    ( $input: expr, $min : expr, $max: expr ) => {
        if $input < $min {
            $min
        } else if $input > $max {
            $max
        } else {
            $input
        }
    }
}

macro_rules! is_set_on {
    ( $flag: expr, $field : expr) => {
        $field & $flag != 0
    };

    ( $flag: expr, $field : expr, $number_type:ty) => {
        $field & ($flag as $number_type) != 0
    }
}

macro_rules! mod_bit {
    ( $x: expr, $set : expr, $bit : expr) => {
        if $set != 0 {
            $x |= $bit;
        } else {
            $x &= !$bit;
        };
    }
}

macro_rules! is_between {
    ( $x: expr, $min : expr, $max : expr) => {
        $min <= $x && $x <= $max
    }
}

macro_rules! new {
    (TCursor) => {
        TCursor {
            attr: Glyph {
                u: 0,
                mode: ATTR_NULL as u16,
                fg: config::defaultfg,
                bg: config::defaultbg,
            },
            x: 0,
            y: 0,
            state: CURSOR_DEFAULT as c_char,
        }
    };

    (Font) => {
        Font {
            height: 0,
            width: 0,
            ascent: 0,
            descent: 0,
            lbearing: 0,
            rbearing: 0,
            match_: 0 as *mut xft::XftFont,
            set: 0 as *mut FcFontSet,
            pattern: 0 as *mut FcPattern,
        }
    };

    (Coords) => {
        Coords {
            x : 0,
            y : 0
        }
    };

    (libc::timespec) => {
        libc::timespec {
            tv_sec : 0,
            tv_nsec : 0
        }
    };

    (xlib::XColor) => {
        xlib::XColor {
    pixel: 0,
    red: 0,
    green: 0,
    blue: 0,
    flags: 0,
    pad: 0,
}
    };
}

// .d88888b  dP                                        dP
// 88.    "' 88                                        88
// `Y88888b. 88d888b. .d8888b. 88d888b. .d8888b. .d888b88
//       `8b 88'  `88 88'  `88 88'  `88 88ooood8 88'  `88
// d8'   .8P 88    88 88.  .88 88       88.  ... 88.  .88
//  Y88888P  dP    dP `88888P8 dP       `88888P' `88888P8


#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum cursor_state {
    CURSOR_DEFAULT = 0,
    CURSOR_WRAPNEXT = 1,
    CURSOR_ORIGIN = 2,
}
use cursor_state::*;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum selection_mode {
    SEL_IDLE = 0,
    SEL_EMPTY = 1,
    SEL_READY = 2,
}
use selection_mode::*;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum term_mode {
    MODE_WRAP = 1 << 0,
    MODE_INSERT = 1 << 1,
    MODE_APPKEYPAD = 1 << 2,
    MODE_ALTSCREEN = 1 << 3,
    MODE_CRLF = 1 << 4,
    MODE_MOUSEBTN = 1 << 5,
    MODE_MOUSEMOTION = 1 << 6,
    MODE_REVERSE = 1 << 7,
    MODE_KBDLOCK = 1 << 8,
    MODE_HIDE = 1 << 9,
    MODE_ECHO = 1 << 10,
    MODE_APPCURSOR = 1 << 11,
    MODE_MOUSESGR = 1 << 12,
    MODE_8BIT = 1 << 13,
    MODE_BLINK = 1 << 14,
    MODE_FBLINK = 1 << 15,
    MODE_FOCUS = 1 << 16,
    MODE_MOUSEX10 = 1 << 17,
    MODE_MOUSEMANY = 1 << 18,
    MODE_BRCKTPASTE = 1 << 19,
    MODE_PRINT = 1 << 20,
    MODE_MOUSE = MODE_MOUSEBTN as isize
        |MODE_MOUSEMOTION as isize
        |MODE_MOUSEX10 as isize
        |MODE_MOUSEMANY as isize,
}
use term_mode::*;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum glyph_attribute {
    ATTR_NULL = 0,
    ATTR_BOLD = 1 << 0,
    ATTR_FAINT = 1 << 1,
    ATTR_ITALIC = 1 << 2,
    ATTR_UNDERLINE = 1 << 3,
    ATTR_BLINK = 1 << 4,
    ATTR_REVERSE = 1 << 5,
    ATTR_INVISIBLE = 1 << 6,
    ATTR_STRUCK = 1 << 7,
    ATTR_WRAP = 1 << 8,
    ATTR_WIDE = 1 << 9,
    ATTR_WDUMMY = 1 << 10,
    ATTR_BOLD_FAINT = ATTR_BOLD as isize | ATTR_FAINT as isize,
}
use glyph_attribute::*;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum charset {
    CS_GRAPHIC0 = 0,
    CS_GRAPHIC1 = 1,
    CS_UK = 2,
    CS_USA = 3,
    CS_MULTI = 4,
    CS_GER = 5,
    CS_FIN = 6,
}
use charset::*;

static mut CURSOR_STORAGE: [TCursor; 2] = [new!(TCursor), new!(TCursor)];

#[repr(C)]
#[allow(dead_code)]
pub struct Coords {
    x: c_int,
    y: c_int,
}

#[repr(C)]
#[allow(dead_code)]
pub struct Selection {
    mode: c_int,
    type_: c_int,
    snap: c_int,
    /*
     * Selection variables:
     * nb – normalized coordinates of the beginning of the selection
     * ne – normalized coordinates of the end of the selection
     * ob – original coordinates of the beginning of the selection
     * oe – original coordinates of the end of the selection
     */
    nb: Coords,
    ne: Coords,
    ob: Coords,
    oe: Coords,

    primary: *mut c_char,
    clipboard: *mut c_char,
    xtarget: xlib::Atom,
    alt: c_int,
    tclick1: libc::timespec,
    tclick2: libc::timespec,
}

#[no_mangle]
pub static mut sel: Selection = Selection {
    mode: 0,
    type_: 0,
    snap: 0,
    nb: new!(Coords),
    ne: new!(Coords),
    ob: new!(Coords),
    oe: new!(Coords),
    primary: 0 as *mut c_char,
    clipboard: 0 as *mut c_char,
    xtarget: 0,
    alt: 0,
    tclick1: new!(libc::timespec),
    tclick2: new!(libc::timespec),
};

type Color = xft::XftColor;

#[repr(C)]
#[allow(dead_code)]
struct Font {
    height: c_int,
    width: c_int,
    ascent: c_int,
    descent: c_int,
    lbearing: c_short,
    rbearing: c_short,
    match_: *mut xft::XftFont,
    set: *mut FcFontSet,
    pattern: *mut FcPattern,
}

const colours_size: usize = 258; //MAX(LEN(colorname), 256)

/* Drawing Context */
#[repr(C)]
#[allow(dead_code)]
pub struct DC {
    col: [Color; colours_size],
    font: Font,
    bfont: Font,
    ifont: Font,
    ibfont: Font,
    gc: xlib::GC,
}

#[no_mangle]
pub static mut dc: DC = DC {
    col: [Color {
        pixel: 0 as c_ulong,
        color: x11::xrender::XRenderColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
    }; colours_size],
    font: new!(Font),
    bfont: new!(Font),
    ifont: new!(Font),
    ibfont: new!(Font),
    gc: 0 as xlib::GC,
};

type Draw = *mut xft::XftDraw;

#[repr(C)]
#[allow(dead_code)]
/* Purely graphic info */
pub struct XWindow {
    dpy: *mut xlib::Display,
    cmap: xlib::Colormap,
    win: xlib::Window,
    buf: xlib::Drawable,
    xembed: xlib::Atom,
    wmdeletewin: xlib::Atom,
    netwmname: xlib::Atom,
    netwmpid: xlib::Atom,
    xim: xlib::XIM,
    xic: xlib::XIC,
    draw: Draw,
    vis: *mut xlib::Visual,
    attrs: xlib::XSetWindowAttributes,
    scr: c_int,
    isfixed: c_int, /* is fixed geometry? */
    l: c_int, /* left and top offset */
    t: c_int,
    gm: c_int, /* geometry mask */
    tw: c_int,
    th: c_int, /* tty width and height */
    w: c_uint,
    h: c_uint, /* window width and height */
    ch: c_int, /* char height */
    cw: c_int, /* char width  */
    state: c_char, /* focus, redraw, visible */
    cursor: c_int, /* cursor style */
}

#[no_mangle]
pub static mut xw: XWindow = XWindow {
    dpy: 0 as *mut xlib::Display,
    cmap: 0 as xlib::Colormap,
    win: 0 as xlib::Window,
    buf: 0 as xlib::Drawable,
    xembed: 0,
    wmdeletewin: 0,
    netwmname: 0,
    netwmpid: 0,
    xim: 0 as xlib::XIM,
    xic: 0 as xlib::XIC,
    draw: 0 as Draw,
    vis: 0 as *mut xlib::Visual,
    attrs: xlib::XSetWindowAttributes {
        background_pixmap: 0,
        background_pixel: 0,
        border_pixmap: 0,
        border_pixel: 0,
        bit_gravity: 0,
        win_gravity: 0,
        backing_store: 0,
        backing_planes: 0,
        backing_pixel: 0,
        save_under: 0,
        event_mask: 0,
        do_not_propagate_mask: 0,
        override_redirect: 0,
        colormap: 0,
        cursor: 0,
    },
    scr: 0,
    isfixed: 0,
    l: 0,
    t: 0,
    gm: 0,
    tw: 0,
    th: 0,
    w: 0,
    h: 0,
    ch: 0,
    cw: 0,
    state: 0,
    cursor: config::cursorshape,
};

pub type Rune = uint32_t;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Glyph {
    u: Rune, /* character code */
    mode: c_ushort, /* attribute flags */
    fg: uint32_t, /* foreground  */
    bg: uint32_t, /* background  */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TCursor {
    attr: Glyph, /* current char attributes */
    x: c_int,
    y: c_int,
    state: c_char,
}

#[no_mangle]
pub static mut term: Term = Term {
    row: 0,
    col: 0,
    line: 0 as *mut *mut Glyph,
    alt: 0 as *mut *mut Glyph,
    hist: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    histi: 0,
    scr: 0,
    dirty: 0,
    specbuf: 0,
    c: new!(TCursor),
    top: 0,
    bot: 0,
    mode: 0,
    esc: 0,
    trantbl: [0, 0, 0, 0],
    charset: 0,
    icharset: 0,
    numlock: 1,
    tabs: 0 as *mut c_int,
};

#[repr(C)]
#[allow(dead_code)]
pub struct Term {
    row: c_int,
    col: c_int,
    line: *mut *mut Glyph,
    alt: *mut *mut Glyph,
    hist: [usize; config::histsize],
    histi: c_int,
    scr: c_int,
    dirty: usize,
    specbuf: usize,
    c: TCursor,
    top: c_int,
    bot: c_int,
    mode: c_int,
    esc: c_int,
    trantbl: [c_char; 4],
    charset: c_int,
    icharset: c_int,
    numlock: c_int,
    tabs: *mut c_int,
}

#[no_mangle]
pub static mut cmdfd: c_int = 0;

// a88888b.    .8888b
// d8'   `88    88   "
// 88           88aaa  88d888b. .d8888b.
// 88           88     88'  `88 Y8ooooo.
// Y8.   .88    88     88    88       88
//  Y88888P'    dP     dP    dP `88888P'

#[no_mangle]
pub unsafe extern "C" fn tsavecursor() {
    let alt = is_set_on!(MODE_ALTSCREEN, term.mode, c_int) as usize;

    CURSOR_STORAGE[alt] = term.c.clone();
}

#[no_mangle]
pub unsafe extern "C" fn tloadcursor() {
    let alt = is_set_on!(MODE_ALTSCREEN, term.mode, c_int) as usize;

    term.c = CURSOR_STORAGE[alt];
    tmoveto(CURSOR_STORAGE[alt].x, CURSOR_STORAGE[alt].y);
}

#[no_mangle]
pub unsafe extern "C" fn tfulldirt() {
    tsetdirt(0, term.row - 1);
}

static mut usedfont: Option<CString> = None;

#[no_mangle]
pub unsafe extern "C" fn loadfonts(fontsize: c_double) {
    xloadfonts(to_ptr(usedfont.as_ref()), fontsize);
}

#[no_mangle]
pub unsafe extern "C" fn tswapscreen() {
    let tmp = term.line;

    term.line = term.alt;
    term.alt = tmp;

    term.mode ^= MODE_ALTSCREEN as c_int;
    tfulldirt();
}

#[no_mangle]
pub unsafe extern "C" fn treset() {

    term.c = new!(TCursor);

    libc::memset(term.tabs as *mut c_void,
                 0,
                 term.col as size_t * mem::size_of::<*mut c_int>() as size_t);

    //TODO reduce casting here
    let mut i: c_uint = config::tabspaces;
    while i < term.col as c_uint {
        ptr::write(term.tabs.offset(i as isize), 1);
        i += config::tabspaces;
    }

    term.top = 0;
    term.bot = term.row - 1;
    term.mode = MODE_WRAP as c_int;
    term.trantbl = [CS_USA as c_char, CS_USA as c_char, CS_USA as c_char, CS_USA as c_char];
    term.charset = 0;

    for _ in 0..2 {
        tmoveto(0, 0);
        tsavecursor();
        tclearregion(0, 0, term.col - 1, term.row - 1);
        tswapscreen();
    }
}

#[no_mangle]
pub unsafe extern "C" fn tmoveto(x: c_int, y: c_int) {
    let miny;
    let maxy;

    if term.c.state & (CURSOR_ORIGIN as c_char) != 0 {
        miny = term.top;
        maxy = term.bot;
    } else {
        miny = 0;
        maxy = term.row - 1;
    }
    term.c.state &= !(CURSOR_WRAPNEXT as c_char);
    term.c.x = limit!(x, 0, term.col - 1);
    term.c.y = limit!(y, miny, maxy);
}

fn sixd_to_16bit(x: c_int) -> c_ushort {
    (if x == 0 { 0 } else { 0x3737 + 0x2828 * x }) as c_ushort
}

#[no_mangle]
pub unsafe extern "C" fn xloadcolor(i: c_int, name: *const c_char, ncolor: *mut Color) -> c_int {
    let mut color: xrender::XRenderColor = mem::zeroed();
    color.alpha = 0xffff;

    if name.is_null() {
        if is_between!(i, 16, 255) {
            /* 256 color */
            if i < 6 * 6 * 6 + 16 {
                /* same colors as xterm */
                color.red = sixd_to_16bit(((i - 16) / 36) % 6);
                color.green = sixd_to_16bit(((i - 16) / 6) % 6);
                color.blue = sixd_to_16bit(((i - 16) / 1) % 6);
            } else {
                /* greyscale */
                color.red = (0x0808 + 0x0a0a * (i - (6 * 6 * 6 + 16))) as c_ushort;
                color.blue = color.red;
                color.green = color.red;
            }
            return xft::XftColorAllocValue(xw.dpy, xw.vis, xw.cmap, &color, ncolor);
        } else {
            if let Some(col_name) = get_colourname(i) {
                return xft::XftColorAllocName(xw.dpy,
                                              xw.vis,
                                              xw.cmap,
                                              CString::new(col_name).unwrap().as_ptr(),
                                              ncolor);
            } else {
                return xft::XftColorAllocName(xw.dpy, xw.vis, xw.cmap, ptr::null(), ncolor);
            }
        }
    } else {
        return xft::XftColorAllocName(xw.dpy, xw.vis, xw.cmap, name, ncolor);
    }

}

static mut loaded: bool = false;
#[no_mangle]
pub unsafe extern "C" fn xloadcols() {
    if loaded {
        let mut current = &mut dc.col[0] as *mut Color;
        for i in 0..(dc.col.len() as isize) {
            xft::XftColorFree(xw.dpy, xw.vis, xw.cmap, current.offset(i));
        }
    }

    for i in 0..(dc.col.len() as c_int) {
        if xloadcolor(i, 0 as *const c_char, &mut dc.col[i as usize] as *mut Color) == 0 {
            if let Some(name) = get_colourname(i) {
                die!("Could not allocate color {:?}\n", name);
            } else {
                die!("Could not allocate color index {}\n", i);
            }
        }
    }
    loaded = true;
}


//  a88888b.                   .8888b oo
// d8'   `88                   88   "
// 88        .d8888b. 88d888b. 88aaa  dP .d8888b.
// 88        88'  `88 88'  `88 88     88 88'  `88
// Y8.   .88 88.  .88 88    88 88     88 88.  .88
//  Y88888P' `88888P' dP    dP dP     dP `8888P88
//                                            .88
//                                        d8888P


/*
    TODO move these into config.rs once they are completely within Rust's purview
 * spaces per tab
 *
 * When you are changing this value, don't forget to adapt the »it« value in
 * the st.info and appropriately install the st.info in the environment where
 * you use this st version.
 *
 *	it#$tabspaces,
 *
 * Secondly make sure your kernel is not expanding tabs. When running `stty
 * -a` »tab0« should appear. You can tell the terminal to not expand tabs by
 *  running following command:
 *
 *	stty tabs
 */


#[no_mangle]
pub static tabspaces: c_uint = 8;



#[no_mangle]
pub static defaultcs: c_uint = 256;
#[no_mangle]
pub static defaultrcs: c_uint = 257;

/* frames per second st should at maximum draw to the screen */
#[no_mangle]
pub static xfps: c_long = 120;
#[no_mangle]
pub static actionfps: c_uint = 30;

/*
 * blinking timeout (set to 0 to disable blinking) for the terminal blinking
 * attribute.
 */
#[no_mangle]
pub static blinktimeout: c_int = 800;
#[no_mangle]
pub static colorname_total_len: c_int = (config::colorname_len + config::extra_len) as c_int;

#[no_mangle]
pub static mut allowaltscreen: c_int = 1;
pub static mut colourname: Option<Vec<*const c_char>> = None;

fn basename(path: &str) -> &str {
    path.rsplitn(2, "/").next().unwrap_or(path)
}

fn usage(exe_path: &str) {
    die!("usage:  {} [-aiv] [-c class] [-f font] [-g geometry] [-n name] [-o file]\n
        [-T title] [-t title] [-w windowid] [[-e] command [args ...]]\n
        {} [-aiv] [-c class] [-f font] [-g geometry] [-n name] [-o file]\n
        [-T title] [-t title] [-w windowid] -l line [stty_args ...]\n",
         exe_path,
         exe_path);
}


unsafe fn tattrset(attr: c_int) -> c_int {
    for i in 0..((term.row - 1) as isize) {
        for j in 0..((term.col - 1) as isize) {
            let glyph: Glyph = *(*term.line.offset(i)).offset(j);
            if is_set_on!(attr, glyph.mode, c_ushort) {
                return 1;
            }
        }
    }

    return 0;
}

fn get_colourname(i: c_int) -> Option<&'static str> {
    let index = i as usize;
    if is_between!(index, 0, config::colorname_len) {
        Some(config::colorname[index])
    } else if is_between!(index, 256, 256 + config::extra_len) {
        Some(config::colorname[index - 256])
    } else {
        None
    }
}

/*
 * Bitmask returned by XParseGeometry().  Each bit tells if the corresponding
 * value (x, y, width, height) was found in the parsed string.
 */
// const NoValue: c_int = 0x0000;
// const XValue: c_int = 0x0001;
// const YValue: c_int = 0x0002;
// const WidthValue: c_int = 0x0004;
// const HeightValue: c_int = 0x0008;
// const AllValues: c_int = 0x000F;
const XNegative: c_int = 0x0010;
const YNegative: c_int = 0x0020;

unsafe fn xinit(opt_embed: Option<String>) {
    xw.dpy = xlib::XOpenDisplay(ptr::null());

    if xw.dpy.is_null() {
        die!("Can't open display\n");
    }

    xw.scr = xlib::XDefaultScreen(xw.dpy);
    xw.vis = xlib::XDefaultVisual(xw.dpy, xw.scr);

    xw.cmap = xlib::XDefaultColormap(xw.dpy, xw.scr);

    /* Fc == fontconfig */
    if FcInit() == 0 {
        die!("Could not init fontconfig.\n");
    }

    loadfonts(0.0);

    xloadcols();

    /* adjust fixed window geometry */
    xw.w = (2 * config::borderpx + term.col * xw.cw) as c_uint;
    xw.h = (2 * config::borderpx + term.row * xw.ch) as c_uint;

    if is_set_on!(XNegative, xw.gm) {
        xw.l += xlib::XDisplayWidth(xw.dpy, xw.scr) - (xw.w as c_int) - 2;
    }
    if is_set_on!(YNegative, xw.gm) {
        xw.t += xlib::XDisplayHeight(xw.dpy, xw.scr) - (xw.h as c_int) - 2;
    }

    xw.attrs.background_pixel = dc.col[config::defaultbg as usize].pixel;
    xw.attrs.border_pixel = dc.col[config::defaultbg as usize].pixel;
    xw.attrs.bit_gravity = xlib::NorthWestGravity;
    xw.attrs.event_mask =
        xlib::FocusChangeMask | xlib::KeyPressMask | xlib::ExposureMask |
        xlib::VisibilityChangeMask | xlib::StructureNotifyMask |
        xlib::ButtonMotionMask | xlib::ButtonPressMask | xlib::ButtonReleaseMask;
    xw.attrs.colormap = xw.cmap;

    let parent;

    if let Some(embed) = opt_embed {
        if let Ok(window_id) = embed.parse::<c_ulong>() {
            parent = window_id;
        } else {
            parent = xlib::XRootWindow(xw.dpy, xw.scr);
        }
    } else {
        parent = xlib::XRootWindow(xw.dpy, xw.scr);
    }

    xw.win = xlib::XCreateWindow(xw.dpy,
                                 parent,
                                 xw.l,
                                 xw.t,
                                 xw.w,
                                 xw.h,
                                 0,
                                 xlib::XDefaultDepth(xw.dpy, xw.scr),
                                 xlib::InputOutput as c_uint,
                                 xw.vis,
                                 xlib::CWBackPixel | xlib::CWBorderPixel | xlib::CWBitGravity |
                                 xlib::CWEventMask |
                                 xlib::CWColormap,
                                 &mut xw.attrs as *mut xlib::XSetWindowAttributes);

    let mut gcvalues: xlib::XGCValues = mem::zeroed();

    dc.gc = xlib::XCreateGC(xw.dpy,
                            parent,
                            xlib::GCGraphicsExposures as c_ulong,
                            &mut gcvalues as *mut xlib::XGCValues);

    xw.buf = xlib::XCreatePixmap(xw.dpy,
                                 xw.win,
                                 xw.w,
                                 xw.h,
                                 xlib::XDefaultDepth(xw.dpy, xw.scr as c_int) as c_uint);

    xlib::XSetForeground(xw.dpy, dc.gc, dc.col[config::defaultbg as usize].pixel);
    xlib::XFillRectangle(xw.dpy, xw.buf, dc.gc, 0, 0, xw.w, xw.h);

    /* Xft rendering context */
    xw.draw = xft::XftDrawCreate(xw.dpy, xw.buf, xw.vis, xw.cmap);

    /* input methods */
    xw.xim = xlib::XOpenIM(xw.dpy,
                           0 as xlib::XrmDatabase,
                           0 as *mut c_char,
                           0 as *mut c_char);
    if xw.xim.is_null() {
        xlib::XSetLocaleModifiers(CString::new("@im=local").unwrap().as_ptr());
        xw.xim = xlib::XOpenIM(xw.dpy,
                               0 as xlib::XrmDatabase,
                               0 as *mut c_char,
                               0 as *mut c_char);
        if xw.xim.is_null() {
            xlib::XSetLocaleModifiers(CString::new("@im=").unwrap().as_ptr());
            xw.xim = xlib::XOpenIM(xw.dpy,
                                   0 as xlib::XrmDatabase,
                                   0 as *mut c_char,
                                   0 as *mut c_char);
            if xw.xim.is_null() {
                die!("XOpenIM failed. Could not open input device.\n");
            }
        }
    }

    xw.xic = xlib::XCreateIC(xw.xim,
                             CString::new(xlib::XNInputStyle).unwrap().as_ptr(),
                             xlib::XIMPreeditNothing | xlib::XIMStatusNothing,
                             CString::new(xlib::XNClientWindow).unwrap().as_ptr(),
                             xw.win,
                             CString::new(xlib::XNFocusWindow).unwrap().as_ptr(),
                             xw.win,
                             0 as *mut libc::c_void);


    if xw.xic.is_null() {
        die!("XCreateIC failed. Could not obtain input method.\n");
    }

    xw.xembed = xlib::XInternAtom(xw.dpy, CString::new("_XEMBED").unwrap().as_ptr(), 0);
    xw.wmdeletewin = xlib::XInternAtom(xw.dpy,
                                       CString::new("WM_DELETE_WINDOW").unwrap().as_ptr(),
                                       0);
    xw.netwmname = xlib::XInternAtom(xw.dpy, CString::new("_NET_WM_NAME").unwrap().as_ptr(), 0);
    xlib::XSetWMProtocols(xw.dpy, xw.win, &mut xw.wmdeletewin as *mut c_ulong, 1);

    let thispid = libc::getpid() as u32;
    // I guess this assumes this is running on a little endian machine?
    // As far as I can tell, Rust doesn't (currently) have an easy way to
    // do this conversion without knowing the endianess. Hopefully anyone
    // who cares will be able to find this by searching for "getpid"
    let pid_array = [(thispid & 0x000000FF) as u8,
                     ((thispid & 0x0000FF00) >> 8) as u8,
                     ((thispid & 0x00FF0000) >> 16) as u8,
                     ((thispid & 0xFF000000) >> 24) as u8];
    xw.netwmpid = xlib::XInternAtom(xw.dpy, CString::new("_NET_WM_PID").unwrap().as_ptr(), 0);
    xlib::XChangeProperty(xw.dpy,
                          xw.win,
                          xw.netwmpid,
                          xlib::XA_CARDINAL,
                          32,
                          xlib::PropModeReplace,
                          &pid_array as *const c_uchar,
                          1);

    let mut xmousefg = new!(xlib::XColor);
    let mut xmousebg = new!(xlib::XColor);

    /* white cursor, black outline */
    let mut cursor = xlib::XCreateFontCursor(xw.dpy, config::mouseshape as c_uint);
    xlib::XDefineCursor(xw.dpy, xw.win, cursor);

    let mut fg_result = 0;
    if let Some(fg_name) = get_colourname(config::mousefg) {
        fg_result = xlib::XParseColor(xw.dpy,
                                      xw.cmap,
                                      CString::new(fg_name).unwrap().as_ptr(),
                                      &mut xmousefg as *mut xlib::XColor)
    }
    if fg_result == 0 {
        xmousefg.red = 0xffff;
        xmousefg.green = 0xffff;
        xmousefg.blue = 0xffff;
    }

    let mut bg_result = 0;
    if let Some(bg_name) = get_colourname(config::mousebg) {
        bg_result = xlib::XParseColor(xw.dpy,
                                      xw.cmap,
                                      CString::new(bg_name).unwrap().as_ptr(),
                                      &mut xmousebg as *mut xlib::XColor)
    }
    if bg_result == 0 {
        xmousebg.red = 0x0000;
        xmousebg.green = 0x0000;
        xmousebg.blue = 0x0000;
    }


    xlib::XRecolorCursor(xw.dpy,
                         cursor,
                         &mut xmousefg as *mut xlib::XColor,
                         &mut xmousebg as *mut xlib::XColor);

}

fn main() {
    let mut args: Vec<String> = std::env::args().collect::<Vec<String>>();

    let exe_path = if args.len() > 0 {
        args.remove(0)
    } else {
        "stru".to_string()
    };

    let mut opt_title: Option<CString> = None;
    let mut opt_class: Option<CString> = None;
    let mut opt_io: Option<CString> = None;
    let mut opt_geo: Option<CString> = None;
    let mut opt_font: Option<CString> = None;
    let mut opt_line: Option<CString> = None;
    let mut opt_name: Option<CString> = None;

    let mut opt_embed: Option<String> = None;

    let mut opt_allow_alt_screen = true;
    let mut opt_is_fixed = false;

    let mut cmd_start = 0;
    let mut len = args.len();
    while cmd_start < len && args[cmd_start].starts_with("-") {
        let mut flag = args[cmd_start].split_at(1).1.to_owned();

        flag = flag.chars()
            .filter(|c| match *c {
                'v' => {
                    die!("{}
A port of st to Rust.

Original port was done from the version of st found at
https://github.com/Ryan1729/st-plus-some-patches

C version of st (c) 2010-2016 st engineers
and can be found at st.suckless.org\n",
                         exe_path)
                }
                'a' => {
                    opt_allow_alt_screen = false;
                    false
                }
                'i' => {
                    opt_is_fixed = true;
                    false
                }

                _ => true,
            })
            .collect();

        match flag.as_ref() {
            "t" | "T" => arg_set!(CString opt_title, args, cmd_start, len, &exe_path),
            "c" => arg_set!(CString opt_class, args, cmd_start, len, &exe_path),
            "o" => arg_set!(CString opt_io, args, cmd_start, len, &exe_path),
            "g" => arg_set!(CString opt_geo, args, cmd_start, len, &exe_path),
            "f" => arg_set!(CString opt_font, args, cmd_start, len, &exe_path),
            "l" => arg_set!(CString opt_line, args, cmd_start, len, &exe_path),
            "n" => arg_set!(CString opt_name, args, cmd_start, len, &exe_path),
            "w" => arg_set!(opt_embed, args, cmd_start, len, &exe_path),
            "e" => {
                cmd_start += 1;
                break;
            }
            "" => {
                args.remove(cmd_start);

                len = args.len();
            }
            _ => usage(&exe_path),
        }

    }
    let opt_cmd = args.split_at(cmd_start).1.to_owned();

    //http://stackoverflow.com/a/34379937/4496839
    // create a vector of zero terminated strings
    let zt_args = opt_cmd.iter()
        .cloned()
        .map(|arg| CString::new((*arg).to_string()).unwrap())
        .collect::<Vec<CString>>();

    // convert the strings to raw pointers
    let c_args = zt_args.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const c_char>>();

    if c_args.len() > 0 {
        if opt_title.is_none() && opt_line.is_none() {
            opt_title = opt_cmd.get(0)
                .map(|arg| CString::new((basename((*arg).as_ref())).to_string()).unwrap());
        }
    }

    unsafe {
        allowaltscreen = if opt_allow_alt_screen { 1 } else { 0 } as c_int;

        xw.isfixed = if opt_is_fixed { 1 } else { 0 } as c_int;

        let mut cols = 80;
        let mut rows = 24;

        if let Some(geo) = opt_geo {
            xw.gm = xlib::XParseGeometry(geo.as_ptr(), &mut xw.l, &mut xw.t, &mut cols, &mut rows);
        }

        tresize(max(cols as c_int, 1), max(rows as c_int, 1));
        treset();

        usedfont = if opt_font.is_some() {
            opt_font
        } else {
            Some(CString::new(config::defaultfont).unwrap())
        };

        xinit(opt_embed);

        selinit();

        st_main(c_args.len() as c_int,
                c_args.as_ptr(),
                to_ptr(opt_title.as_ref()),
                to_ptr(opt_class.as_ref()),
                to_ptr(opt_io.as_ref()),
                to_ptr(opt_line.as_ref()),
                to_ptr(opt_name.as_ref()));


        let mut ev = xlib::XEvent { pad: [0; 24] };

        let mut w = xw.w as c_int;
        let mut h = xw.h as c_int;
        /* Waiting for window mapping */
        loop {
            xlib::XNextEvent(xw.dpy, &mut ev as *mut xlib::XEvent);
            /*
             * This XFilterEvent call is required because of XOpenIM. It
             * does filter out the key event and some client message for
             * the input method too.
             */
            if xlib::XFilterEvent(&mut ev as *mut xlib::XEvent, 0) != 0 {
                continue;
            }


            let type_ = ev.get_type();
            if type_ == xlib::ConfigureNotify {
                let config_event: xlib::XConfigureEvent = From::from(ev);

                w = config_event.width;
                h = config_event.height;
            }

            if type_ == xlib::MapNotify {
                break;
            }
        }

        cresize(w, h);
        ttynew();
        ttyresize();

        run(ev);
    };

    std::process::exit(0);
}

unsafe fn run(ev: xlib::XEvent) {
    let mut xfd = xlib::XConnectionNumber(xw.dpy);
    let mut xev = 0;
    let mut blinkset = 0;
    let mut dodraw = 0;
    let mut drawtimeout = new!(libc::timespec);
    let mut tv = 0 as *mut libc::timespec;
    let mut now = new!(libc::timespec);
    let mut last = new!(libc::timespec);
    let mut lastblink = new!(libc::timespec);
    let mut deltatime = 0;
    let mut rfd = mem::zeroed();

    clock_gettime(CLOCK_MONOTONIC, &mut last as *mut libc::timespec);
    lastblink = last;

    loop {
        xev = actionfps;

        FD_ZERO(&mut rfd as *mut fd_set);
        FD_SET(cmdfd, &mut rfd as *mut fd_set);
        FD_SET(xfd, &mut rfd as *mut fd_set);

        if pselect(max(xfd, cmdfd) + 1,
                   &mut rfd as *mut fd_set,
                   0 as *mut libc::fd_set,
                   0 as *mut libc::fd_set,
                   tv,
                   0 as *const libc::sigset_t) < 0 {
            let errno_value = errno();
            if errno_value.0 == libc::EINTR {
                continue;
            }
            die!("select failed: {}\n", errno_value);
        }

        if FD_ISSET(cmdfd, &mut rfd as *mut fd_set) {
            ttyread();
            if blinktimeout != 0 {
                blinkset = tattrset(ATTR_BLINK as c_int);
                if blinkset != 0 {
                    mod_bit!(term.mode, 0, MODE_BLINK as c_int);
                }
            }
        }

        if FD_ISSET(xfd, &mut rfd as *mut fd_set) {
            xev = actionfps;
        }

        clock_gettime(CLOCK_MONOTONIC, &mut now as *mut libc::timespec);
        drawtimeout.tv_sec = 0;
        drawtimeout.tv_nsec = (1_000_000_000) / xfps;
        tv = &mut drawtimeout as *mut libc::timespec;

        run_step(ev,
                 xfd,
                 xev,
                 blinkset,
                 dodraw,
                 drawtimeout,
                 tv,
                 now,
                 last,
                 lastblink,
                 deltatime,
                 rfd);
    }
}

unsafe fn selinit() {
    libc::clock_gettime(CLOCK_MONOTONIC, &mut sel.tclick1 as *mut libc::timespec);
    libc::clock_gettime(CLOCK_MONOTONIC, &mut sel.tclick2 as *mut libc::timespec);
    sel.mode = SEL_IDLE as c_int;
    sel.snap = 0;
    sel.ob.x = -1;
    sel.xtarget = xlib::XInternAtom(xw.dpy, CString::new("UTF8_STRING").unwrap().as_ptr(), 0);

    if sel.xtarget == 0 {
        sel.xtarget = xlib::XA_STRING;
    }
}


fn to_ptr(possible_arg: Option<&CString>) -> *const c_char {
    match possible_arg {
        Some(arg) => arg.as_ptr(),
        None => std::ptr::null(),
    }
}
