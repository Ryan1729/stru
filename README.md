# stru

st + rust => stru

This is a (currenly incomplete!) port of [st](http://st.suckless.org/) to [Rust](https://www.rust-lang.org).

This was done (in as far as it was done,) mostly as a learning exercise, (although having a terminal emulator which I understood from top to bottom and could add features to also souned cool).

## Current Status

I am shelving this for now, mostly because I feel I've learned most of what I can learn from this project, and the rest of the work necessary is largely just more of what was done before.
I'm actually releasing this in it's current state because I'm starting another porting project and I'm planning to use this repo as a reference on how to get started with dual language compilation, and I thought it might be useful to someone else, (though if I'm honest, having it on github so I can look at it from anywhere is also a plus.)

## Running/building

use `run.sh`.
currently the project requires `cargo`, a nightly version (it currently uses `nightly-2017-02-09` but other versions are likely to work as well), and a c compiler that works with [the rust gcc package](https://github.com/alexcrichton/gcc-rs)
