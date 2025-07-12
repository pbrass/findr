Project from the book Command-Line Rust. This is a
Rust implementation of the standard Unixy find command, but
unlike the exercise from the book I have implemented most of the
file-testing expression language including boolean operators,
parentheses, etc.

The link-oriented tests and the "do I actually have these perms"
tests are not implemented, and the only *command* that works is the
default, just print the filename. No pruning, no execution.

I admit this one got away from me a bit, but you can't ask me to
implement a scaled down version of a program that has a perfectly
good predicate logic language in its command-line options.

Learned a lot about using Pest in Rust to implement a little DSL. Fun!