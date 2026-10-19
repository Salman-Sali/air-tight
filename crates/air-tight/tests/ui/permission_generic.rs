use air_tight::Permission;

#[derive(Permission)]
enum Action<T> {
    Read,
    #[allow(dead_code)]
    Never(std::marker::PhantomData<T>),
}

fn main() {}
