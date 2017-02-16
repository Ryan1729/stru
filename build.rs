extern crate gcc;

fn main() {
    gcc::Config::new().file("src/st.c").compile("libst.a");
}
