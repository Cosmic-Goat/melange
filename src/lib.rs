#[macro_use]
extern crate smart_default;

use settings::{FullscreenType, Settings};
use std::{fs, process::Command};
use wry::{
    application::{
        event::{Event, KeyEvent, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        keyboard::Key,
        window::{Fullscreen, Window, WindowBuilder},
    },
    http::{Request, Response, ResponseBuilder},
    webview::{WebView, WebViewBuilder},
    Error,
};

mod settings;

pub struct Melange {
    config_dir: String,
    settings: Settings,
}

impl Melange {
    pub fn new(config_dir: String) -> Self {
        let settings = Settings::new(&config_dir);

        Melange {
            config_dir,
            settings,
        }
    }

    fn ipc_handler(window: &Window, message: String) {
        println!("{message}");
    }

    // fn run_protocol(&self, request: &Request) -> Result<Response, wry::Error> {
    //     // Remove url scheme
    //     let uri = request.uri().replace("run://", "");
    // }

    fn protocol(&self, request: &Request) -> Result<Response, wry::Error> {
        // Remove url scheme
        let uri = dbg!(request.uri().replace("melange://", ""));

        if uri.starts_with(&self.config_dir) {
            // TODO: Add check to make sure only files in the config directory can be accessed (with an option, maybe?)

            // get the file's location
            let path = fs::canonicalize(&uri)?;
            // Use MimeGuess to guess a mime type
            let mime = mime_guess::from_path(&path).first_raw().unwrap_or("");

            // Read the file content from file path
            let content = fs::read(path)?;
            ResponseBuilder::new().mimetype(mime).body(content)
        } else {
            // Create a response with this header set due to a bug on Linux with empty headers
            let response = ResponseBuilder::new().header("Access-Control-Allow-Origin", "*");
            if let Some(inputs) = self.settings.commands.get(&uri) {
                let output = Command::new(&inputs[0])
                    .args(&inputs[1..])
                    .output()
                    .expect("failed to execute process");
                // print!("{}", String::from_utf8(output.stdout).unwrap());

                response
                    .status(200)
                    // See https://www.iana.org/assignments/media-types/text/strings
                    .mimetype("text/strings")
                    .body(output.stdout)
            } else {
                dbg!(response
                    .status(404)
                    .body("Command not found in config!".as_bytes().to_vec()))
            }
        }
    }

    pub fn make_window(&self, event_loop: &EventLoop<()>) -> Window {
        let window = WindowBuilder::new()
            .with_title(&self.settings.window.title)
            .with_decorations(self.settings.window.decorated)
            .with_always_on_top(self.settings.window.always_on_top)
            .with_transparent(self.settings.window.transparent)
            .with_fullscreen(match self.settings.window.mode {
                FullscreenType::Windowed => None,
                FullscreenType::Borderless => None,
                FullscreenType::Full => Some(Fullscreen::Borderless(None)),
            })
            .build(event_loop)
            .unwrap();

        match self.settings.window.mode {
            FullscreenType::Windowed => {
                // Only set the window size and position if it's specified in the config,
                // otherwise just let the WM handle it with its default behaviour
                if let Some(size) = self.settings.window.size {
                    window.set_inner_size(size);
                };
                if let Some(position) = self.settings.window.position {
                    window.set_outer_position(position);
                };
            }
            FullscreenType::Borderless => {
                let monitor = window.primary_monitor().unwrap();
                window.set_inner_size(monitor.size());
                window.set_outer_position(monitor.position());
            }
            _ => {}
        };

        window
    }

    pub fn make_webview(self, window: Window) -> Result<WebView, Error> {
        // Allow the use of web servers, e.g. for local dev
        let url = if self.config_dir.starts_with("http") {
            self.config_dir.to_owned()
        } else {
            format!("melange://{}/index.html", self.config_dir)
        };

        let webview = WebViewBuilder::new(window)
            .unwrap()
            .with_transparent(true)
            .with_devtools(self.settings.debug.devtools)
            .with_ipc_handler(Melange::ipc_handler)
            // .with_custom_protocol("run".into(), move |s| self.run_protocol(s))
            .with_custom_protocol("melange".into(), move |s| self.protocol(s))
            // tell the webview to load the custom protocol
            .with_url(&url)?
            .build();

        // This has to be set AFTER any window size changes are made, otherwise they won't take effect
        // Doesn't seem to work with setting a window size, so disabled for now
        // webview.window().set_resizable(false);

        webview
    }

    pub fn run_loop(event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                // TODO: Add setting for quit key?
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
}