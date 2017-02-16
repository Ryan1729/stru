extern crate gcc;

fn main() {
    gcc::Config::new()
        .include("/usr/include")
        .include("/usr/X11R6/include")
        .include("/usr/include/freetype2")
        .flag("-L/usr/lib -lc -L/usr/X11R6/lib -lm -lrt -lX11 -lutil -lXft -lfontconfig -lfreetype -lfreetype")
        .file("src/st.c").compile("libst.a");
    
    //~ -g -std=c99 -pedantic -Wall -Wvariadic-macros -Os -I. -I -I -I/usr/include/freetype2 -I/usr/include/freetype2 -DVERSION="0.6" -D_XOPEN_SOURCE=600

}
