use console::style;
use dialoguer::{Input, Select};
use actix_web::web::Data;
use parking_lot::Mutex;
use crate::{
    app::state::AppState,
    pico::{better_theme, clear_and_get_memory}
};



pub fn initialize_example_classification(state: Data<Mutex<AppState>>) {
    let cli_options = &["Start", "Save Model", "Clear Memory", "Exit"];

    loop {
        let cli_selection = Select::with_theme(&better_theme())
            .with_prompt(&format!(
                "{}",
                style("EXAMPLE CLASSIFICATION").bold().green()
            ))
            .default(0)
            .items(cli_options)
            .interact()
            .unwrap();
        match cli_options[cli_selection] {
            "Begin" => {
                let num_of_rounds: u32 = Input::with_theme(&better_theme())
                    .with_prompt(&format!("{}", style("Enter num of rounds").bold().bold()))
                    .default(20)
                    .interact()
                    .unwrap();
                prompt_user_reactions(state.clone(), num_of_rounds)
            },
            "Clear Memory" => {let _ = clear_and_get_memory(state.clone(), true);}
            "Exit" => break,
            _ => println!("Unimplemented Selection"),
        }
    }
}

fn prompt_user_reactions(state: Data<Mutex<AppState>>,num_of_rounds: u32) {
    print!("If I was functional, I would run {} rounds of prompts",num_of_rounds)
}
