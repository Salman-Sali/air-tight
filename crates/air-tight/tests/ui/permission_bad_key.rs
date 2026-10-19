use air_tight::Permission;

#[derive(Permission)]
enum Action {
    #[air_tight(key = "A::B")]
    Read,
}

fn main() {}
