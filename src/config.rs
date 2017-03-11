extern crate libc;
use libc::*;

//NOTE must be synced with config.h for as long as  that exists
pub const histsize: usize = 16; //2000;

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
