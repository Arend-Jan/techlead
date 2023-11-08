use anyhow::{Context, Result};
use chat_gpt_lib_rs::client::Message;
use chat_gpt_lib_rs::{ChatGPTClient, ChatInput, ChatResponse, Model, Role};
use console::{style, StyledObject};
use dotenvy::dotenv;
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::io::{BufRead, BufReader};
use std::iter::Skip;
use std::time::Duration;

// The main function, which is asynchronous due to the API call
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger
    env_logger::init();

    // Load the environment variables from the .env file
    dotenv().ok();

    // Get the API key and icon usage setting from the environment variables
    let api_key =
        env::var("OPENAI_API_KEY").context("Failed to read OPENAI_API_KEY environment variable")?;

    // Add USE_ICONS=true to your .env file, if your terminal is running with a
    // Nerd Font, so you get some pretty icons
    let use_icons = env::var("USE_ICONS")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        .eq("true");

    // Initialize the ChatGPT client
    let client = ChatGPTClient::new(&api_key, "https://api.openai.com");

    let content = system_content(&api_key).await?;

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
        stdout().flush().context("Failed to flush stdout")?;

        // Read the user input
        let mut user_message_content = String::new();
        stdin()
            .read_line(&mut user_message_content)
            .context("Failed to read user input")?;

        // Process the user input and generate a response
        process_user_input(&client, &mut messages, user_message_content)
            .await
            .context("Failed to process user input")?;
    }
}

// this is what makes this the techlead cli application
// Here we set the behaviour of our chat gpt client
// for now it works for Rust project only
async fn system_content(api_key: &String) -> Result<String> {
    let mut return_value = "You are a very helpfull techlead of this project, who likes to teach and show code solutions.".to_string();
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
        "{}\n Directory tree (excluding /target and hidden directories)\n",
        return_value
    );
    for entry in walker.filter_map(Result::ok) {
        let depth = entry.depth().saturating_sub(1);
        let name = entry.file_name().to_string_lossy();
        let indent = "|____".repeat(depth);
        return_value = format!("{}{}{}\n", return_value, indent, name);

        if let Some(file_type) = entry.file_type() {
            if file_type.is_file() {
                let path = entry.path();
                let ext = path.extension().and_then(|ext| ext.to_str());

                if let Some(ext) = ext {
                    match ext {
                        "md" | "toml" => {
                            // Set up a spinner to display while waiting for the API response
                            let spinner = ProgressBar::new_spinner();
                            spinner.set_style(
                                ProgressStyle::default_spinner()
                                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                                    .template("{spinner:.green} Make summary for file ...")
                                    .unwrap(),
                            );
                            spinner.enable_steady_tick(Duration::from_millis(100));

                            return_value = format!("{return_value}# {name}\n");
                            let file = File::open(path).unwrap();
                            let reader = BufReader::new(file);
                            let mut text_to_summarize = String::new();
                            for line in reader.lines() {
                                text_to_summarize =
                                    format!("{}{}\n", text_to_summarize, line.unwrap());
                            }
                            // if name ends in .rs, add the entire input. if not, then make the
                            // summary

                            //println!("to_summarize: {text_to_summarize}");
                            let summary = summary(&api_key, text_to_summarize).await?;
                            return_value = format!(
                                "{return_value}\n {name} summary: {:?}",
                                summary.choices[0].message.content
                            );

                            spinner.finish_and_clear();

                            return_value = format!("{}\n", return_value);
                        }
                        "rs" => {
                            let file = File::open(path).unwrap();
                            let reader = BufReader::new(file);
                            let mut compacted_code = String::new();
                            for line in reader.lines() {
                                let line = line.unwrap();
                                let stripped_line = line.trim(); // Remove leading and trailing whitespace

                                // Remove comments and empty lines
                                if !stripped_line.starts_with("//") && !stripped_line.is_empty() {
                                    compacted_code =
                                        format!("{}{}\n", compacted_code, stripped_line);
                                }
                            }

                            return_value = format!("{}{}\n", return_value, compacted_code);
                            println!("{return_value}");
                        }

                        _ => {}
                    }
                }
            }
        }
    }
    Ok(return_value)
}

async fn summary(api_key: &String, to_summarize: String) -> Result<ChatResponse> {
    let client = ChatGPTClient::new(&api_key, "https://api.openai.com");
    let content = "Make a compact summary of the given input. if it is code, then give back only give back the code directly".to_string();

    // Initialize the message history with a system message
    let mut messages = vec![Message {
        role: Role::System,
        content,
    }];

    // Add a user message with the text to be summarized
    messages.push(Message {
        role: Role::User,
        content: format!("{}", to_summarize),
    });

    let input = ChatInput {
        model: Model::Gpt_4Turbo,
        messages: messages.clone(),
        ..Default::default()
    };

    let return_value = client
        .chat(input)
        .await
        .context("Failed to get chat response from client");
    return_value
}

async fn process_user_input(
    client: &ChatGPTClient,
    messages: &mut Vec<Message>,
    user_message_content: String,
) -> Result<()> {
    // Add the user message to the message history
    messages.push(Message {
        role: Role::User,
        content: user_message_content.trim().to_string(),
    });

    // Prepare the ChatInput object for the API call
    let input = ChatInput {
        model: Model::Gpt_4,
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
