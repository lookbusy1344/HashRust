#![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]

// unit tests are part of the main crate

#[cfg(test)]
use super::*;

#[test]
fn unit_it_works() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn help_length() {
    assert!(HELP.len() > 10);
}
