extern crate libc;
use libc::*;

//NOTE must be synced with config.h for as long as  that exists
pub const histsize: usize = 16; //2000;

/*
 * Default colors (colorname index)
 * foreground, background, cursor, reverse cursor
 */
pub const defaultfg: c_uint = 7;
pub const defaultbg: c_uint = 0;
// pub const defaultcs: c_uint = 256;
// pub const defaultrcs: c_uint = 257;

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
