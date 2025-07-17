pub enum OutputFormat {
    Dot,
}

pub trait RenderGraph {
    const OUTPUT: OutputFormat;

    fn render(&self) -> String;
}
