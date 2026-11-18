use eframe::egui;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

pub struct ExampleApp {
    url: String,
    error: String,
    frontend: Option<FrontEnd>,
}

impl Default for ExampleApp {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:9001".to_owned(),
            error: Default::default(),
            frontend: None,
        }
    }
}

impl eframe::App for ExampleApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            egui::Panel::top("top_panel").show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });
        }

        egui::Panel::top("server").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("URL:");
                if ui.text_edit_singleline(&mut self.url).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    self.connect(ui.clone());
                }
            });
        });

        if !self.error.is_empty() {
            egui::Panel::top("error").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Error:");
                    ui.colored_label(egui::Color32::RED, &self.error);
                });
            });
        }

        if let Some(frontend) = &mut self.frontend {
            frontend.ui(ui);
        }
    }
}

impl ExampleApp {
    fn connect(&mut self, ctx: egui::Context) {
        let wakeup = move || ctx.request_repaint(); // wake up UI thread on new message
        match ewebsock::connect_with_wakeup(&self.url, Default::default(), wakeup) {
            Ok((ws_sender, ws_receiver)) => {
                self.frontend = Some(FrontEnd::new(ws_sender, ws_receiver));
                self.error.clear();
            }
            Err(error) => {
                log::error!("Failed to connect to {:?}: {}", &self.url, error);
                self.error = error;
            }
        }
    }
}

// ----------------------------------------------------------------------------

struct FrontEnd {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    events: Vec<WsEvent>,
    text_to_send: String,
}

impl FrontEnd {
    fn new(ws_sender: WsSender, ws_receiver: WsReceiver) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            events: Default::default(),
            text_to_send: Default::default(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        while let Some(event) = self.ws_receiver.try_recv() {
            self.events.push(event);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Message to send:");
                if ui.text_edit_singleline(&mut self.text_to_send).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    self.ws_sender
                        .send(WsMessage::Text(std::mem::take(&mut self.text_to_send)));
                }
            });

            ui.separator();
            ui.heading("Received events:");
            for event in &self.events {
                ui.label(format!("{event:?}"));
            }
        });
    }
}
