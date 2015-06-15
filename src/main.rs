#[macro_use] extern crate log;


mod bencode;
mod nodes;


pub use bencode::BItem;
pub use nodes::ID;


fn main() {
    println!("Hello, world!");
}
