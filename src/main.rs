#[macro_use]
extern crate smart_default;

use std::env;
use std::fs::{canonicalize, read};
use std::process::Command;

use settings::{FullscreenType, Settings};
use wry::application::event::KeyEvent;
use wry::application::keyboard::Key;
use wry::application::window::{Fullscreen, Window};
use wry::http::{Request, Response};
use wry::{
    application::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    http::ResponseBuilder,
    webview::WebViewBuilder,
};

mod settings;

fn execute(inputs: &Vec<String>) {
    let output = Command::new(&inputs[0])
        .args(&inputs[1..])
        .output()
        .expect("failed to execute process");
    print!("{}", String::from_utf8(output.stdout).unwrap());
}

// if let Some(command) = self.config.keymap.get(&Input { key, mods }) {
//     MainState::execute(command);
// }

fn ipc_handler(window: &Window, message: String) {
    println!("{message}");
}

fn protocol(request: &Request) -> Result<Response, wry::Error> {
    // TODO: Add check to make sure only files in the config directory can be accessed (with an option, maybe?)

    // Remove url scheme
    let uri = request.uri().replace("melange://", "");
    // get the file's location
    let path = canonicalize(&uri)?;
    // Use MimeGuess to guess a mime type
    let mime = mime_guess::from_path(&path).first_raw().unwrap_or("");

    // Read the file content from file path
    let content = read(path)?;
    ResponseBuilder::new().mimetype(mime).body(content)
}

// TODO: Refactor code to lib.rs
fn main() -> wry::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config_dir = if let Some(path) = args.get(1) {
        path.to_owned()
    } else {
        format!(
            "{}/informant",
            env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!(
            "{}/.config",
            env::var("HOME").expect(
                "Your $HOME variable isn't set, I think you have bigger problems than this error."
            )
        ))
        )
    };

    let settings = Settings::new(&config_dir);
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title(settings.window.title)
        .with_decorations(settings.window.decorated)
        .with_always_on_top(settings.window.always_on_top)
        .with_transparent(settings.window.transparent)
        .with_fullscreen(match settings.window.mode {
            FullscreenType::Windowed => None,
            FullscreenType::Borderless => None,
            FullscreenType::Full => Some(Fullscreen::Borderless(None)),
        })
        .build(&event_loop)
        .unwrap();

    match settings.window.mode {
        FullscreenType::Windowed => {
            // Only set the window size and position if it's specified in the config,
            // otherwise just let the WM handle it with its default behaviour
            if let Some(size) = settings.window.size {
                window.set_inner_size(size);
            };
            if let Some(position) = settings.window.position {
                window.set_outer_position(position);
            };
        }
        FullscreenType::Borderless => {
            let monitor = window.primary_monitor().unwrap();
            window.set_inner_size(monitor.size());
            window.set_outer_position(monitor.position());
        }
        _ => {}
    }

    // Allow the use of web servers, e.g. for local dev
    let url = if config_dir.starts_with("http") {
        config_dir
    } else {
        format!("melange://{}/index.html", &config_dir)
    };

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_transparent(true)
        .with_ipc_handler(ipc_handler)
        .with_custom_protocol("melange".into(), protocol)
        // tell the webview to load the custom protocol
        .with_url(&url)?
        .build()?;

    // This has to be set AFTER any window size changes are made, otherwise they won't take effect
    // Doesn't seem to work with setting a window size, so disabled for now
    // webview.window().set_resizable(false);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Escape,
                                ..
                            },
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
