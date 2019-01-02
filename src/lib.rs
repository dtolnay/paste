//! The nightly-only [`concat_idents!`] macro in the Rust standard library is
//! notoriously underpowered in that its concatenated identifiers can only refer to
//! existing items, they can never be used to define something new.
//!
//! [`concat_idents!`]: https://doc.rust-lang.org/std/macro.concat_idents.html
//!
//! This crate provides a flexible way to paste together identifiers in a macro,
//! including using pasted identifiers to define new items.
//!
//! This approach works with any stable or nightly Rust compiler 1.30+.
//!
//! # Pasting identifiers
//!
//! There are two entry points, `paste::expr!` for macros in expression position and
//! `paste::item!` for macros in item position.
//!
//! Within either one, identifiers inside `[<`...`>]` are pasted together to form a
//! single identifier.
//!
//! ```rust
//! extern crate paste;
//!
//! // Macro in item position: at module scope or inside of an impl block.
//! paste::item! {
//!     // Defines a const called `QRST`.
//!     const [<Q R S T>]: &str = "success!";
//! }
//!
//! fn main() {
//!     // Macro in expression position: inside a function body.
//!     assert_eq!(
//!         paste::expr! { [<Q R S T>].len() },
//!         8,
//!     );
//! }
//! ```
//!
//! # More elaborate examples
//!
//! This program demonstrates how you may want to bundle a paste invocation inside
//! of a more convenient user-facing macro of your own. Here the `routes!(A, B)`
//! macro expands to a vector containing `ROUTE_A` and `ROUTE_B`.
//!
//! ```rust
//! extern crate paste;
//!
//! const ROUTE_A: &str = "/a";
//! const ROUTE_B: &str = "/b";
//!
//! macro_rules! routes {
//!     ($($route:ident),*) => {{
//!         paste::expr! {
//!             vec![$( [<ROUTE_ $route>] ),*]
//!         }
//!     }}
//! }
//!
//! fn main() {
//!     let routes = routes!(A, B);
//!     assert_eq!(routes, vec!["/a", "/b"]);
//! }
//! ```
//!
//! The next example shows a macro that generates accessor methods for some struct
//! fields.
//!
//! ```rust
//! extern crate paste;
//!
//! macro_rules! make_a_struct_and_getters {
//!     ($name:ident { $($field:ident),* }) => {
//!         // Define a struct. This expands to:
//!         //
//!         //     pub struct S {
//!         //         a: String,
//!         //         b: String,
//!         //         c: String,
//!         //     }
//!         pub struct $name {
//!             $(
//!                 $field: String,
//!             )*
//!         }
//!
//!         // Build an impl block with getters. This expands to:
//!         //
//!         //     impl S {
//!         //         pub fn get_a(&self) -> &str { &self.a }
//!         //         pub fn get_b(&self) -> &str { &self.b }
//!         //         pub fn get_c(&self) -> &str { &self.c }
//!         //     }
//!         paste::item! {
//!             impl $name {
//!                 $(
//!                     pub fn [<get_ $field>](&self) -> &str {
//!                         &self.$field
//!                     }
//!                 )*
//!             }
//!         }
//!     }
//! }
//!
//! make_a_struct_and_getters!(S { a, b, c });
//!
//! fn call_some_getters(s: &S) -> bool {
//!     s.get_a() == s.get_b() && s.get_c().is_empty()
//! }
//! #
//! # fn main() {}
//! ```

#![no_std]

extern crate paste_impl;
extern crate proc_macro_hack;

use proc_macro_hack::proc_macro_hack;

/// Paste identifiers within a macro invocation that expands to an expression.
#[proc_macro_hack]
pub use paste_impl::expr;

/// Paste identifiers within a macro invocation that expands to one or more
/// items.
///
/// An item is like a struct definition, function, impl block, or anything else
/// that can appear at the top level of a module scope.
pub use paste_impl::item;

/// Paste identifiers within a macro invocation that expands to one or more
/// macro_rules macros or items containing macros.
pub use paste_impl::item_with_macros;

#[doc(hidden)]
pub use paste_impl::EnumHack;
