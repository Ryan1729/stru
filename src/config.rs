extern crate libc;
use libc::*;

extern crate x11;
use x11::xlib::*;
use x11::keysym::*;


//NOTE must be synced with config.h for as long as  that exists
pub const histsize: c_int = 16; //2000;

//TODO does this comment mean anything for this port?
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


pub const tabspaces: c_uint = 8;


/*
 * Default shape of cursor
 * 2: Block ("█")
 * 4: Underline ("_")
 * 6: Bar ("|")
 * 7: Snowman ("☃")
 */
pub const cursorshape: c_int = 2;
/*
 * thickness of underline and bar cursors
 */
pub const cursorthickness: c_uint = 2;

pub const defaultfont: &'static str = "Liberation Mono:pixelsize=16:antialias=true:autohint=true";
pub const borderpx: c_int = 2;


pub const colorname_len : usize = 16;
/* Terminal colors (16 first used in escape sequence) */
pub static colorname : [&'static str; colorname_len] = [
  /* 8 normal colors */
  "black",
  "red3",
  "green3",
  "yellow3",
  "blue2",
  "magenta3",
  "cyan3",
  "gray90",

  /* 8 bright colors */
  "gray50",
  "red",
  "green",
  "yellow",
  "#5c5cff",
  "magenta",
  "cyan",
  "white",

  /* this The idicies below 256 that are not filled in here
   will be filled in with a standard set of colours */
];

/* more colors can be added after 255 to use with DefaultXX */
pub const extra_len : usize = 2;
pub const extras : [&'static str; extra_len] = [
  "#cccccc",
  "#555555",
];

/*
 * Default colors (colorname index)
 * foreground, background, cursor, reverse cursor
 */
pub const defaultfg: c_uint = 7;
pub const defaultbg: c_uint = 0;
// pub const defaultcs: c_uint = 256;
// pub const defaultrcs: c_uint = 257;

/*
 * Default colour and shape of the mouse cursor
 * see https://tronche.com/gui/x/xlib/appendix/b/ for shape numbers
 */

pub const mouseshape : c_int = 152;
pub const mousefg : c_int = 7;
pub const mousebg : c_int = 0;

/* Kerning / character bounding-box multipliers */
pub const cwscale:c_float = 1.0;
pub const chscale:c_float = 1.0;

/*
 * State bits to ignore when matching key or button events.  By default,
 * numlock (Mod2Mask) and keyboard layout (XK_SWITCH_MOD) are ignored.
 */
const XK_SWITCH_MOD : c_uint = 1<<13;
pub static ignoremod: c_uint = Mod2Mask|XK_SWITCH_MOD;

/*
 * Override mouse-select while mask is active (when MODE_MOUSE is set).
 * Note that if you want to use ShiftMask with selmasks, set this to an other
 * modifier, set to 0 to not use it.
 */
pub const forceselmod: c_uint = ShiftMask;

/* selection timeouts (in milliseconds) */
pub const doubleclicktimeout: c_long = 300;
pub const tripleclicktimeout: c_long = 600;
