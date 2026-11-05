//! Pet Android - Wrapper para build Android via Gradle
//!
//! O build real Android é feito via Gradle + NDK.
//! Este crate serve apenas para integração no workspace Cargo.

pub use pet_core as core;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}