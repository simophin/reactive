mod clean_up;
mod component;
mod effect;
mod render;
mod signal;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
