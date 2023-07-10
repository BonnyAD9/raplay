mod sink;
mod source;

pub fn run() {
    println!("Hello");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main() {
        run();
    }
}
