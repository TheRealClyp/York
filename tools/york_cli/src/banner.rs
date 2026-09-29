use colored::Colorize;

/// ASCII-art York logo banner, printed at startup.
pub fn banner(version: &str) {
    let cyan = "   ██╗  ██╗ ██████╗ ██████╗ ██╗  ██╗".cyan();
    let cyan2 = "   ██║ ██╔╝██╔═══██╗██╔══██╗██║ ██╔╝".cyan();
    let purple = "   █████╔╝ ██║   ██║██████╔╝█████╔╝ ".purple();
    let purple2 = "   ██╔═██╗ ██║   ██║██╔══██╗██╔═██╗ ".purple();
    let end = "   ██║  ██╗╚██████╔╝██║  ██║██║  ██╗".cyan();
    let end2 = "   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝".cyan();

    // "WASAY" — printed below the YORK mark.
    let w1 = "██╗    ██╗  █████╗  ███████╗  █████╗  ██╗   ██╗".purple();
    let w2 = "██║    ██║  ██╔══██╗  ██╔════╝  ██╔══██╗  ╚██╗ ██╔╝".purple();
    let w3 = "██║ █╗ ██║  ███████║  ███████╗  ███████║   ╚████╔╝ ".purple();
    let w4 = "██║███╗██║  ██╔══██║  ╚════██║  ██╔══██║    ╚██╔╝  ".purple();
    let w5 = "╚███╔███╔╝  ██║  ██║  ███████║  ██║  ██║     ██║   ".purple();
    let w6 = " ╚══╝╚══╝  ╚═╝  ╚═╝  ╚══════╝  ╚═╝  ╚═╝     ╚═╝   ".purple();

    println!();
    println!("{cyan}");
    println!("{cyan2}");
    println!("{purple}");
    println!("{purple2}");
    println!("{end}");
    println!("{end2}");
    println!();
    println!("{w1}");
    println!("{w2}");
    println!("{w3}");
    println!("{w4}");
    println!("{w5}");
    println!("{w6}");
    println!();
    println!("{}  {}      a fast, friendly systems language", "york".bold().cyan(), format!("v{version}").dimmed());
    println!("{}", "Created by Wasay & contributors. Released under the MIT license.".dimmed());
    println!();
}

/// Compact one-line header for repeated rebuilds in dev mode.
pub fn dev_header(version: &str) {
    println!();
    println!("{} {} {}  watch mode — press Ctrl+C to stop", "york".bold().cyan(), format!("v{version}").dimmed(), "dev".purple().bold());
    println!("{}", "-".repeat(56).dimmed());
}