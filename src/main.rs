use std::cell::RefCell;
use std::path::Path;

use anyhow::Error;
use gio::SimpleAction;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::Application;
use log::debug;
use pango::FontDescription;
use serde::{Deserialize, Serialize};
use vte::{self, TerminalExt};

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

mod hacks;

#[derive(Debug, Serialize, Deserialize)]
struct ColorConfig {
    foreground: String,
    background: String,
    palette: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    font_family: String,
    font_size: u32,
    colors: ColorConfig,
}

impl Config {
    fn from_file(path: &Path) -> Result<Config, Error> {
        let file = std::fs::read(path)?;
        let config = toml::from_slice(&file)?;
        Ok(config)
    }

    fn from_str(s: &str) -> Result<Config, Error> {
        let config = toml::from_str(s)?;
        Ok(config)
    }
}

struct TerminalConfig {
    font: FontDescription,
    foreground: gdk::RGBA,
    background: gdk::RGBA,
    palette: Vec<gdk::RGBA>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            font: FontDescription::default(),
            foreground: gdk::RGBA::white(),
            background: gdk::RGBA::black(),
            palette: Vec::new(),
        }
    }
}

struct Env {
    user: String,
    host: String,
    cur_dir: String,
}

impl Default for Env {
    fn default() -> Self {
        Env {
            user: glib::user_name()
                .into_string()
                .unwrap_or_else(|_| String::new()),
            host: glib::host_name().to_string(),
            cur_dir: glib::current_dir()
                .map(|s| s.into_os_string().into_string().unwrap())
                .unwrap_or_else(String::new),
        }
    }
}

fn new_terminal(config: &TerminalConfig) -> vte::Terminal {
    let terminal = vte::Terminal::builder()
        .font_desc(&config.font)
        .has_focus(true)
        .is_focus(true)
        .build();

    terminal.set_colors(
        Some(&config.foreground),
        Some(&config.background),
        &config.palette,
    );
    let shell = glib::getenv("SHELL").expect("SHELL must be set");

    terminal.spawn_async(
        vte::PtyFlags::DEFAULT,
        None,
        &[Path::new(&shell)],
        &[],
        glib::SpawnFlags::DEFAULT,
        Some(Box::new(|| /* println!("child setup") */ {})),
        -1,
        None::<&gio::Cancellable>,
        Some(Box::new(|terminal, pid, error| {
            debug!("{:?} {:?} {:?}", terminal, pid, error);
        })),
    );
    terminal
}

glib::wrapper! {
    pub struct Term(ObjectSubclass<TermImpl>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Container, gtk::Widget, gtk::Buildable,
        @implements gio::ActionMap;
}

#[derive(Default)]
pub struct TermImpl {
    notebook: RefCell<gtk::Notebook>,
    env: RefCell<Env>,
    config: RefCell<TerminalConfig>,
}

#[glib::object_subclass]
impl ObjectSubclass for TermImpl {
    const NAME: &'static str = "Terminal";
    type Type = Term;
    type ParentType = gtk::ApplicationWindow;
}

impl ObjectImpl for TermImpl {
    fn constructed(&self, _obj: &Self::Type) {
        //obj.add(&*self.notebook.borrow());
    }
}
impl WidgetImpl for TermImpl {}
impl BinImpl for TermImpl {}
impl WindowImpl for TermImpl {}
impl ApplicationWindowImpl for TermImpl {}
impl ContainerImpl for TermImpl {}

impl Term {
    fn new(app: &Application, env: Env, config: TerminalConfig) -> Self {
        let obj: Term = glib::Object::new(&[("application", app)]).unwrap();
        let term = TermImpl::from_instance(&obj);
        *term.env.borrow_mut() = env;
        *term.config.borrow_mut() = config;
        *term.notebook.borrow_mut() = gtk::NotebookBuilder::new().parent(&obj).build();

        obj.add_actions();
        obj.add_new_tab();
        obj.add_events();

        obj
    }

    fn notebook(&self) -> gtk::Notebook {
        let term = TermImpl::from_instance(self);
        term.notebook.borrow().clone()
    }

    fn add_events(&self) {
        let this = self.clone();
        self.connect_local(
            "key-press-event",
            false,
            glib::clone!(@weak this => @default-return Some(false.to_value()), move |val| {
                let event = val[1].get::<gdk::Event>().unwrap();
                let event: Option<&gdk::EventKey> = event.downcast_ref();
                let mut set_tab: Option<u32> = None;
                if let Some(event) = event {
                    if event.state().contains(gdk::ModifierType::MOD1_MASK) { // TODO: make configurable
                        let c = event.keyval().to_unicode().and_then(|c| c.to_digit(10));
                        set_tab = c;
                    }
                }
                if let Some(new_tab) = set_tab {
                    let nb = this.notebook();
                    nb.set_current_page(new_tab.checked_sub(1));
                    return Some(true.to_value());
                }
                Some(false.to_value())
            }),
        )
        .unwrap();

        self.connect_local(
            "configure-event",
            false,
            glib::clone!(@weak this => @default-return Some(false.to_value()), move |val| {
                let event = val[1].get::<gdk::Event>().unwrap();
                let event: Option<&gdk::EventConfigure> = event.downcast_ref();
                if let Some(event) = event {
                    let new_size = event.size();
                    this.resize(new_size.0 as i32, new_size.1 as i32);
                    let nb = this.notebook();
                    nb.set_allocation(&gdk::Rectangle { x: 0, y: 0, width: new_size.0 as i32, height: new_size.1 as i32 });
                    nb.queue_resize();
                }
                Some(false.to_value())
            }),
        )
        .unwrap();
    }

    fn add_actions(&self) {
        let app = self.application().unwrap();
        let new_tab = SimpleAction::new("new_tab", None);
        app.set_accels_for_action("win.new_tab", &["<Alt>t"]);

        let win = self.clone();
        new_tab.connect_activate(glib::clone!(@weak win => move |_, _| {
            win.add_new_tab();
        }));
        self.add_action(&new_tab);
    }

    fn remove_tab(&self, term: &vte::Terminal) {
        let notebook = self.notebook();
        let mut removed = false;

        for (page, child) in notebook.children().iter().enumerate() {
            if child == term {
                notebook.remove_page(Some(page as u32));
                removed = true;
                continue;
            }
            if removed {
                notebook.set_tab_label(child, Some(&self.page_label(page as u32, None)));
            }
        }
        notebook.set_show_tabs(notebook.n_pages() > 1);
        if notebook.children().is_empty() {
            self.close();
        }
    }

    fn page_label(&self, page_number: u32, title: Option<&str>) -> gtk::Label {
        let term = TermImpl::from_instance(self);
        let env = &*term.env.borrow();
        let title = title
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}@{}:{}", env.user, env.host, env.cur_dir));
        let text = format!("{}. {}", page_number, title);

        gtk::Label::builder()
            .label(&text)
            .ellipsize(pango::EllipsizeMode::Middle)
            .expand(true)
            .build()
    }

    fn add_new_tab(&self) {
        let term = TermImpl::from_instance(self);
        let notebook = &*term.notebook.borrow();

        let terminal = new_terminal(&term.config.borrow());
        let page_number = notebook.n_pages() + 1;

        let label = self.page_label(page_number, None);

        let page = notebook.append_page::<_, gtk::Label>(&terminal, Some(&label));
        let children = notebook.children();

        let this = self.clone();

        terminal.connect_child_exited(glib::clone!(@weak this => move |term, _exit_code| {
            this.remove_tab(term);
        }));
        terminal.connect_window_title_notify(glib::clone!(@weak notebook => move |term| {
            if let Some(new_title) = term.window_title() {
                if let Some(num) = notebook.page_num(term) {
                    let label = this.page_label(num + 1, Some(&new_title));
                    notebook.set_tab_label(term, Some(&label));
                }
            }
        }));

        notebook.set_show_tabs(children.len() > 1);
        for child in &children {
            hacks::set_child_property(notebook.clone(), child.clone(), "tab-fill", true);
            hacks::set_child_property(notebook.clone(), child.clone(), "tab-expand", true);
        }
        notebook.show_all(); // can't switch page until child is shown
        notebook.set_current_page(Some(page));
        terminal.grab_focus();
    }
}

fn main() -> Result<(), Error> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("pterm")?;
    let config_path = xdg_dirs.place_config_file("config.toml")?;
    let config = match Config::from_file(&config_path) {
        Err(err) => {
            eprintln!(
                "error reading config file at {}: {}",
                config_path.display(),
                err
            );
            Config::from_str(DEFAULT_CONFIG).unwrap()
        }
        Ok(config) => config,
    };

    let app = Application::builder()
        .application_id("ru.plhk.pterm")
        .build();

    app.connect_activate(move |app| {
        let terminal_config = TerminalConfig {
            background: hacks::parse_color(&config.colors.background)
                .unwrap_or_else(|_| gdk::RGBA::black()),
            foreground: hacks::parse_color(&config.colors.foreground)
                .unwrap_or_else(|_| gdk::RGBA::white()),
            font: {
                let mut font = FontDescription::new();
                font.set_family(&config.font_family);
                font
            },
            palette: config
                .colors
                .palette
                .iter()
                .map(|color| hacks::parse_color(color).unwrap())
                .collect(),
        };

        let term = Term::new(app, Env::default(), terminal_config);

        term.show_all();
    });

    app.run();

    Ok(())
}
