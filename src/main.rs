#![feature(link_args)]
#![allow(non_upper_case_globals)]
extern crate libc;
use libc::*;

extern crate x11;
use x11::xlib;
use x11::xft;

use std::ffi::CString;
use std::mem;
use std::ptr;
use std::cmp::max;


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
               opt_font: *const c_char,
               opt_line: *const c_char,
               opt_name: *const c_char,
               opt_embed: *const c_char)
               -> c_int;

    fn tsetdirt(top: c_int, bot: c_int);
    fn tresize(col: c_int, row: c_int);
    fn tclearregion(x1: c_int, y1: c_int, x2: c_int, y2: c_int);
}

macro_rules! arg_set {
    ( $target:ident, $args:ident, $cmd_start:ident, $len:ident, $exe_path:expr) => {{
        $cmd_start += 1;
        if $cmd_start < $len {
            $target = Some(CString::new($args.remove($cmd_start)).unwrap());

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

macro_rules! new {
    (TCursor) => {
        TCursor {
            attr: Glyph {
                u: 0,
                mode: ATTR_NULL as u16,
                fg: defaultfg,
                bg: defaultbg,
            },
            x: 0,
            y: 0,
            state: CURSOR_DEFAULT as c_char,
        }
    }
}

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
    let mut i: c_uint = tabspaces;
    while i < term.col as c_uint {
        ptr::write(term.tabs.offset(i as isize), 1);
        i += tabspaces;
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

//NOTE must be synced with config.h for as long as  that exists
const histsize: usize = 16; //2000;
const defaultfg: c_uint = 7;
const defaultbg: c_uint = 0;
/*
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
const tabspaces: c_uint = 8;
const cursorshape: c_int = 2;

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
    w: c_int,
    h: c_int, /* window width and height */
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
    cursor: cursorshape,
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
    line: 0,
    alt: 0,
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
    line: usize,
    alt: usize,
    hist: [usize; histsize],
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
pub static mut allowaltscreen: c_int = 1;

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
    let mut opt_embed: Option<CString> = None;

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
            "t" | "T" => arg_set!(opt_title, args, cmd_start, len, &exe_path),
            "c" => arg_set!(opt_class, args, cmd_start, len, &exe_path),
            "o" => arg_set!(opt_io, args, cmd_start, len, &exe_path),
            "g" => arg_set!(opt_geo, args, cmd_start, len, &exe_path),
            "f" => arg_set!(opt_font, args, cmd_start, len, &exe_path),
            "l" => arg_set!(opt_line, args, cmd_start, len, &exe_path),
            "n" => arg_set!(opt_name, args, cmd_start, len, &exe_path),
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
    let exit_code;

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

        exit_code = st_main(c_args.len() as c_int,
                            c_args.as_ptr(),
                            to_ptr(opt_title.as_ref()),
                            to_ptr(opt_class.as_ref()),
                            to_ptr(opt_io.as_ref()),
                            to_ptr(opt_font.as_ref()),
                            to_ptr(opt_line.as_ref()),
                            to_ptr(opt_name.as_ref()),
                            to_ptr(opt_embed.as_ref()));
    };

    std::process::exit(exit_code);
}

fn to_ptr(possible_arg: Option<&CString>) -> *const c_char {
    match possible_arg {
        Some(arg) => arg.as_ptr(),
        None => std::ptr::null(),
    }
}
