use air_tight::Permission;

#[derive(Permission)]
enum Action {
    Read,
    Update(u8),
}

fn main() {}
