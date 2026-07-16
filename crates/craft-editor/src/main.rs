fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    craft_editor::run(args)
}
