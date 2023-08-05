# Design Patterns In Rust

**Warning:** Do not overuse design patterns. Use it when needed. Otherwise, it adds unnecessary complexity to your code base.


## Abstract Factory

### Usecase

Let say I have a trait Called TaskExecutor, and I have multiple implementations for this trait. These implementations also depend on other components bounded by some other traits. Thus, it is not trivial to define a simple constructor for each implementation. Instead, we have to use factory method to encapsulate the construction logic. Since we have one factory for each implementation, and client modules need to decide which one to use at runtime or by configuration, we need to provide a common interface for client modules to create these implementations. Client modules can designate which one to create by passing some request object (which encapsulate specific parameters for each factory) to that abstract factory to obtain a factory method bound to that specific implementation.

### How to use it in Rust

In Rust, we can use the associated type feature to implement an abstract factory. Here are some resources:

1. [Refactor Guru: Abstract Factory Pattern](https://refactoringguru.cn/design-patterns/abstract-factory)

2. [Refactor Guru: Abstract Factory in Rust](https://refactoringguru.cn/design-patterns/abstract-factory/rust/example)

### Example

To write an abstract factory method for traits in Rust, you can use associated types. Factory can have an associated type called Output. You can add a bound that requires Output to implement Thingy:

```rust
trait Factory {
    type Output: Thingy;
    fn make (&self) -> Self::Output;
}
```

Now, AFactory’s Output will be AThingy:

```rust
impl Factory for AFactory {
    type Output = AThingy;
    fn make (&self) -> AThingy {
        AThingy {}
    }
}
```

And BFactory’s Output will be BThingy:

```rust
impl Factory for BFactory {
    type Output = BThingy;
    fn make (&self) -> BThingy {
        BThingy {}
    }
}
```

You can also use Box`<dyn Trait>` to store a Factory. However, because Factory has an associated type, you also have to specify the Output when making it into a trait object:

```rust
let aFactory: Box<dyn Factory<Output = AThingy>> = AFactory {};
let bFactory: Box<dyn Factory<Output = BThingy>> = BFactory {};
```
