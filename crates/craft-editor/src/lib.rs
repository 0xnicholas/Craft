pub fn run(_args: Vec<String>) -> eframe::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn smoke() {
        assert!(run(vec![]).is_ok());
    }
}
