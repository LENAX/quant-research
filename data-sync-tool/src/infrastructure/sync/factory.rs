/**
 * Factory traits and methods for building components in the sync module
 */

/**
 * The Builder traits for all builders
 */
pub trait Builder {
    type Product;

    fn new() -> Self;
    fn build(self) -> Self::Product;
}
