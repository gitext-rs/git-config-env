//! Parse git's env configuration

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

mod env;
mod param;

pub use env::*;
pub use param::*;
pub mod quote;
