#![forbid(unsafe_code)]

fn main() {
    #[cfg(feature = "gtk-ui")]
    {
        forge_lab::ui::run();
    }
    #[cfg(not(feature = "gtk-ui"))]
    {
        println!("Developer Labs requires --features gtk-ui");
    }
}
