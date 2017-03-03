#![feature(link_args)]
extern crate libc;

use std::ffi::CString;

// "the `link_args` attribute is not portable across platforms" but that's fine,
// I just need it for the purposes of the port and only until I can move everything
// over to the rust X11 bindings
#[link_args = "-L/usr/lib -lc -L/usr/X11R6/lib -lm -lrt -lX11 -lutil -lXft -lfontconfig -lfreetype"]
extern "C" {
    fn st_main(argc: libc::c_int,
               argv: *const *const libc::c_char,
               opt_title: *const libc::c_char,
               opt_class: *const libc::c_char,
               opt_io: *const libc::c_char,
               opt_geo: *const libc::c_char,
               opt_font: *const libc::c_char,
               opt_line: *const libc::c_char,
               opt_name: *const libc::c_char,
               opt_embed: *const libc::c_char,
               opt_allow_alt_screen: libc::c_int,
               opt_is_fixed: libc::c_int)
               -> libc::c_int;
}

macro_rules! arg_set {
    ( $target:ident, $args:ident, $cmd_start:ident, $len:ident) => {{
        $cmd_start += 1;
        if $cmd_start < $len {
            $target = Some(CString::new($args.remove($cmd_start)).unwrap());

            $cmd_start -= 1;
            $args.remove($cmd_start); //remove the flag
            $len = $args.len();
        } else {
            println!("TODO usage");
            std::process::exit(1);
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
            "t" | "T" => arg_set!(opt_title, args, cmd_start, len),
            "c" => arg_set!(opt_class, args, cmd_start, len),
            "o" => arg_set!(opt_io, args, cmd_start, len),
            "g" => arg_set!(opt_geo, args, cmd_start, len),
            "f" => arg_set!(opt_font, args, cmd_start, len),
            "l" => arg_set!(opt_line, args, cmd_start, len),
            "n" => arg_set!(opt_name, args, cmd_start, len),
            "w" => arg_set!(opt_embed, args, cmd_start, len),
            "e" => {
                cmd_start += 1;
                break;
            }
            "" => {
                args.remove(cmd_start);

                len = args.len();
            }
            _ => {
                println!("TODO usage");
                std::process::exit(1);
            }
        }


    }

    let opt_cmd = args.split_at(cmd_start).1.to_owned();

    println!("opt_cmd {:?} args {:?}, ", opt_cmd, args);

    //http://stackoverflow.com/a/34379937/4496839
    // create a vector of zero terminated strings
    let zt_args = opt_cmd.iter()
        .cloned()
        .map(|arg| CString::new((*arg).to_string()).unwrap())
        .collect::<Vec<CString>>();
    println!("{:?}", zt_args);
    // convert the strings to raw pointers
    let c_args = zt_args.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const libc::c_char>>();
    let exit_code;

    unsafe {
        exit_code = st_main(c_args.len() as libc::c_int,
                            c_args.as_ptr(),
                            to_ptr(opt_title.as_ref()),
                            to_ptr(opt_class.as_ref()),
                            to_ptr(opt_io.as_ref()),
                            to_ptr(opt_geo.as_ref()),
                            to_ptr(opt_font.as_ref()),
                            to_ptr(opt_line.as_ref()),
                            to_ptr(opt_name.as_ref()),
                            to_ptr(opt_embed.as_ref()),
                            if opt_allow_alt_screen { 1 } else { 0 } as libc::c_int,
                            if opt_is_fixed { 1 } else { 0 } as libc::c_int);
    };

    std::process::exit(exit_code);
}

fn to_ptr(possible_arg: Option<&CString>) -> *const libc::c_char {
    match possible_arg {
        Some(arg) => arg.as_ptr(),
        None => std::ptr::null(),
    }
}
