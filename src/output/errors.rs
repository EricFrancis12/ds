pub fn print_errors<I>(errors: I)
where
    I: IntoIterator,
    I::Item: std::fmt::Display,
{
    eprintln!("\n=== START ERRORS ===");
    for err in errors {
        eprintln!("{}", err);
    }
    eprintln!("=== END ERRORS ===\n");
}
