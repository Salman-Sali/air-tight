#[air_tight::permission(Workspace(Read))]
async fn handler() -> &'static str {
    "no setup"
}

fn main() {}
