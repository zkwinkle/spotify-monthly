use anyhow::{anyhow, Result};
use rspotify::{auth_code_pkce::AuthCodePkceSpotify, prelude::*};
use std::{
    io::prelude::*,
    net::{Shutdown, TcpListener, TcpStream},
};

fn request_token(url: &str) {
    match webbrowser::open(url) {
        Ok(_) => println!("Opened {} in your browser.", url),
        Err(why) => eprintln!(
            "Error when trying to open an URL in your browser: {:?}. \
                Please navigate here manually: {}",
            why, url
        ),
    }
}

pub fn redirect_uri_web_server(
    spotify: &AuthCodePkceSpotify,
    url: &str,
    port: u16,
) -> Result<String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port));

    match listener {
        Ok(listener) => {
            request_token(url);

            //for stream in listener.incoming().take {
            match listener.incoming().next() {
                Some(Ok(stream)) => {
                    if let Some(url) = handle_connection(stream) {
                        return spotify.parse_response_code(&url).ok_or_else(
                            || anyhow!("Unable to parse the response code"),
                        );
                    }
                }
                Some(Err(e)) => {
                    eprintln!("{}", e)
                }
                _ => (),
            };
        }
        Err(e) => {
            eprintln!("{}", e)
        }
    }
    Err(anyhow!("Couldn't listen to incoming authentication"))
}

fn handle_connection(mut stream: TcpStream) -> Option<String> {
    // The request will be quite large (> 512) so just assign plenty just in case
    let mut buffer = [0; 1000];
    let _ = stream.read(&mut buffer).unwrap();

    // convert buffer into string and 'parse' the URL
    match String::from_utf8(buffer.to_vec()) {
        Ok(request) => {
            let split: Vec<&str> = request.split_whitespace().collect();

            if split.len() > 3 && !split[1].contains("error") {
                respond_with_success(stream);
                return Some(split[3].to_owned() + split[1]);
            }

            respond_with_error("Malformed request".to_owned(), stream);
        }
        Err(e) => {
            respond_with_error(
                format!("Invalid UTF-8 sequence: {}", e),
                stream,
            );
        }
    };

    None
}

fn respond_with_success(mut stream: TcpStream) {
    let contents = include_str!("redirect_uri.html");

    let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
}

fn respond_with_error(error_message: String, mut stream: TcpStream) {
    eprintln!("Error: {}", error_message);
    let response = format!(
        "HTTP/1.1 400 Bad Request\r\n\r\n400 - Bad Request - {}",
        error_message
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
}
