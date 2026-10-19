use air_tight::Permission;

#[derive(Permission)]
enum Action {
    Read,
    #[air_tight(key = "Read")]
    View,
}

fn main() {}
