use chat_gpt_lib_rs::client::{ChatGPTError, Message};
use chat_gpt_lib_rs::{ChatGPTClient, ChatInput, Model, Role};
use console::{style, StyledObject};
use dotenvy::dotenv;
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::io::{BufRead, BufReader};
use std::iter::Skip;
use std::time::Duration;

// The main function, which is asynchronous due to the API call
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Load the environment variables from the .env file
    dotenv().ok();

    // Get the API key and icon usage setting from the environment variables
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env");

    // Add USE_ICONS=true to your .env file, if your terminal is running with a
    // Nerd Font, so you get some pretty icons
    let use_icons = env::var("USE_ICONS")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        .eq("true");

    // Initialize the ChatGPT client
    let client = ChatGPTClient::new(&api_key, "https://api.openai.com");

    let content = system_content().await?;

    // Initialize the message history with a system message
    let mut messages = vec![Message {
        role: Role::System,
        content,
    }];

    // Check if any command line arguments are provided
    let mut args: Skip<env::Args> = env::args().skip(1);
    if let Some(first_arg) = args.next() {
        let user_message_content = args.fold(first_arg, |acc, arg| acc + " " + &arg);

        // Process the user input from command line arguments
        process_user_input(&client, &mut messages, user_message_content).await?;
    }

    // Enter the main loop, where user input is accepted and responses are generated
    loop {
        // Display the input prompt with an optional icon
        let input_prompt: StyledObject<&str> = if use_icons {
            style("\u{f0ede} Input: ").green()
        } else {
            style("Input: ").green()
        };
        print!("{}", input_prompt);
        stdout().flush().unwrap();

        // Read the user input
        let mut user_message_content = String::new();
        stdin().read_line(&mut user_message_content).unwrap();

        // Process the user input and generate a response
        process_user_input(&client, &mut messages, user_message_content).await?;
    }
}

// this is what makes this the techlead cli application
// Here we set the behaviour of our chat gpt client
// for now it works for Rust project only
async fn system_content() -> Result<String, Box<dyn Error>> {
    let mut return_value = "You're the techlead on this project".to_string();
    let root = ".";
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .hidden(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .filter_entry(|entry| {
            !entry
                .file_name()
                .to_str()
                .map_or(false, |s| s.starts_with('.') || s == "target")
        })
        .build();

    return_value = format!(
        "{}\n Directory tree (excluding /target and hidden directories\n",
        return_value
    );
    for entry in walker.filter_map(Result::ok) {
        let depth = entry.depth().saturating_sub(1);
        let name = entry.file_name().to_string_lossy();
        let indent = "|____".repeat(depth);
        return_value = format!("{}{}{}", return_value, indent, name);

        if let Some(file_type) = entry.file_type() {
            if file_type.is_file() {
                let path = entry.path();
                let ext = path.extension().and_then(|ext| ext.to_str());

                if let Some(ext) = ext {
                    match ext {
                        "md" | "rs" | "toml" => {
                            return_value = format!("{}===\n", return_value);
                            let file = File::open(path).unwrap();
                            let reader = BufReader::new(file);
                            for line in reader.lines() {
                                return_value = format!("{}{}\n", return_value, line.unwrap());
                            }
                            return_value = format!("{}===\n", return_value);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(return_value)
}

async fn process_user_input(
    client: &ChatGPTClient,
    messages: &mut Vec<Message>,
    user_message_content: String,
) -> Result<(), ChatGPTError> {
    // Add the user message to the message history
    messages.push(Message {
        role: Role::User,
        content: user_message_content.trim().to_string(),
    });

    // Prepare the ChatInput object for the API call
    let input = ChatInput {
        model: Model::Gpt3_5Turbo,
        messages: messages.clone(),
        ..Default::default()
    };

    // Set up a spinner to display while waiting for the API response
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} Processing...")
            .unwrap(),
    );

    // Make the API call and store the result
    let chat = {
        spinner.enable_steady_tick(Duration::from_millis(100));
        let result = client.chat(input).await;
        spinner.finish_and_clear();
        result?
    };

    // Extract the assistant's message from the API response
    let assistant_message = &chat.choices[0].message.content;

    // Display the computer's response with an optional icon
    let computer_label: StyledObject<&str> = if env::var("USE_ICONS")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        .eq("true")
    {
        style("\u{f12ca} Computer: ").color256(39)
    } else {
        style("Computer: ").color256(39)
    };
    let computer_response: StyledObject<String> = style(assistant_message.clone());

    println!("{}{}", computer_label, computer_response);

    // Add the assistant's message to the message history
    messages.push(Message {
        role: Role::Assistant,
        content: assistant_message.clone(),
    });

    Ok(())
}
