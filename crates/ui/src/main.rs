use essential_types::{
    solution::{InputMessage, KeyMutation, Mutation, OutputMessage, RangeMutation, StateMutation},
    IntentAddress,
};
use intent_server::{
    intent::{Intent, ToIntentAddress},
    solution::Solution,
    Server,
};
use yurtc::error::ReportableError;

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Essential Intent Server",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .unwrap();
}

pub struct App {
    intent: String,
    compiled_intents: Vec<Intent>,
    server: Server,
    window: Window,
    solution_editor: Solution,
    utility: Option<u64>,
    errors: Vec<String>,
}

enum Window {
    IntentEditor,
    SolutionEditor,
    ServerState,
}

impl App {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            window: Window::IntentEditor,
            intent: "solve satisfy;".to_string(),
            compiled_intents: Default::default(),
            server: Server::new(),
            solution_editor: Solution {
                data: Default::default(),
                state_mutations: Default::default(),
            },
            utility: None,
            errors: Default::default(),
        }
    }

    fn intent_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Intent Editor");
        ui.horizontal(|ui| {
            ui.label("Intent: ");
            ui.add(
                egui::TextEdit::multiline(&mut self.intent)
                    .font(egui::TextStyle::Monospace) // for cursor height
                    .code_editor()
                    .desired_rows(10)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal(|ui| {
            if ui.button("Compile").clicked() {
                if let Some(intent) = compile_intent(&self.intent) {
                    self.compiled_intents.push(intent);
                }
            }
        });

        ui.separator();

        for compiled_intent in &mut self.compiled_intents {
            show_intent(ui, compiled_intent);
        }

        if !self.compiled_intents.is_empty() {
            if self.compiled_intents.len() == 1 && ui.button("Submit").clicked() {
                let mut intent = self.compiled_intents[0].clone();
                intent.slots.output_messages = 1;
                if let Err(e) = self.server.submit_intent(intent) {
                    ui.monospace(format!("{}", e));
                }
            }
            if ui.button("Deploy").clicked() {
                let intents = self.compiled_intents.clone();
                let intents = intents
                    .into_iter()
                    .map(|mut i| {
                        i.slots.input_message_args = Some(vec![]);
                        i
                    })
                    .collect();
                match self.server.deploy_intent_set(intents) {
                    Ok(hash) => {
                        ui.monospace(format!("Deployed at: {:?}", hash));
                    }
                    Err(e) => {
                        ui.monospace(format!("{}", e));
                    }
                }
            }
            if ui.button("Clear").clicked() {
                self.compiled_intents.clear();
            }
        }
    }

    fn solution_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Solution Editor");
        ui.monospace("Choose intents to solve");

        let addresses: Vec<IntentAddress> = self.solution_editor.data.keys().cloned().collect();
        for mut address in addresses {
            let mut old = None;
            ui.monospace("Intent Address:");
            let mut val = to_hex(address.clone());
            ui.text_edit_singleline(&mut val);
            match from_hex(&val) {
                Some(a) => {
                    old = (a != address).then_some(address);
                    address = a
                }
                None => {
                    self.errors.push(format!("Invalid address: {}", val));
                }
            }
            if let Some(old) = old {
                if let Some(i) = self.solution_editor.data.remove(&old) {
                    self.solution_editor.data.insert(address.clone(), i);
                }
            }
            let data = &mut self.solution_editor.data.get_mut(&address).unwrap();
            for v in &mut data.decision_variables {
                num_line(ui, v);
            }
            if ui.button("Add Decision Variable").clicked() {
                data.decision_variables.push(0);
            }
            if let Some(input) = &mut data.input_message {
                ui.monospace("Input Message:");
                hex_line("Sender: ", ui, &mut input.sender);
                hex_line("Recipient: ", ui, &mut input.recipient);
                for (i, arg) in input.args.iter_mut().enumerate() {
                    ui.monospace(format!("Arg {}", i));
                    for arg in arg.iter_mut() {
                        num_line(ui, arg);
                    }
                    if ui.button("Add Word").clicked() {
                        arg.push(0);
                    }
                }
                if ui.button("Add Argument").clicked() {
                    input.args.push(Default::default());
                }
            }
            for output in &mut data.output_messages {
                ui.monospace("Output Message:");
                for (i, arg) in output.args.iter_mut().enumerate() {
                    ui.monospace(format!("Arg {}", i));
                    for arg in arg.iter_mut() {
                        num_line(ui, arg);
                    }
                    if ui.button("Add Word").clicked() {
                        arg.push(0);
                    }
                }
                if ui.button("Add Argument").clicked() {
                    output.args.push(Default::default());
                }
            }
            if data.input_message.is_none() && ui.button("Add Input Message").clicked() {
                data.input_message = Some(InputMessage {
                    sender: IntentAddress([0; 32]),
                    recipient: IntentAddress([0; 32]),
                    args: Default::default(),
                });
            }
            if ui.button("Add Output Message").clicked() {
                data.output_messages.push(OutputMessage {
                    args: Default::default(),
                });
            }
        }
        for (i, mutation) in self.solution_editor.state_mutations.iter_mut().enumerate() {
            ui.monospace(format!("Mutation: {}", i));
            hex_line("Address", ui, &mut mutation.address);

            for (i, m) in mutation.mutations.iter_mut().enumerate() {
                ui.monospace(format!("State: {}", i));
                match m {
                    Mutation::Key(KeyMutation { key, value }) => {
                        let mut k: IntentAddress = (*key).into();
                        hex_line("Key", ui, &mut k);
                        *key = k.into();

                        match value {
                            Some(v) => {
                                num_line(ui, v);
                                if ui.button("Delete").clicked() {
                                    *m = Mutation::Key(KeyMutation {
                                        key: *key,
                                        value: None,
                                    });
                                }
                            }
                            None => {
                                ui.monospace("Value: None");
                                if ui.button("Add Value").clicked() {
                                    *value = Some(0);
                                }
                            }
                        }
                    }
                    Mutation::Range(RangeMutation { key_range, values }) => {
                        let mut k: IntentAddress = (key_range.start).into();
                        hex_line("Key Start: ", ui, &mut k);
                        key_range.start = k.into();
                        let mut k: IntentAddress = (key_range.end).into();
                        hex_line("Key End: ", ui, &mut k);
                        key_range.end = k.into();

                        let mut to_remove = vec![];
                        for (i, value) in values.iter_mut().enumerate() {
                            ui.monospace(format!("Value {}", i));
                            match value {
                                Some(v) => {
                                    num_line(ui, v);
                                    if ui.button("Delete").clicked() {
                                        to_remove.push(i);
                                    }
                                }
                                None => {
                                    ui.monospace("Value: None");
                                    if ui.button("Add Value").clicked() {
                                        *value = Some(0);
                                    }
                                }
                            }
                        }
                        for i in to_remove.into_iter().rev() {
                            values.remove(i);
                        }
                    }
                }
            }

            if ui.button("Add Key Mutation").clicked() {
                mutation.mutations.push(Mutation::Key(KeyMutation {
                    key: [0; 4],
                    value: None,
                }));
            }
            if ui.button("Add Range Mutation").clicked() {
                mutation.mutations.push(Mutation::Range(RangeMutation {
                    key_range: [0; 4]..[0; 4],
                    values: Default::default(),
                }));
            }
        }

        if ui.button("Add Intent").clicked() {
            self.solution_editor
                .data
                .insert(IntentAddress([0; 32]), Default::default());
        }

        if ui.button("Add Mutation").clicked() {
            self.solution_editor.state_mutations.push(StateMutation {
                address: IntentAddress([0; 32]),
                mutations: Default::default(),
            })
        }

        if ui.button("Check").clicked() {
            match self.server.check(self.solution_editor.clone()) {
                Err(err) => {
                    self.errors.push(format!("{}", err));
                }
                Ok(utility) => self.utility = Some(utility),
            }
        }
        if ui.button("Solve").clicked() {
            match self.server.submit_solution(self.solution_editor.clone()) {
                Err(err) => {
                    self.errors.push(format!("{}", err));
                }
                Ok(utility) => self.utility = Some(utility),
            }
        }
        if let Some(utility) = self.utility {
            ui.monospace(format!("Utility: {}", utility));
        }
        if ui.button("Clear").clicked() {
            self.solution_editor = Solution {
                data: Default::default(),
                state_mutations: Default::default(),
            };
        }
    }

    fn server_state(&mut self, ui: &mut egui::Ui) {
        ui.heading("Server State");
        ui.heading("Submitted Intents");

        for (_, intent) in self.server.list_intents() {
            show_intent(ui, intent);
        }

        ui.separator();

        ui.heading("Deployed Intents");

        for (address, intents) in self.server.list_deployed() {
            let address: IntentAddress = (*address).into();
            ui.monospace(format!("Intent Set Address {}", to_hex(address)));
            for (_, intent) in intents {
                show_intent(ui, intent);
            }
        }

        ui.heading("Database State");

        for (i, k, v) in self.server.db().set_values() {
            let i: IntentAddress = i.into();
            let k: IntentAddress = k.into();
            ui.monospace(format!("{}: {} -> {}", to_hex(i), to_hex(k), v));
        }
    }
}

fn num_line(ui: &mut egui::Ui, arg: &mut u64) {
    let mut val = arg.to_string();
    ui.text_edit_singleline(&mut val);
    if let Ok(val) = val.parse() {
        *arg = val;
    }
}

fn hex_line(title: &str, ui: &mut egui::Ui, address: &mut IntentAddress) {
    ui.monospace(title);
    let mut val = to_hex(address.clone());
    ui.text_edit_singleline(&mut val);
    if let Some(a) = from_hex(&val) {
        *address = a;
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Intent Editor").clicked() {
                        self.window = Window::IntentEditor;
                    }
                    if ui.button("Solution Editor").clicked() {
                        self.window = Window::SolutionEditor;
                    }
                    if ui.button("Server State").clicked() {
                        self.window = Window::ServerState;
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.window {
                Window::IntentEditor => self.intent_editor(ui),
                Window::SolutionEditor => self.solution_editor(ui),
                Window::ServerState => self.server_state(ui),
            }
            ui.separator();
            for error in &self.errors {
                ui.monospace(error);
            }
            if ui.button("Clear Errors").clicked() {
                self.errors.clear();
            }
        });
    }
}

fn show_intent(ui: &mut egui::Ui, compiled_intent: &Intent) {
    ui.heading("Intent");
    ui.monospace(format!(
        "Address {}",
        to_hex(compiled_intent.intent_address())
    ));
    ui.monospace(format!("{:#?}", compiled_intent.slots));
    ui.monospace(format!(
        "Num constraints: {}",
        compiled_intent.constraints.len()
    ));
    ui.monospace(format!(
        "Num state reads: {}",
        compiled_intent.state_read.len()
    ));
    match compiled_intent.directive {
        intent_server::check::Directive::Satisfy => {
            ui.monospace("Directive: Satisfy");
        }
        intent_server::check::Directive::Maximize(_) => {
            ui.monospace("Directive: Maximize");
        }
        intent_server::check::Directive::Minimize(_) => {
            ui.monospace("Directive: Minimize");
        }
    }
}

fn compile_intent(code: &str) -> Option<Intent> {
    use std::io::Write;
    let mut tmpfile = tempfile::NamedTempFile::new().ok()?;
    write!(tmpfile.as_file_mut(), "{}", code).ok()?;
    let intent = match yurtc::parser::parse_project(tmpfile.path()) {
        Ok(intent) => intent,
        Err(errs) => {
            yurtc::error::print_errors(&errs);
            return None;
        }
    };
    let mut intent = match intent.flatten() {
        Ok(intent) => intent,
        Err(err) => {
            err.print();
            return None;
        }
    };
    let intent = match intent.compile() {
        Ok(intent) => intent,
        Err(err) => {
            err.print();
            return None;
        }
    };
    let intent = yurtc::asm_gen::intent_to_asm(&intent).ok()?;
    let intent = serde_json::to_string(&intent).ok()?;
    serde_json::from_str(&intent).ok()
}

fn to_hex<T: Into<[u8; 32]>>(t: T) -> String {
    let t = t.into();
    hex::encode(t)
}

fn from_hex(s: &str) -> Option<IntentAddress> {
    let bytes = hex::decode(s).ok()?;
    let bytes: [u8; 32] = bytes.try_into().ok()?;
    Some(IntentAddress(bytes))
}
