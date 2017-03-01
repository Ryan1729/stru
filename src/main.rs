#![feature(link_args)]
extern crate libc;

use std::ffi::CString;

// "the `link_args` attribute is not portable across platforms" but that's fine,
// I just need it for the purposes of the port and only until I can move everything
// over to the rust X11 bindings
#[link_args = "-L/usr/lib -lc -L/usr/X11R6/lib -lm -lrt -lX11 -lutil -lXft -lfontconfig -lfreetype"]
extern "C" {
    fn st_main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int;
    fn fake_main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int;
}

fn main() {


    //http://stackoverflow.com/a/34379937/4496839
    // create a vector of zero terminated strings
    let args = std::env::args().map(|arg| CString::new(arg).unwrap()).collect::<Vec<CString>>();
    // convert the strings to raw pointers
    let c_args = args.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const libc::c_char>>();
    let result;
    unsafe {
        // pass the pointer of the vector's internal buffer to a C function
        fake_main(c_args.len() as libc::c_int, c_args.as_ptr());
        st_main(c_args.len() as libc::c_int, c_args.as_ptr());
        result = fake_main(c_args.len() as libc::c_int, c_args.as_ptr());
    };

    println!("{:?}", result);

}
