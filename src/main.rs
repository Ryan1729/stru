extern crate libc;

use std::ffi::CString;

extern "C" {
    // fn st_main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int;
    fn fake_main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int;
    fn double_input(input: libc::c_int) -> libc::c_int;
}

fn main() {
    let input = 4;
    let output = unsafe { double_input(input) };
    println!("{} * 2 = {}", input, output);

    //http://stackoverflow.com/a/34379937/4496839
    // create a vector of zero terminated strings
    let args = std::env::args().map(|arg| CString::new(arg).unwrap()).collect::<Vec<CString>>();
    // convert the strings to raw pointers
    let c_args = args.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const libc::c_char>>();
    let result;
    unsafe {
        // pass the pointer of the vector's internal buffer to a C function
        // st_main(c_args.len() as libc::c_int, c_args.as_ptr());
        result = fake_main(c_args.len() as libc::c_int, c_args.as_ptr());
    };

    println!("{:?}", result);

}
