# Trunk In-Line Rust

A plugin for [Trunk](https://github.com/trunk-rs/trunk) that allows in-line Rust scripts declared using [RFC 3424](https://rust-lang.github.io/rfcs/3424-cargo-script.html).

## Disclaimer

This is just a proof of concept and is _riddled_ with bugs and untested edge cases. Please don't _actually_ use this!

## Installation

* Compile this project using `cargo build` and place the executable wherever is desirable.
* Add a post-build hook launching the executable to your `Trunk.toml` file.

```toml
[[hooks]]
stage = "post_build"
command = "trunk_inline_rust"
```

* Add a `<script type="rust">` element to your HTML document and start writing Rust!

```html
<script type="rust">
    //! ```cargo
    //! [dependencies]
    //! wasm-bindgen = "0.2.92"
    //! ```

    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        fn alert(s: &str);
    }

    #[wasm_bindgen(start)]
    fn run() {
        alert("Hello World!");
    }
</script>
```

## Features

### Scoped `Cargo.toml`

Each script block has access to declaring its own `Cargo.toml` using [RFC 3424](https://rust-lang.github.io/rfcs/3424-cargo-script.html) syntax. No dependencies are included by default, so `wasm-bindgen` is recommended!

### Published Public Members

Any elements marked as `pub` within a script block will be made available on the `Window` context, allowing for immediate use in the rest of the document:

```html
<script type="rust">
    //! ```cargo
    //! [dependencies]
    //! wasm-bindgen = "0.2.92"
    //! ```
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        fn alert(s: &str);
    }

    #[wasm_bindgen]
    pub fn rusty_alert(input: &str) {
        alert(format!("Rusty Alert: {}", input).as_str());
    }
</script>

<button onclick="rusty_alert('Clicked!')">Click me</button>
```
