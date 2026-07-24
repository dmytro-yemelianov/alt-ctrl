use std::{env, io};

fn main() -> io::Result<()> {
    if env::args().any(|argument| argument == "--snapshot") {
        alt_ctrl_tui::print_snapshot(120, 40)
    } else {
        alt_ctrl_tui::run()
    }
}
