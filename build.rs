extern crate gcc;

fn main() {
    // gcc::Config::new()
    //     .include("/usr/include")
    //     .include("/usr/X11R6/include")
    //     .include("/usr/include/freetype2")
    //     .flag("-D_XOPEN_SOURCE=600")
    //     .flag("-L/usr/lib")
    //     .flag("-lc")
    //     .flag("-L/usr/X11R6/lib")
    //     .flag("-lm")
    //     .flag("-lrt")
    //     .flag("-lX11")
    //     .flag("-lutil")
    //     .flag("-lXft")
    //     .flag("-lfontconfig")
    //     .flag("-lfreetype")
    //     .file("src/st.c")
    //     .compile("libst1.a");
    gcc::Config::new()
        .include("src")
        .flag("-g")
        .flag("-Os")
        .flag("-I/usr/include")
        .flag("-I/usr/X11R6/include")
        .flag("-I/usr/include/freetype2")
        .flag("-D_XOPEN_SOURCE=600")
        .flag("-L/usr/lib")
        .flag("-lc")
        .flag("-L/usr/X11R6/lib")
        .flag("-lm")
        .flag("-lrt")
        .flag("-lX11")
        .flag("-lutil")
        .flag("-lXft")
        .flag("-lfontconfig")
        .flag("-lfreetype")
        .file("src/st.c")
        .compile("libst1.a");


}
