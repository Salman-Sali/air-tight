use air_tight::IdentifyResources;

#[derive(IdentifyResources)]
struct Body {
    #[air_tight(nested)]
    inner: u8,
}

fn main() {}
