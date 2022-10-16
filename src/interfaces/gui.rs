use std::{
    fmt::Display,
    sync::mpsc::{channel, Receiver, Sender},
};

use tokio::runtime::Handle;

use crate::{
    dump,
    ipc::{self, IPCMessage},
    wilma::{
        self,
        models::{Course, OpenIDProvider, WilmaRole},
        Wilma, WilmaApi,
    },
};

use super::{Interface, InterfaceContext};

use eframe::egui::{self, Ui};

pub struct GuiInterface {
    _rt: Handle,
}

impl Interface for GuiInterface {
    fn new(handle: Handle) -> Self {
        Self { _rt: handle }
    }

    fn start(self, ctx: InterfaceContext) -> anyhow::Result<()> {
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            env!("CARGO_PKG_NAME"),
            native_options,
            Box::new(|cc| Box::new(GuiApp::new(cc, ctx))),
        );

        Ok(())
    }
}

#[derive(PartialEq)]
enum Dumper {
    Courses,
}

impl Display for Dumper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dumper::Courses => write!(f, "Courses"),
        }
    }
}

#[allow(clippy::enum_variant_names)]
enum AppMessage {
    WilmaList(Vec<Wilma>),
    WilmaProviderList(Option<Vec<OpenIDProvider>>),
    WilmaLogin(Box<Wilma>),
    WilmaRoles(Vec<WilmaRole>),
    WilmaCourses(Vec<Course>),
}

struct GuiApp {
    ctx: InterfaceContext,
    rx: Receiver<AppMessage>,
    tx: Sender<AppMessage>,

    wilma_list: Option<Vec<Wilma>>,
    wilma_providers: Option<Vec<OpenIDProvider>>,
    wilma_list_filter: String,
    wilma_roles: Option<Vec<WilmaRole>>,
    selected_wilma: Option<Wilma>,
    logging_in: bool,

    dumper: Option<Dumper>,

    courses_format: dump::courses::Format,
    courses: Option<Vec<Course>>,
    courses_path: String,
    courses_points: Option<(f32, f32)>,
}

impl GuiApp {
    fn new(_cc: &eframe::CreationContext<'_>, ctx: InterfaceContext) -> Self {
        let (tx, rx) = channel();
        Self {
            ctx,
            rx,
            tx,
            wilma_list: None,
            wilma_list_filter: String::new(),
            wilma_providers: None,
            wilma_roles: None,
            selected_wilma: None,
            logging_in: false,
            dumper: None,
            courses_format: dump::courses::Format::Json,
            courses: None,
            courses_path: String::new(),
            courses_points: None,
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.rx.try_recv() {
            Ok(AppMessage::WilmaList(wilma_list)) => {
                self.wilma_list = Some(wilma_list);
            }
            Ok(AppMessage::WilmaProviderList(providers)) => {
                self.wilma_providers = providers;
            }
            Ok(AppMessage::WilmaLogin(wilma)) => {
                self.logging_in = false;
                self.selected_wilma = Some(*wilma);

                let tx = self.tx.clone();
                let ctx = ctx.clone();
                let client = self.ctx.client.clone();
                let wilma = self.selected_wilma.as_ref().unwrap().clone();
                tokio::spawn(async move {
                    let roles = wilma.get_roles(&client).await.unwrap();
                    tx.send(AppMessage::WilmaRoles(roles)).unwrap();
                    ctx.request_repaint();
                });
            }
            Ok(AppMessage::WilmaRoles(roles)) => {
                self.wilma_roles = Some(roles);
            }
            Ok(AppMessage::WilmaCourses(courses)) => {
                self.courses = Some(courses);
            }
            Err(_) => {}
        }

        if self
            .selected_wilma
            .as_ref()
            .map_or(true, |w| !w.is_logged_in())
        {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Wilma Dumper");
                ui.separator();

                ui.add_enabled_ui(!self.logging_in, |ui| match &self.selected_wilma {
                    Some(w) => {
                        wilma_status(w, ui);
                        if ui.button("Change wilma").clicked() {
                            self.selected_wilma = None;
                            self.wilma_providers = None;
                        }
                        if let Some(providers) = &self.wilma_providers {
                            ui.separator();
                            ui.label("Select openid provider:");
                            for provider in providers {
                                if ui.button(provider.name.clone()).clicked() {
                                    self.logging_in = true;

                                    let tx = self.tx.clone();
                                    let ctx = ctx.clone();
                                    let mut wilma = self.selected_wilma.clone().unwrap();
                                    let client = self.ctx.client.clone();
                                    let provider = provider.clone();

                                    tokio::spawn(async move {
                                        wilma::auth::oauth_authorize(&client, &provider)
                                            .await
                                            .unwrap();
                                        match ipc::receive_data().await.unwrap() {
                                            IPCMessage::TokenResponse {
                                                access_token,
                                                id_token,
                                            } => wilma
                                                .openid_login(
                                                    &client,
                                                    provider.configuration.clone(),
                                                    provider.client_id.clone(),
                                                    access_token,
                                                    id_token,
                                                )
                                                .await
                                                .unwrap(),
                                            _ => unreachable!(),
                                        };

                                        tx.send(AppMessage::WilmaLogin(Box::new(wilma))).unwrap();
                                        ctx.request_repaint();
                                    });
                                }
                            }
                        }

                        if let Some(roles) = &self.wilma_roles {
                            ui.separator();
                            ui.label("Select role:");
                            for role in roles {
                                if ui
                                    .button(format!("{} ({})", role.name, role.slug))
                                    .clicked()
                                {
                                    self.selected_wilma.as_mut().unwrap().role = Some(role.clone());
                                }
                            }
                        }
                    }
                    None => {
                        ui.label("No Wilma selected");
                        wilma_selector(self, ctx.clone(), ui);
                    }
                })
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Wilma Dumper");
                ui.separator();

                wilma_status(self.selected_wilma.as_ref().unwrap(), ui);
                if ui.button("Change wilma").clicked() {
                    self.selected_wilma = None;
                    self.wilma_providers = None;
                }
                ui.separator();
                egui::ComboBox::from_label("Select dumper")
                    .selected_text(self.dumper.as_ref().map_or("".into(), |d| d.to_string()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.dumper, Some(Dumper::Courses), "Courses");
                    });
                ui.separator();
                match &self.dumper {
                    Some(Dumper::Courses) => courses_dumper(self, ctx, ui),
                    None => {}
                }
            });
        }
    }
}

fn courses_dumper(app: &mut GuiApp, ctx: &egui::Context, ui: &mut Ui) {
    ui.heading("Course dumper");
    if ui
        .button(if app.courses.is_some() {
            "Re-fetch courses"
        } else {
            "Fetch courses"
        })
        .clicked()
    {
        let tx = app.tx.clone();
        let ctx = ctx.clone();
        let client = app.ctx.client.clone();
        let wilma = app.selected_wilma.as_ref().unwrap().clone();
        tokio::spawn(async move {
            let courses = wilma.get_courses(&client).await.unwrap();
            tx.send(AppMessage::WilmaCourses(courses)).unwrap();
            ctx.request_repaint();
        });
    }
    ui.vertical(|ui| {
        ui.group(|ui| {
            egui::ComboBox::from_label("Select format")
                .selected_text(app.courses_format.clone().to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.courses_format,
                        dump::courses::Format::Json,
                        "Json",
                    );
                    ui.selectable_value(&mut app.courses_format, dump::courses::Format::Csv, "Csv");
                });
            ui.horizontal(|ui| {
                ui.label("File path");
                ui.text_edit_singleline(&mut app.courses_path);
            });
            ui.add_enabled_ui(app.courses.is_some(), |ui| {
                if ui.button("Dump").clicked() {
                    if let Ok(file) = std::fs::File::create(&app.courses_path) {
                        dump::courses::dump_to_writer(
                            app.courses.as_ref().unwrap(),
                            file,
                            app.courses_format.clone(),
                        )
                        .unwrap();
                        app.courses_path = String::new();
                    }
                }
            })
        });
        ui.vertical(|ui| {
            ui.label("Study credits");
            ui.add_enabled_ui(app.courses.is_some(), |ui| {
                if ui.button("Calculate").clicked() {
                    app.courses_points = Some(dump::courses::calculate_study_points(
                        app.courses.as_ref().unwrap(),
                    ))
                }
            });
            if app.courses_points.is_some() {
                let (total, earned) = app.courses_points.as_ref().unwrap();
                ui.label(format!("Not yet earned: {}", total - earned));
                ui.label(format!("Earned: {earned}",));
                ui.label(format!("Selected and earned: {total}"));
            }
        });
    });
}

fn wilma_status(w: &Wilma, ui: &mut Ui) {
    ui.label(format!("Selected Wilma: {}", w.name));
    ui.label(format!("Logged in: {}", w.is_authenticated()));
    ui.label(format!(
        "Role: {}",
        w.role
            .as_ref()
            .map(|r| format!("{} ({})", r.name, r.slug))
            .unwrap_or_else(|| "None".into())
    ));
}

fn wilma_selector(app: &mut GuiApp, ctx: egui::Context, ui: &mut Ui) {
    ui.add_enabled(
        app.wilma_list.is_some(),
        egui::TextEdit::singleline(&mut app.wilma_list_filter).hint_text("Filter Wilmas"),
    );
    egui::ScrollArea::new([false, true]).show(ui, |ui| {
        if let Some(wilmas) = &app.wilma_list {
            for wilma in wilmas {
                if !wilma
                    .name
                    .to_lowercase()
                    .contains(&app.wilma_list_filter.to_lowercase())
                {
                    continue;
                }
                ui.horizontal(|ui| {
                    if ui.button(wilma.name.clone()).clicked() {
                        app.selected_wilma = Some(wilma.clone());

                        let tx = app.tx.clone();
                        let client = app.ctx.client.clone();
                        let ctx = ctx.clone();
                        let wilma = wilma.clone();

                        tokio::spawn(async move {
                            let wilma_providers = wilma.get_providers(&client).await.unwrap();
                            tx.send(AppMessage::WilmaProviderList(wilma_providers))
                                .unwrap();
                            ctx.request_repaint();
                        });
                    }
                });
            }
        } else if ui.button("Fetch wilmas").clicked() {
            let tx = app.tx.clone();
            let client = app.ctx.client.clone();
            let ctx = ctx.clone();

            tokio::spawn(async move {
                if let Ok(wilmas) = wilma::get_wilmas(&client).await {
                    tx.send(AppMessage::WilmaList(wilmas)).unwrap();
                    ctx.request_repaint();
                }
            });
        }
    });
}
