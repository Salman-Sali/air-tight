use air_tight::IdentifyResources;

#[derive(IdentifyResources)]
struct Body {
    #[air_tight(id)]
    id: u8,
}

fn main() {}
