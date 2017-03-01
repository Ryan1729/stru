extern crate gcc;

fn main() {
    gcc::Config::new()
        .include("src")
        .flag("-g")
        .flag("-Os")
        .flag("-I/usr/include")
        .flag("-I/usr/X11R6/include")
        .flag("-I/usr/include/freetype2")
        .flag("-D_XOPEN_SOURCE=600")
        .file("src/st.c")
        .compile("libst1.a");


}
