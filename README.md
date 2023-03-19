# Techlead CLI

Techlead CLI is a command-line interface that allows you to chat with an AI assistant powered by the OpenAI GPT language model. This CLI was designed to be used specifically by a tech lead managing a Rust project.

## Installation

To install the Techlead CLI, you must have Rust installed. Then, you can simply clone this repository and run the following command:

```
cargo run
```

This will compile and run the project. Note that you will need an OpenAI API key, which you can obtain from the [OpenAI website](https://beta.openai.com/signup/).

## Usage

Once the Techlead CLI is up and running, you can start chatting with the AI assistant. The assistant is specifically trained to help with Rust project management, so feel free to ask questions related to that topic. You can also provide command-line arguments to the CLI to pre-populate the chat with a message.

Before running the Techlead CLI, make sure that you have set your OpenAI API key in the .env file. To do so, open the .env file and replace `OPENAI_API_KEY=<your_api_key_here>` with your actual OpenAI API key.

Here's an example of how to start a chat with a pre-populated message:

```
cargo run "Hello, how can I help you today?"
```

## Credits

This project was made possible thanks to the [Chat GPT Library for Rust](https://github.com/BlackPhlox/chat-gpt-lib-rs), which provides a Rust API for the OpenAI GPT language model.

## License
