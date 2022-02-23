use eframe::{egui, epi};
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

#[derive(Default)]
pub struct ExampleApp {
    url: String,
    frontend: Option<FrontEnd>,
}

impl epi::App for ExampleApp {
    fn name(&self) -> &str {
        "ewebsocket example app"
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        if let Some(web_info) = &frame.info().web_info {
            // allow `?url=` query param
            if let Some(url) = web_info.location.query_map.get("url") {
                self.url = url.clone()
            }
        }
        if self.url.is_empty() {
            self.url = "ws://echo.websocket.events/.ws".into(); // echo server
        }

        self.connect(frame.clone());
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        if !frame.is_web() {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            frame.quit();
                        }
                    });
                });
            });
        }

        egui::TopBottomPanel::top("server").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("URL:");
                if ui.text_edit_singleline(&mut self.url).lost_focus()
                    && ui.input().key_pressed(egui::Key::Enter)
                {
                    self.connect(frame.clone());
                }
            });
        });

        if let Some(frontend) = &mut self.frontend {
            frontend.ui(ctx);
        }
    }
}

impl ExampleApp {
    fn connect(&mut self, frame: epi::Frame) {
        let wakeup = move || frame.request_repaint(); // wake up UI thread on new message
        let (ws_sender, ws_receiver) = ewebsock::connect_with_wakeup(&self.url, wakeup).unwrap();
        self.frontend = Some(FrontEnd::new(ws_sender, ws_receiver));
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

    fn ui(&mut self, ctx: &egui::Context) {
        while let Some(event) = self.ws_receiver.try_recv() {
            self.events.push(event);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Message to send:");
                if ui.text_edit_singleline(&mut self.text_to_send).lost_focus()
                    && ui.input().key_pressed(egui::Key::Enter)
                {
                    self.ws_sender
                        .send(WsMessage::Text(std::mem::take(&mut self.text_to_send)));
                }
            });

            ui.separator();
            ui.heading("Received events:");
            for event in &self.events {
                ui.label(format!("{:?}", event));
            }
        });
    }
}
