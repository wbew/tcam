# tcam

<img width="1836" height="1302" alt="image" src="https://github.com/user-attachments/assets/6926acc9-97fd-48be-a017-458568b9a7a5" />


## Set up

With a Rust toolchain, just clone the project and `cargo run`!

## Usage 

`tcam`. That's it!

A TUI will launch, and, hopefully, is self-explanatory.

Selfies save as `capture-{timestamp}` and will be available in the same folder
as you're running `tcam`.

## Why?

I built `tcam` as a warm up project at the Recurse Center. It's in Rust, which I
used to use at work, in the terminal, which I enjoy, and in good fun, which I
think is more important than ever.

I spent a few minutes thinking it up and a few hours over a few days building
it. I used Cursor and Cursor's agent workflow extensively towards the end. The
project isn't so much about `tcam` as it is about (1) shipping, shipping
anything and (2) searching for a way to work with AI tools. I'm bullish on new
tools redefining the software engineering experience.

If you're curious about `tcam` as an artifact though, here are a few cool threads to tug on,

- `ratatui` is an excellent TUI library. I haven't used many so don't have a
  great comparison, but I found it easy to get started.
- `halfblocks` are a cool concept. Similar to ASCII, this involves using
  halfblock unicode symbols as a hack to print images to terminals that do not
  support image protocols.
- `terminal protocols` Your terminal speaks a language, which I've never thought
  about it. Programs interacting with the terminal, say a TUI, will send some
  command string through this protocol to get the terminal to do things. Try it
  out yourself. `echo -en "\x1b[18t"` sends a command asking for the terminal
  window's dimensions (I've only tested this on Mac's built-in terminal).
- `performance` `tcam` is so damn slow. The live preview is insanely choppy. I'm
  not sure why, but surely has something to do with how images are taken and
  rendered. 
