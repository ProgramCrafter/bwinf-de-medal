# The Medal Contest Platform

Medal is a small platform for in-browser running contest written in rust.

It is designed for the German Jugendwettbewerb Informatik, a computer science contest with tasks using Google Blockly as a programming language-replacement.




## Translation

…


## Folder structure

### tasks/


## Running Medal

Needs `rustc` 1.28 (stable) or higher (cf. https://rustup.rs/). 

Running `make` compiles and runs a debug-/test-server.

For production use, a release binary should be compiled and served behind a reveres proxy (nginx, apache, …). The directories `tasks/` and `static/` can (and for throughput-purposes should) be served by the reverse proxy directly.
