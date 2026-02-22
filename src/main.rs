#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, toggler
};
use serde_json::Error as SerdeError;
use iced::{Subscription, time};
use std::time::{Duration, Instant};
use iced::{Element, Length, Task, Theme, Color, Background, Alignment, Font, font::Weight, Padding};
use std::io;
mod managers;
use managers::data_manager::{FiveGRecord,GridSquare, GsmRecord, LteRecord,process_dataset, get_grid_map,create_protocol,process_point_dataset, process_multiple_points_dataset, MobilePathProvider};
use managers::json_manager::{try_read_json,create_blank_json, save_json};

const folder_name: &str = "App-zony-100m";

fn main() -> iced::Result {
    let mut logs: Vec<(String, TerminalMessageType)> = Vec::new();
    let settings_result = try_read_json(folder_name); // Premenoval som to na result pre prehľadnosť

    let settings = match settings_result {
        Ok(data) => {
            logs.push(("Systém pripravený.".to_string(), TerminalMessageType::Success));
            data
        }, 
        
        Err(error) => {
            if let Some(io_err) = error.downcast_ref::<io::Error>() {
                if io_err.kind() == io::ErrorKind::NotFound {
                        if let Err(e) = create_blank_json(folder_name) {
                            logs.push((format!("Chyba pri vytvorení súboru config.json: {}", e).to_string(), TerminalMessageType::Error));
                            logs.push(("Nastavenia sa nebudú ukladať!!!".to_string(), TerminalMessageType::Warning));
                        }
                } else {
                    logs.push((format!("Chyba: {}", error).to_string(), TerminalMessageType::Error));
                }
            } else if let Some(serde_err) = error.downcast_ref::<SerdeError>() {
                if serde_err.is_data() && serde_err.to_string().contains("missing field") {
                    let _ = save_json(folder_name, &AppSettings::default());
                } else {
                    logs.push((
                        format!("Chyba v JSON formáte: {}", serde_err),
                        TerminalMessageType::Error,
                    ));
                }
            }
            else {
                logs.push((format!("Neznáma chyba: {}", error), TerminalMessageType::Error));
            }

            logs.push(("Systém pripravený.".to_string(), TerminalMessageType::Success));
            AppSettings::default()
        }
    };
    

    iced::application(
        move || AppState::new(settings.clone(),logs.clone()), 
        AppState::update,
        AppState::view,
    )
    .title("App")
    .subscription(AppState::subscription)
    .theme(AppState::theme)
    .run()
}

#[derive(Serialize,Deserialize, Clone)]
pub struct AppSettings {
    pub zone_file_path: String,
    pub filter_lte_path: String,
    pub filter_5g_path: String,
    pub protokol_path: String,
    pub protocol_points_path: String,
    pub protocol_mobile_path: String,

    pub is_dark_mode: bool,
    pub is_terminal_open: bool,
    pub generate_missing_operators: bool,
    pub use_lte_filter: bool,
    pub use_5g_filter: bool,
    pub use_multiple_filters: bool,
    pub use_protocol_points: bool,
}

impl AppSettings {
    pub fn default() -> Self{
        Self {
            is_dark_mode: true,
            is_terminal_open: true,
            zone_file_path: "".to_string(),
            filter_lte_path: "".to_string(),
            filter_5g_path: "".to_string(),
            protokol_path: "".to_string(),
            protocol_points_path: "".to_string(),
            protocol_mobile_path: "".to_string(),
            generate_missing_operators: true,
            use_lte_filter: true,
            use_5g_filter: true,
            use_multiple_filters: true,
            use_protocol_points: false,
        }
    }
}

struct AppState {
    settings: AppSettings,
    logs: Vec<(String, TerminalMessageType)>,
    multiple_paths: Vec<String>,
    is_generating: bool,
    active_screen: Screen,
    first_path: String,
    second_path: String,
    third_path: String,
    output_path: String,
    second_output_path: String,
    is_timer_running: bool,
    measured_city: String,
    total_power: String,
    sinr: String,
    rsrp: String,
    antenna_height: String,
    internal_environment: String,
    max_distance_of_point: String,
    threshold_rsrp: String,
    threshold_sinr: String,
    selected_dropdown: SelectedDropDown,
    mobile_paths: Vec<MobilePathEntry>,
}

#[derive(Debug, Clone)]
struct MobilePathEntry {
    lte_path: String,
    g5_path: String,
}

impl MobilePathEntry {
    pub fn lte_pathbuf(&self) -> PathBuf {
        PathBuf::from(self.lte_path.clone())
    }

    pub fn g5_pathbuf(&self) -> PathBuf {
        PathBuf::from(self.g5_path.clone())
    }
}

impl MobilePathProvider for MobilePathEntry {
    fn lte_pathbuf(&self) -> PathBuf {
        self.lte_pathbuf()
    }

    fn g5_pathbuf(&self) -> PathBuf {
        self.g5_pathbuf()
    }
}

#[derive(Debug, Clone, Copy)]
enum ProtocolInputType {
    City,
    TotalPower,
    Sinr,
    Rsrp,
    AntennaHeight,
    InternalEnv,
    MaxDistance,
    ThresholdRsrp,
    ThresholdSinr
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    ToggleTerminal,
    ToggleScreen(Screen),
    PathChanged(FileTarget,String),
    ToggleChanged(ToggleTarget,bool),
    SelectFileClicked {
        target: FileTarget,
        filter_name: &'static str,
        extensions: &'static [&'static str],
    },
    SelectFolderClicked { target: FileTarget },
    SaveFileClicked{
        target: FileTarget,
        default_name: &'static str,
        filter_name: &'static str,
        extensions: &'static [&'static str],
    },
    FilePicked(FileTarget, Option<String>),
    GenerateClicked,
    SaveTick(Instant),  
    GenerationFinished(Result<(), String>),
    ProtocolInputChanged(ProtocolInputType, String),
    ToggleTimer,
    RemovePath(usize), 
    AddMobilePathEntry,
    RemoveMobilePathEntry(usize),
    SelectedDropDownChanged(SelectedDropDown),
}

#[derive(Debug, Clone)]
enum TerminalMessageType {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
enum SelectedDropDown {
    None,
    Zone,
    Points,
    Protocols,
}

#[derive(Debug, Clone)]
enum Screen {
    GSM,
    LTE_Zone,
    LTE_Point,
    Protocol_LTE,
    Protocol_5G,
    _5G_Zone,
    _5G_Point,
    Mobile_Point,
    Settings
}

#[derive(Debug, Clone, Copy)]
enum FileTarget {
    FirstPath,
    SecondPath,
    ThirdPath,
    OutputPath,
    SecondOutputPath,
    ZonePath,
    FilterLTEPath,
    Filter5GPath,
    ProtokolPath,
    MultiplePaths,
    ProtocolPointsPath,
    ProtocolMobilePath,
    MobileLtePath(usize),
    Mobile5GPath(usize),
}

#[derive(Debug, Clone, Copy)]
enum ToggleTarget {
    GenerateMissingOperators,
    UseLTEFilter,
    Use5GFilter,
    UseMultipleFilters,
    UseProtocolPoints,
}

impl AppState {
    fn new(settings: AppSettings, logs: Vec<(String, TerminalMessageType)>) -> (Self, Task<Message>) {
        (
            Self {
                settings: settings,
                logs: logs,
                is_generating: false,
                active_screen: Screen::GSM,
                multiple_paths: Vec::new(),
                first_path: String::new(),
                second_path: String::new(),
                third_path: String::new(),
                output_path: String::new(),
                second_output_path: String::new(),
                is_timer_running: false,
                selected_dropdown: SelectedDropDown::None,
                mobile_paths: Vec::new(),

                measured_city: String::new(),
                total_power: "".to_string(), // Predvolená hodnota
                sinr: "".to_string(),
                rsrp: "".to_string(),
                antenna_height: "".to_string(),
                internal_environment: "".to_string(),
                max_distance_of_point: "".to_string(),
                threshold_rsrp: "".to_string(),
                threshold_sinr: "".to_string()
            },
            Task::none(),
        )
    }

    fn reset_attributes(&mut self) {
        self.first_path.clear();
        self.output_path.clear();
        self.second_output_path.clear();
        self.threshold_rsrp.clear();
        self.threshold_sinr.clear();
        self.reset_console();
    }

    fn reset_console(&mut self) {
        self.logs.clear();
        self.logs.push(("Systém pripravený.".to_string(), TerminalMessageType::Success));
    }

    fn update(&mut self, message: Message) -> Task<Message> {
    match message {
            Message::SelectFileClicked { target, filter_name, extensions } => {
                self.logs.push(("Otváram dialóg...".to_string(), TerminalMessageType::Info));
                
                // Task::future spustí kód na inom vlákne, takže UI nezamrzne
                Task::future(async move {
                    let result = rfd::AsyncFileDialog::new() // Použijeme asynchrónny RFD
                        .add_filter(filter_name, extensions)
                        .pick_file()
                        .await; // Tu čakáme, ale len na tomto pozadí

                    let path = result.map(|handle| handle.path().to_string_lossy().to_string());
                    Message::FilePicked(target, path)
                })
            }

            Message::ProtocolInputChanged(input_type, value) => {
                match input_type {
                    // 1. PRÍPAD: Mesto - tu chceme povoliť písmená, takže žiadna validácia na čísla
                    ProtocolInputType::City => {
                        self.measured_city = value; 
                    },

                    // 2. PRÍPAD: Všetky ostatné polia (TotalPower, Sinr...) - tu chceme iba čísla
                    _ => {
                        // Kontrola: je to prázdne ALEBO sú to len čísla/bodka/mínus?
                        let is_number = value.is_empty() || value.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-');

                        if is_number {
                            match input_type {
                                ProtocolInputType::TotalPower => self.total_power = value,
                                ProtocolInputType::Sinr => self.sinr = value,
                                ProtocolInputType::Rsrp => self.rsrp = value,
                                ProtocolInputType::AntennaHeight => self.antenna_height = value,
                                ProtocolInputType::InternalEnv => self.internal_environment = value,
                                ProtocolInputType::MaxDistance => self.max_distance_of_point = value,
                                ProtocolInputType::ThresholdRsrp => self.threshold_rsrp = value,
                                ProtocolInputType::ThresholdSinr => self.threshold_sinr = value,
                                _ => {} // City už je vyriešené vyššie, sem sa nedostane
                            }
                        }
                    }
                }
                Task::none()
            }

            Message::SelectFolderClicked { target } => {
                self.logs.push(("Otváram výber priečinka...".to_string(), TerminalMessageType::Info));
                
                Task::future(async move {
                    let result = rfd::AsyncFileDialog::new()
                        .pick_folder() // <--- Tu je hlavná zmena, vyberáme priečinok
                        .await;

                    let path = result.map(|handle| handle.path().to_string_lossy().to_string());
                    
                    // Zrecyklujeme existujúcu správu FilePicked, keďže tá len berie cestu string
                    Message::FilePicked(target, path) 
                })
            }

            Message::SaveFileClicked { target, default_name, filter_name, extensions } => {
                self.logs.push(("Otváram ukladací dialóg...".to_string(), TerminalMessageType::Info));
                
                Task::future(async move {
                    let result = rfd::AsyncFileDialog::new()
                        .set_file_name(default_name)
                        .add_filter(filter_name, extensions)
                        .save_file()
                        .await;

                    let path = result.map(|handle| handle.path().to_string_lossy().to_string());
                    
                    Message::FilePicked(target, path)
                })
            }

            Message::FilePicked(target, path_opt) => {
                let mut trigger_timmer = false;
                if let Some(path) = path_opt {
                    match target {
                        FileTarget::FirstPath => self.first_path = path.clone(),
                        FileTarget::OutputPath => self.output_path = path.clone(),
                        FileTarget::SecondOutputPath => self.second_output_path = path.clone(),
                        FileTarget::SecondPath => self.second_path = path.clone(),
                        FileTarget::ThirdPath => self.third_path = path.clone(),
                        FileTarget::ZonePath => {
                            self.settings.zone_file_path = path.clone();
                            trigger_timmer = true;
                        }
                        FileTarget::FilterLTEPath => {
                            self.settings.filter_lte_path = path.clone();
                            trigger_timmer = true;
                        }
                        FileTarget::Filter5GPath => {
                            self.settings.filter_5g_path = path.clone();
                            trigger_timmer = true;
                        }
                        FileTarget::ProtokolPath => {
                            self.settings.protokol_path = path.clone();
                            trigger_timmer = true;
                        }
                            FileTarget::MultiplePaths => {
                            if !self.multiple_paths.contains(&path) {
                                self.multiple_paths.push(path.clone());
                            }
                        }
                        FileTarget::ProtocolPointsPath => {
                            self.settings.protocol_points_path = path.clone();
                            trigger_timmer = true;
                        }
                        FileTarget::ProtocolMobilePath => {
                            self.settings.protocol_mobile_path = path.clone();
                            trigger_timmer = true;
                        }
                        FileTarget::MobileLtePath(index) => {
                            if let Some(entry) = self.mobile_paths.get_mut(index) {
                                entry.lte_path = path.clone();
                            }
                        }
                        FileTarget::Mobile5GPath(index) => {
                            if let Some(entry) = self.mobile_paths.get_mut(index) {
                                entry.g5_path = path.clone();
                            }
                        }
                    }
                    self.logs.push((format!("Cesta vybraná: {}", path), TerminalMessageType::Success));
                } else {
                    self.logs.push(("Výber zrušený.".to_string(), TerminalMessageType::Warning));
                }
                if trigger_timmer {
                    Task::done(Message::ToggleTimer)
                } else {
                    Task::none()
                }
            }
            
            Message::PathChanged(target,val) => {
                match target {
                    FileTarget::FirstPath => {self.first_path = val}
                    FileTarget::SecondPath => {self.second_path = val}
                    FileTarget::ThirdPath => {self.third_path = val}
                    FileTarget::OutputPath => {self.output_path = val}
                    FileTarget::ZonePath => {
                        self.settings.zone_file_path = val;
                        return Task::done(Message::ToggleTimer);
                    }
                    FileTarget::FilterLTEPath => {
                        self.settings.filter_lte_path = val;
                        return Task::done(Message::ToggleTimer);
                    }
                    FileTarget::Filter5GPath => {
                        self.settings.filter_5g_path = val;
                        return Task::done(Message::ToggleTimer);
                    }
                    FileTarget::ProtokolPath => {
                        self.settings.protokol_path = val;
                        return Task::done(Message::ToggleTimer);
                    }
                    FileTarget::ProtocolPointsPath => {
                        self.settings.protocol_points_path = val;
                        return Task::done(Message::ToggleTimer);
                    }
                    FileTarget::ProtocolMobilePath => {
                        self.settings.protocol_mobile_path = val;
                        return Task::done(Message::ToggleTimer);
                    }
                    FileTarget::MobileLtePath(index) => {
                        if let Some(entry) = self.mobile_paths.get_mut(index) {
                            entry.lte_path = val;
                        }
                    }
                    FileTarget::Mobile5GPath(index) => {
                        if let Some(entry) = self.mobile_paths.get_mut(index) {
                            entry.g5_path = val;
                        }
                    }
                    _ => ()
                }
                Task::none()},
            Message::ToggleTheme => {self.settings.is_dark_mode = !self.settings.is_dark_mode; Task::done(Message::ToggleTimer) },
            Message::ToggleTerminal => {self.settings.is_terminal_open = !self.settings.is_terminal_open; Task::done(Message::ToggleTimer) },
            Message::ToggleChanged(target,val) => {
                match target {
                    ToggleTarget::GenerateMissingOperators => {self.settings.generate_missing_operators = val}
                    ToggleTarget::UseLTEFilter => {self.settings.use_lte_filter = val}
                    ToggleTarget::Use5GFilter => {self.settings.use_5g_filter = val}
                    ToggleTarget::UseMultipleFilters => {self.settings.use_multiple_filters = val}
                    ToggleTarget::UseProtocolPoints => {self.settings.use_protocol_points = val}
                }

                Task::done(Message::ToggleTimer)
            },
            Message::SelectedDropDownChanged(dropdown) => {
                self.selected_dropdown = dropdown;
                Task::none()
            },

            Message::ToggleScreen(screen) => {self.active_screen = screen;self.reset_attributes(); Task::none()},
            Message::GenerateClicked => {
                self.is_generating = true;
                let screen = self.active_screen.clone();
                let generate_missing_operators = self.settings.generate_missing_operators.clone();
                let use_lte_filter = self.settings.use_lte_filter.clone();
                let use_5g_filter = self.settings.use_5g_filter.clone();
                let use_multiple_filters = self.settings.use_multiple_filters.clone();
                let filter_lte_path = PathBuf::from(self.settings.filter_lte_path.clone());
                let filter_5g_path = PathBuf::from(self.settings.filter_5g_path.clone());
                let grid_path = PathBuf::from(self.settings.zone_file_path.clone());
                let first_path = PathBuf::from(self.first_path.clone()); // Predpokladám, že first_path je String
                let second_path = PathBuf::from(self.second_path.clone());
                let third_path = PathBuf::from(self.third_path.clone());
                let output_path = PathBuf::from(self.output_path.clone());
                let second_output_path = PathBuf::from(self.second_output_path.clone());
                let protocol_path = PathBuf::from(self.settings.protokol_path.clone());
                let protocol_mobile_path = PathBuf::from(self.settings.protocol_mobile_path.clone());
                let measured_city = self.measured_city.clone();
                let total_power: f32 = self.total_power.parse().unwrap_or(0.0);
                let sinr: f32 = self.sinr.parse().unwrap_or(0.0);
                let rsrp: f32 = self.rsrp.parse().unwrap_or(0.0);
                let threshold_sinr: f32 = self.threshold_sinr.parse().unwrap_or(0.0);
                let threshold_rsrp: f32 = self.threshold_rsrp.parse().unwrap_or(0.0);
                let antenna_height: f32 = self.antenna_height.parse().unwrap_or(0.0);
                let internal_environment: f32 = self.internal_environment.parse().unwrap_or(0.0);
                let max_distance: f64 = self.max_distance_of_point.parse().unwrap_or(0.0);
                let multiple_paths: Vec<PathBuf> = self.multiple_paths
                    .iter()
                    .map(PathBuf::from)
                    .collect();
                let mobile_paths = self.mobile_paths.clone();
                if max_distance <= 0.0 && matches!(screen, Screen::_5G_Point | Screen::LTE_Point){
                    self.logs.push(("Veľkosť bodu musí byť väčšie ako 0.".to_string(), TerminalMessageType::Error));
                    self.is_generating = false;
                    return Task::none();
                }
                 let protocol_point_path = if matches!(screen, Screen::_5G_Point | Screen::LTE_Point) && self.settings.use_protocol_points {
                    Some(PathBuf::from(self.settings.protocol_points_path.clone()))
                } else {
                    None
                };
                self.logs.push((format!("Generujem výstup do: {}", self.output_path), TerminalMessageType::Warning));
                return Task::perform(
            async move {
                // Keďže CSV operácie sú blokujúce (sync), musíme ich spustiť v inom vlákne,
                // aby nezamrzlo GUI. Najlepšie cez tokio::task::spawn_blocking.
                tokio::task::spawn_blocking(move || {
                    match screen {
                        Screen::GSM => process_dataset::<GsmRecord>(grid_path,first_path, output_path,generate_missing_operators, false, filter_lte_path),
                        Screen::LTE_Zone => process_dataset::<LteRecord>(grid_path,first_path, output_path,generate_missing_operators, use_lte_filter,filter_lte_path),
                        Screen::Protocol_LTE => {
                            create_protocol(protocol_path, first_path, second_path,None ,output_path,measured_city, total_power, sinr, rsrp, antenna_height, internal_environment)
                        }
                        Screen::Protocol_5G => {
                            create_protocol(protocol_path, first_path, second_path,Some(third_path) ,output_path,measured_city, total_power, sinr, rsrp, antenna_height, internal_environment)
                        }
                        Screen::_5G_Zone => process_dataset::<FiveGRecord>(grid_path,first_path, output_path,generate_missing_operators, use_5g_filter,filter_5g_path),
                        Screen::_5G_Point => process_point_dataset::<FiveGRecord>(multiple_paths, output_path, filter_5g_path, protocol_point_path, second_output_path, use_5g_filter, max_distance, generate_missing_operators,threshold_sinr,threshold_rsrp),
                        Screen::LTE_Point => process_point_dataset::<LteRecord>(multiple_paths, output_path, filter_lte_path,protocol_point_path, second_output_path, use_lte_filter, max_distance,  generate_missing_operators,threshold_sinr,threshold_rsrp),
                        Screen::Mobile_Point => process_multiple_points_dataset::<MobilePathEntry>(mobile_paths, output_path, filter_lte_path, filter_5g_path, Some(protocol_mobile_path), second_output_path, use_multiple_filters, max_distance, generate_missing_operators, threshold_sinr, threshold_rsrp),
                        _ => Err("Nepodporovaný typ obrazovky".to_string()),
                    }
                })
                .await // Počkáme na vlákno (non-blocking pre UI)
                .map_err(|e| e.to_string()) // Chyba threadu (JoinError)
                .and_then(|res| res)        // Chyba z process_dataset
            },
            Message::GenerationFinished, // Keď to skončí, pošli túto správu
        );
            }
            Message::GenerationFinished(result) => {
                self.is_generating = false;
                match result {
                    Ok(_) => self.logs.push(("Generovanie dokončené úspešne.".to_string(), TerminalMessageType::Success)),
                    Err(e) => self.logs.push((format!("Chyba počas generovania: {}", e), TerminalMessageType::Error)),
                }
                Task::none()
            }
            Message::ToggleTimer => {
                //if !self.is_timer_running {self.logs.push(("Spúštam časovač...".to_string(), TerminalMessageType::Info));}
                self.is_timer_running = true;
                Task::none()
            }
            Message::SaveTick(_instance) => {
                //self.logs.push(("Ukladam nastavenia...".to_string(), TerminalMessageType::Info));
                if let Err(error) = save_json(folder_name, &self.settings){
                    self.logs.push((format!("Nepodarilo sa nastavenia uložit nastavenia. Error: {}", error), TerminalMessageType::Warning));
                }
                self.is_timer_running = false;
                Task::none()
            }
            Message::RemovePath(index) => {
            if index < self.multiple_paths.len() {
                let removed = self.multiple_paths.remove(index);
                self.logs.push((format!("Odobraný súbor: {}", removed), TerminalMessageType::Info));
            }
            Task::none()
        }
            Message::AddMobilePathEntry => {
                self.mobile_paths.push(MobilePathEntry {
                    lte_path: String::new(),
                    g5_path: String::new(),
                });
                Task::none()
            }
            Message::RemoveMobilePathEntry(index) => {
                if index < self.mobile_paths.len() {
                    self.mobile_paths.remove(index);
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let theme_text = if self.settings.is_dark_mode { "Light Mode" } else { "Dark Mode" };
        let term_text = if self.settings.is_terminal_open { "Skryť Logy" } else { "Zobraziť Logy" };

        // Definícia tučného písma pre moderné UI
        let bold_font = Font {
            weight: Weight::Bold,
            ..Default::default()
        };

        // --- SIDEBAR ---
        let sidebar = container(
            column![
                text("Nástroje").size(24).font(bold_font),
                
                // --- HORNÁ ČASŤ (MENU) ---
                column![
                    // 2. DROPDOWN: ZÓNY
                    column![
                        button(
                            row![
                                text("Zóny").size(16),
                                container(row![]).width(Length::Fill),
                                container(
                                text(if matches!(self.selected_dropdown, SelectedDropDown::Zone) { "▾" } else { "▸" }).size(20)
                            ).height(Length::Fixed(20.0))
                            .align_y(Alignment::Center)
                            .padding(Padding { top: 1.0, right: 0.0, bottom: 0.0, left: 0.0 }) 
                            ]
                            .width(Length::Fill)
                            .align_y(Alignment::Center)
                        )
                        .on_press(Message::SelectedDropDownChanged(
                            if matches!(self.selected_dropdown, SelectedDropDown::Zone) {
                                SelectedDropDown::None
                            } else {
                                SelectedDropDown::Zone
                            }
                        ))
                        .width(Length::Fill)
                        .padding(10),

                        if matches!(self.selected_dropdown, SelectedDropDown::Zone) {
                            column![
                                button(text("GSM").size(16)).on_press(Message::ToggleScreen(Screen::GSM)).width(Length::Fill),
                                button("LTE").on_press(Message::ToggleScreen(Screen::LTE_Zone)).width(Length::Fill),
                                button("5G").on_press(Message::ToggleScreen(Screen::_5G_Zone)).width(Length::Fill),
                            ]
                            .spacing(5)
                            .padding(Padding { top: 5.0, right: 0.0, bottom: 5.0, left: 20.0 })
                        } else {
                            column![]
                        }
                    ].spacing(5),

                    // 3. DROPDOWN: BODY
                    column![
                        button(
                            row![
                                text("Body").size(16),
                                container(row![]).width(Length::Fill),
                                container(
                                text(if matches!(self.selected_dropdown, SelectedDropDown::Points) { "▾" } else { "▸" }).size(20)
                            ).height(Length::Fixed(20.0))
                            .align_y(Alignment::Center)
                            .padding(Padding { top: 1.0, right: 0.0, bottom: 0.0, left: 0.0 }) 
                            ]
                            .width(Length::Fill)
                            .align_y(Alignment::Center)
                        )
                        .on_press(Message::SelectedDropDownChanged(
                            if matches!(self.selected_dropdown, SelectedDropDown::Points) {
                                SelectedDropDown::None
                            } else {
                                SelectedDropDown::Points
                            }
                        ))
                        .width(Length::Fill)
                        .padding(10),

                        if matches!(self.selected_dropdown, SelectedDropDown::Points) {
                            column![
                                button("LTE").on_press(Message::ToggleScreen(Screen::LTE_Point)).width(Length::Fill),
                                button("5G").on_press(Message::ToggleScreen(Screen::_5G_Point)).width(Length::Fill),
                                button("5G Mobil").on_press(Message::ToggleScreen(Screen::Mobile_Point)).width(Length::Fill),
                            ]
                            .spacing(5)
                            .padding(Padding { top: 5.0, right: 0.0, bottom: 5.0, left: 20.0 })
                        } else {
                            column![]
                        }
                    ].spacing(5),

                    // 4. DROPDOWN: PROTOKOLY
                    column![
                        button(
                            row![
                                text("Protokoly").size(16),
                                container(row![]).width(Length::Fill),
                                container(
                                text(if matches!(self.selected_dropdown, SelectedDropDown::Protocols) { "▾" } else { "▸" }).size(20)
                            ).height(Length::Fixed(20.0))
                            .align_y(Alignment::Center)
                            .padding(Padding { top: 1.0, right: 0.0, bottom: 0.0, left: 0.0 }) 
                            ]
                            .width(Length::Fill)
                            .align_y(Alignment::Center)
                        )
                        .on_press(Message::SelectedDropDownChanged(
                            if matches!(self.selected_dropdown, SelectedDropDown::Protocols) {
                                SelectedDropDown::None
                            } else {
                                SelectedDropDown::Protocols
                            }
                        ))
                        .width(Length::Fill)
                        .padding(10),

                        if matches!(self.selected_dropdown, SelectedDropDown::Protocols) {
                            column![
                                button("LTE").on_press(Message::ToggleScreen(Screen::Protocol_LTE)).width(Length::Fill),
                                button("5G").on_press(Message::ToggleScreen(Screen::Protocol_5G)).width(Length::Fill),
                            ]
                            .spacing(5)
                            .padding(Padding { top: 5.0, right: 0.0, bottom: 5.0, left: 20.0 })
                        } else {
                            column![]
                        }
                    ].spacing(5),
                ].spacing(10),

                // --- SPACER (vytlačí spodok dole) ---
                container(column![].height(Length::Fill)), 
                
                // --- SPODNÁ ČASŤ (NASTAVENIA, TERMINÁL, TÉMA) ---
                column![
                    // Tlačidlo Nastavenia
                    button(
                        row![
                            text("Nastavenia").size(16),
                            container(row![]).width(Length::Fill),
                            container(
                            text("⚙").size(16)).height(Length::Fixed(20.0))
                            .align_y(Alignment::Center)
                            .padding(Padding { top: 1.0, right: 0.0, bottom: 0.0, left: 0.0 }) 
                        ]
                        .width(Length::Fill)
                        .align_y(Alignment::Center)
                    )
                    .on_press(Message::ToggleScreen(Screen::Settings))
                    .width(Length::Fill)
                    .padding(10),

                    // Tlačidlo Terminál
                    button(term_text)
                        .on_press(Message::ToggleTerminal)
                        .width(Length::Fill),

                    // Tlačidlo Téma
                    button(theme_text)
                        .on_press(Message::ToggleTheme)
                        .width(Length::Fill),
                ].spacing(10)
            ]
            .spacing(30)
        )
        .width(220)
        .height(Length::Fill)
        .padding(20)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.05))),
            border: iced::border::Border {
                color: Color::from_rgb(0.3, 0.3, 0.3),
                width: 1.0, 
                radius: 0.0.into(),
            },
            ..container::Style::default()
        });

        // --- HLAVNÝ OBSAH ---
        let main_content = match self.active_screen {  
            Screen::GSM => {container(scrollable(
            column![
                text("Konfigurácia GSM Modulu").size(32).font(bold_font),
                
                // Riadok 1: GSM File Path
                row![
                    text("Zony File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.settings.zone_file_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ZonePath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                                                target: FileTarget::ZonePath,
                                                filter_name: "CSV Súbory",
                                                extensions: &["csv" ],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("GSM File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.first_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FirstPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                                                target: FileTarget::FirstPath,
                                                filter_name: "CSV Súbory",
                                                extensions: &["csv" ],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Riadok 2: Output Path
                row![
                    text("Output Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                                                target: FileTarget::OutputPath,
                                                default_name: "zony.csv",
                                                filter_name: "CSV súbory",
                                                extensions: &["csv"],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Riadok 3: Toggles a Generate
                row![
                    text("Vygenerovať chýbajúcich operátorov:").size(16),
                    toggler(self.settings.generate_missing_operators).on_toggle(|value| Message::ToggleChanged(ToggleTarget::GenerateMissingOperators, value))
                    //container(column![].width(Length::Fill)), 

                    
                ].spacing(30).align_y(Alignment::Center),

                row![
                    {let btn = button(text("GENERATE").size(16).font(bold_font))
                        .padding([12, 40])
                        .style(button::primary);
                    if self.is_generating {
                        btn // Vrátime tlačidlo bez on_press -> bude sivé a neklikateľné
                    } else {
                        btn.on_press(Message::GenerateClicked) // Pridáme akciu -> bude aktívne
                    }
                    }
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::LTE_Zone => {container(scrollable(
            column![
                text("Konfigurácia LTE Zony Modulu").size(32).font(bold_font),

                row![
                    text("Zony File Path:").width(175).size(16),
                    text_input("Cesta k súboru...", &self.settings.zone_file_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ZonePath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                                                target: FileTarget::ZonePath,
                                                filter_name: "CSV Súbory",
                                                extensions: &["csv" ],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Filtre Operátorov Folder:").width(175).size(16),
                    text_input("Cesta k priečinku...", &self.settings.filter_lte_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FilterLTEPath, text))
                        .padding(10),
                    button("Select Folder").on_press(Message::SelectFolderClicked {
                                                target: FileTarget::FilterLTEPath
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Riadok 1: LTE File Path
                row![
                    text("LTE File Path:").width(175).size(16),
                    text_input("Cesta k súboru...", &self.first_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FirstPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                                                target: FileTarget::FirstPath,
                                                filter_name: "CSV Súbory",
                                                extensions: &["csv" ],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Riadok 2: Output Path
                row![
                    text("Output Path:").width(175).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                                                target: FileTarget::OutputPath,
                                                default_name: "zony.csv",
                                                filter_name: "CSV súbory",
                                                extensions: &["csv"],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Riadok 3: Toggles a Generate
                row![
                    text("Vygenerovať chýbajúcich operátorov:").size(16),
                    toggler(self.settings.generate_missing_operators).on_toggle(|value| Message::ToggleChanged(ToggleTarget::GenerateMissingOperators, value))

                    
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Použit filtrovanie:").size(16),
                    toggler(self.settings.use_lte_filter).on_toggle(|value| Message::ToggleChanged(ToggleTarget::UseLTEFilter, value))

                    
                ].spacing(30).align_y(Alignment::Center),

                row![
                    button(text("GENERATE").size(16).font(bold_font))
                        .on_press(Message::GenerateClicked)
                        .padding([12, 40])
                        .style(button::primary)
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::Protocol_LTE => {container(scrollable(
            column![
                text("Konfigurácia Protokol Modulu").size(32).font(bold_font),
                
                // Cesta k protokolu
                row![
                    text("Protokol File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.settings.protokol_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ProtokolPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::ProtokolPath,
                        filter_name: "XLSX Súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // GSM File Path
                row![
                    text("GSM File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.first_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FirstPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::FirstPath,
                        filter_name: "CSV Súbory",
                        extensions: &["csv"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // LTE File Path
                row![
                    text("LTE File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.second_path)
                        .on_input(|text| Message::PathChanged(FileTarget::SecondPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::SecondPath,
                        filter_name: "CSV Súbory",
                        extensions: &["csv"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Output Path
                row![
                    text("Output Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                        target: FileTarget::OutputPath,
                        default_name: "protokol-z-merania.xlsx",
                        filter_name: "Excel súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // --- OPRAVENÉ INPUTY ---
                
                row![
                    text("Meraná obec:").size(16),
                    text_input("Mesto/Obec...", &self.measured_city) // Zmena premennej
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::City, text)) // Zmena správy
                        .padding(10)
                        .width(175),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Hodnota Total Power:").size(16),
                    text_input("-20.0", &self.total_power)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::TotalPower, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Hodnota SINR:").size(16),
                    text_input("-20.0", &self.sinr)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::Sinr, text))
                        .padding(10)
                        .width(100),

                    text("Hodnota RSRP:").size(16),
                    text_input("-20.0", &self.rsrp)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::Rsrp, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Korekcia výšky antény:").size(16),
                    text_input("0.0", &self.antenna_height)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::AntennaHeight, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Korekcia vnútorného prostredia:").size(16),
                    text_input("0.0", &self.internal_environment)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::InternalEnv, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                // Generate Button
                row![
                    button(text("GENERATE").size(16).font(bold_font))
                        .on_press(Message::GenerateClicked)
                        .padding([12, 40])
                        .style(button::primary)
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::_5G_Zone => {container(scrollable(
            column![
                text("Konfigurácia 5G Zony Modulu").size(32).font(bold_font),

                row![
                    text("Zony File Path:").width(175).size(16),
                    text_input("Cesta k súboru...", &self.settings.zone_file_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ZonePath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                                                target: FileTarget::ZonePath,
                                                filter_name: "CSV Súbory",
                                                extensions: &["csv" ],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Filtre Operátorov Folder:").width(175).size(16),
                    text_input("Cesta k priečinku...", &self.settings.filter_5g_path)
                        .on_input(|text| Message::PathChanged(FileTarget::Filter5GPath, text))
                        .padding(10),
                    button("Select Folder").on_press(Message::SelectFolderClicked {
                                                target: FileTarget::Filter5GPath
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("5G File Path:").width(175).size(16),
                    text_input("Cesta k súboru...", &self.first_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FirstPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                                                target: FileTarget::FirstPath,
                                                filter_name: "CSV Súbory",
                                                extensions: &["csv" ],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Riadok 2: Output Path
                row![
                    text("Output Path:").width(175).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                                                target: FileTarget::OutputPath,
                                                default_name: "zony.csv",
                                                filter_name: "CSV súbory",
                                                extensions: &["csv"],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Vygenerovať chýbajúcich operátorov:").size(16),
                    toggler(self.settings.generate_missing_operators).on_toggle(|value| Message::ToggleChanged(ToggleTarget::GenerateMissingOperators, value))
                    //container(column![].width(Length::Fill)), 
                ].spacing(30).align_y(Alignment::Center),
                
                row![
                    text("Použit filtrovanie:").size(16),
                    toggler(self.settings.use_5g_filter).on_toggle(|value| Message::ToggleChanged(ToggleTarget::Use5GFilter, value))

                    
                ].spacing(30).align_y(Alignment::Center),

                row![
                    {let btn = button(text("GENERATE").size(16).font(bold_font))
                        .padding([12, 40])
                        .style(button::primary);
                    if self.is_generating {
                        btn // Vrátime tlačidlo bez on_press -> bude sivé a neklikateľné
                    } else {
                        btn.on_press(Message::GenerateClicked) // Pridáme akciu -> bude aktívne
                    }
                    }
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::_5G_Point => {container(scrollable(
            column![
                text("Konfigurácia 5G Body Modulu").size(32).font(bold_font),
                row![
                    text("Filtre Operátorov Folder:").width(175).size(16),
                    text_input("Cesta k priečinku...", &self.settings.filter_5g_path)
                        .on_input(|text| Message::PathChanged(FileTarget::Filter5GPath, text))
                        .padding(10),
                    button("Select Folder").on_press(Message::SelectFolderClicked {
                                                target: FileTarget::Filter5GPath
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // --- NOVÁ ČASŤ: Obdĺžnik pre viacero súborov ---
                container(
                    column![
                        // Nadpis sekcie
                        text("5G File Paths:").size(16).font(bold_font),
                        
                        // Zoznam súborov (Scrollable area)
                        scrollable(
                            column(
                                self.multiple_paths.iter().enumerate().map(|(i, path)| {
                                    row![
                                        // Ikona alebo text súboru
                                        text(path).size(14).width(Length::Fill),
                                        
                                        // Tlačidlo na vymazanie (červené X)
                                        button(text("X").size(14))
                                            .on_press(Message::RemovePath(i))
                                            .padding([5, 10])
                                            .style(button::danger) // Iced štandardne má 'danger' alebo si nadefinuj štýl
                                    ]
                                    .spacing(10)
                                    .align_y(Alignment::Center)
                                    .padding(Padding {top: 5.0, right: 15.0, bottom: 5.0, left: 5.0})
                                    .into()
                                })
                            )
                            .spacing(5)
                        )
                        .height(Length::Fixed(150.0)) // Fixná výška obdĺžnika (napr. 150px)
                        .width(Length::Fill),

                        // Tlačidlo na pridanie ďalšieho súboru
                        button(
                            row![
                                text("+ Pridať súbor").size(14)
                            ].spacing(5).align_y(Alignment::Center)
                        )
                        .on_press(Message::SelectFileClicked {
                            target: FileTarget::MultiplePaths, // Použijeme nový target
                            filter_name: "CSV Súbory",
                            extensions: &["csv"],
                        })
                        .padding(10)
                        .width(Length::Fill), // Tlačidlo na celú šírku
                    ]
                    .spacing(10)
                )
                .padding(15)
                // Pridáme vizuálny rámček (štýl kontajnera)
                .style(container::bordered_box) // Ak máš definovaný štýl, alebo použi default s borderom
                .width(Length::Fill),
                // --- KONIEC NOVEJ ČASTI ---

                // Riadok 2: Output Path
                row![
                    text("Output Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                                                target: FileTarget::OutputPath,
                                                default_name: "zony.csv",
                                                filter_name: "CSV súbory",
                                                extensions: &["csv"],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),
                
                row![
                    text("Protokol File Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.settings.protocol_points_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ProtocolPointsPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::ProtocolPointsPath,
                        filter_name: "XLSX Súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Output Protokol Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.second_output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::SecondOutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                        target: FileTarget::SecondOutputPath,
                        default_name: "protokol-z-merania.xlsx",
                        filter_name: "Excel súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Vygenerovať chýbajúcich operátorov:").size(16),
                    toggler(self.settings.generate_missing_operators).on_toggle(|value| Message::ToggleChanged(ToggleTarget::GenerateMissingOperators, value))
                    //container(column![].width(Length::Fill)), 
                ].spacing(30).align_y(Alignment::Center),
                
                row![
                    text("Použit filtrovanie:").size(16),
                    toggler(self.settings.use_5g_filter).on_toggle(|value| Message::ToggleChanged(ToggleTarget::Use5GFilter, value))
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Vygenerovat protokol:").size(16),
                    toggler(self.settings.use_protocol_points).on_toggle(|value| Message::ToggleChanged(ToggleTarget::UseProtocolPoints, value))
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Veľkosť bodu (polomer v metoch):").size(16),
                    text_input("1.5", &self.max_distance_of_point)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::MaxDistance, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Nastav minimalny SSS RSRP:").size(16),
                    text_input("-20.0", &self.threshold_rsrp)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::ThresholdRsrp, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Nastav minimalny SSS SINR:").size(16),
                    text_input("-20.0", &self.threshold_sinr)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::ThresholdSinr, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    {let btn = button(text("GENERATE").size(16).font(bold_font))
                        .padding([12, 40])
                        .style(button::primary);
                    if self.is_generating {
                        btn // Vrátime tlačidlo bez on_press -> bude sivé a neklikateľné
                    } else {
                        btn.on_press(Message::GenerateClicked) // Pridáme akciu -> bude aktívne
                    }
                    }
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::LTE_Point => {container(scrollable(
            column![
                text("Konfigurácia LTE Body Modulu").size(32).font(bold_font),
                row![
                    text("Filtre Operátorov Folder:").width(175).size(16),
                    text_input("Cesta k priečinku...", &self.settings.filter_lte_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FilterLTEPath, text))
                        .padding(10),
                    button("Select Folder").on_press(Message::SelectFolderClicked {
                                                target: FileTarget::FilterLTEPath
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // --- NOVÁ ČASŤ: Obdĺžnik pre viacero súborov ---
                container(
                    column![
                        // Nadpis sekcie
                        text("LTE File Paths:").size(16).font(bold_font),
                        
                        // Zoznam súborov (Scrollable area)
                        scrollable(
                            column(
                                self.multiple_paths.iter().enumerate().map(|(i, path)| {
                                    row![
                                        // Ikona alebo text súboru
                                        text(path).size(14).width(Length::Fill),
                                        
                                        // Tlačidlo na vymazanie (červené X)
                                        button(text("X").size(14))
                                            .on_press(Message::RemovePath(i))
                                            .padding([5, 10])
                                            .style(button::danger) // Iced štandardne má 'danger' alebo si nadefinuj štýl
                                    ]
                                    .spacing(10)
                                    .align_y(Alignment::Center)
                                    .padding(Padding {top: 5.0, right: 15.0, bottom: 5.0, left: 5.0})
                                    .into()
                                })
                            )
                            .spacing(5)
                        )
                        .height(Length::Fixed(150.0)) // Fixná výška obdĺžnika (napr. 150px)
                        .width(Length::Fill),

                        // Tlačidlo na pridanie ďalšieho súboru
                        button(
                            row![
                                text("+ Pridať súbor").size(14)
                            ].spacing(5).align_y(Alignment::Center)
                        )
                        .on_press(Message::SelectFileClicked {
                            target: FileTarget::MultiplePaths, // Použijeme nový target
                            filter_name: "CSV Súbory",
                            extensions: &["csv"],
                        })
                        .padding(10)
                        .width(Length::Fill), // Tlačidlo na celú šírku
                    ]
                    .spacing(10)
                )
                .padding(15)
                // Pridáme vizuálny rámček (štýl kontajnera)
                .style(container::bordered_box) // Ak máš definovaný štýl, alebo použi default s borderom
                .width(Length::Fill),
                // --- KONIEC NOVEJ ČASTI ---

                // Riadok 2: Output Path
                row![
                    text("Output Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                                                target: FileTarget::OutputPath,
                                                default_name: "zony.csv",
                                                filter_name: "CSV súbory",
                                                extensions: &["csv"],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),
                
                row![
                    text("Protokol File Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.settings.protocol_points_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ProtocolPointsPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::ProtocolPointsPath,
                        filter_name: "XLSX Súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Output Protokol Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.second_output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::SecondOutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                        target: FileTarget::SecondOutputPath,
                        default_name: "protokol-z-merania.xlsx",
                        filter_name: "Excel súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Vygenerovať chýbajúcich operátorov:").size(16),
                    toggler(self.settings.generate_missing_operators).on_toggle(|value| Message::ToggleChanged(ToggleTarget::GenerateMissingOperators, value))
                    //container(column![].width(Length::Fill)), 
                ].spacing(30).align_y(Alignment::Center),
                
                row![
                    text("Použit filtrovanie:").size(16),
                    toggler(self.settings.use_lte_filter).on_toggle(|value| Message::ToggleChanged(ToggleTarget::UseLTEFilter, value))

                    
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Vygenerovat protokol:").size(16),
                    toggler(self.settings.use_protocol_points).on_toggle(|value| Message::ToggleChanged(ToggleTarget::UseProtocolPoints, value))
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Veľkosť bodu (polomer v metoch):").size(16),
                    text_input("1.5", &self.max_distance_of_point)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::MaxDistance, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Nastav minimalny RSRP:").size(16),
                    text_input("-20.0", &self.threshold_rsrp)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::ThresholdRsrp, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Nastav minimalny SINR:").size(16),
                    text_input("-20.0", &self.threshold_sinr)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::ThresholdSinr, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    {let btn = button(text("GENERATE").size(16).font(bold_font))
                        .padding([12, 40])
                        .style(button::primary);
                    if self.is_generating {
                        btn // Vrátime tlačidlo bez on_press -> bude sivé a neklikateľné
                    } else {
                        btn.on_press(Message::GenerateClicked) // Pridáme akciu -> bude aktívne
                    }
                    }
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::Mobile_Point => {container(scrollable(
            column![
                text("Konfigurácia 5G Mobil Body Modulu").size(32).font(bold_font),

                row![
                    text("LTE Operátori Folder:").width(175).size(16),
                    text_input("Cesta k priečinku...", &self.settings.filter_lte_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FilterLTEPath, text))
                        .padding(10),
                    button("Select Folder").on_press(Message::SelectFolderClicked {
                                                target: FileTarget::FilterLTEPath
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("5G Operátori Folder:").width(175).size(16),
                    text_input("Cesta k priečinku...", &self.settings.filter_5g_path)
                        .on_input(|text| Message::PathChanged(FileTarget::Filter5GPath, text))
                        .padding(10),
                    button("Select Folder").on_press(Message::SelectFolderClicked {
                                                target: FileTarget::Filter5GPath
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                container(
                    column![
                        text("Mobile File Paths:").size(16).font(bold_font),

                        scrollable(
                            container(
                                column(
                                    self.mobile_paths.iter().enumerate().map(|(i, entry)| {
                                        container(
                                            column![
                                            row![
                                                text(format!("Bod {}", i + 1)).size(14).font(bold_font),
                                                container(row![]).width(Length::Fill),
                                                button(text("X").size(14))
                                                    .on_press(Message::RemoveMobilePathEntry(i))
                                                    .padding([5, 10])
                                                    .style(button::danger)
                                            ]
                                            .spacing(10)
                                            .align_y(Alignment::Center),

                                            row![
                                                text("LTE cesta:").width(100).size(14),
                                                text_input("Cesta k LTE súboru...", &entry.lte_path)
                                                    .on_input(move |text| Message::PathChanged(FileTarget::MobileLtePath(i), text))
                                                    .padding(10),
                                                button("Pridať cestu")
                                                    .on_press(Message::SelectFileClicked {
                                                        target: FileTarget::MobileLtePath(i),
                                                        filter_name: "CSV Súbory",
                                                        extensions: &["csv"],
                                                    })
                                                    .padding(10),
                                            ].spacing(15).align_y(Alignment::Center),

                                            row![
                                                text("5G cesta:").width(100).size(14),
                                                text_input("Cesta k 5G súboru...", &entry.g5_path)
                                                    .on_input(move |text| Message::PathChanged(FileTarget::Mobile5GPath(i), text))
                                                    .padding(10),
                                                button("Pridať cestu")
                                                    .on_press(Message::SelectFileClicked {
                                                        target: FileTarget::Mobile5GPath(i),
                                                        filter_name: "CSV Súbory",
                                                        extensions: &["csv"],
                                                    })
                                                    .padding(10),
                                            ].spacing(15).align_y(Alignment::Center),
                                            ]
                                            .spacing(10)
                                        )
                                        .padding(Padding {top: 5.0, right: 15.0, bottom: 5.0, left: 10.0})
                                        .padding(10)
                                        .style(container::bordered_box)
                                        .into()
                                    })
                                )
                                .spacing(10)
                            )
                            .padding(Padding { top: 0.0, right: 24.0, bottom: 0.0, left: 0.0 })
                        )
                        .height(Length::Fixed(220.0))
                        .width(Length::Fill),

                        button(
                            row![
                                text("+ Pridať bod").size(14)
                            ].spacing(5).align_y(Alignment::Center)
                        )
                        .on_press(Message::AddMobilePathEntry)
                        .padding(10)
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                )
                .padding(15)
                .style(container::bordered_box)
                .width(Length::Fill),

                row![
                    text("Output Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                                                target: FileTarget::OutputPath,
                                                default_name: "zony.csv",
                                                filter_name: "CSV súbory",
                                                extensions: &["csv"],
                                            }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Protokol File Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.settings.protocol_mobile_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ProtocolMobilePath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::ProtocolMobilePath,
                        filter_name: "XLSX Súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Output Protokol Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.second_output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::SecondOutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                        target: FileTarget::SecondOutputPath,
                        default_name: "protokol-z-merania.xlsx",
                        filter_name: "Excel súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Vygenerovať chýbajúcich operátorov:").size(16),
                    toggler(self.settings.generate_missing_operators).on_toggle(|value| Message::ToggleChanged(ToggleTarget::GenerateMissingOperators, value))
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Pouzit filtrovanie:").size(16),
                    toggler(self.settings.use_multiple_filters).on_toggle(|value| Message::ToggleChanged(ToggleTarget::UseMultipleFilters, value))
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Vygenerovat protokol:").size(16),
                    toggler(self.settings.use_protocol_points).on_toggle(|value| Message::ToggleChanged(ToggleTarget::UseProtocolPoints, value))
                ].spacing(30).align_y(Alignment::Center),

                row![
                    text("Veľkosť bodu (polomer v metoch):").size(16),
                    text_input("1.5", &self.max_distance_of_point)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::MaxDistance, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Nastav minimalny SSS RSRP:").size(16),
                    text_input("-20.0", &self.threshold_rsrp)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::ThresholdRsrp, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Nastav minimalny SSS SINR:").size(16),
                    text_input("-20.0", &self.threshold_sinr)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::ThresholdSinr, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    {let btn = button(text("GENERATE").size(16).font(bold_font))
                        .padding([12, 40])
                        .style(button::primary);
                    if self.is_generating {
                        btn
                    } else {
                        btn.on_press(Message::GenerateClicked)
                    }
                    }
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            Screen::Protocol_5G => {container(scrollable(
            column![
                text("Konfigurácia Protokol 5G Modulu").size(32).font(bold_font),
                
                // Cesta k protokolu
                row![
                    text("Protokol File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.settings.protokol_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ProtokolPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::ProtokolPath,
                        filter_name: "XLSX Súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // GSM File Path
                row![
                    text("GSM File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.first_path)
                        .on_input(|text| Message::PathChanged(FileTarget::FirstPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::FirstPath,
                        filter_name: "CSV Súbory",
                        extensions: &["csv"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // LTE File Path
                row![
                    text("LTE File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.second_path)
                        .on_input(|text| Message::PathChanged(FileTarget::SecondPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::SecondPath,
                        filter_name: "CSV Súbory",
                        extensions: &["csv"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // 5G File Path
                row![
                    text("5G File Path:").width(150).size(16),
                    text_input("Cesta k súboru...", &self.third_path)
                        .on_input(|text| Message::PathChanged(FileTarget::ThirdPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SelectFileClicked {
                        target: FileTarget::ThirdPath,
                        filter_name: "CSV Súbory",
                        extensions: &["csv"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // Output Path
                row![
                    text("Output Path:").width(150).size(16),
                    text_input("Cesta pre uloženie...", &self.output_path)
                        .on_input(|text| Message::PathChanged(FileTarget::OutputPath, text))
                        .padding(10),
                    button("Select File").on_press(Message::SaveFileClicked {
                        target: FileTarget::OutputPath,
                        default_name: "protokol-z-merania.xlsx",
                        filter_name: "Excel súbory",
                        extensions: &["xlsx"],
                    }).padding(10),
                ].spacing(20).align_y(Alignment::Center),

                // --- OPRAVENÉ INPUTY ---
                
                row![
                    text("Meraná obec:").size(16),
                    text_input("Mesto/Obec...", &self.measured_city) // Zmena premennej
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::City, text)) // Zmena správy
                        .padding(10)
                        .width(175),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Hodnota Total Power:").size(16),
                    text_input("-20.0", &self.total_power)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::TotalPower, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Hodnota SSS SINR:").size(16),
                    text_input("-20.0", &self.sinr)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::Sinr, text))
                        .padding(10)
                        .width(100),

                    text("Hodnota SSS RSRP:").size(16),
                    text_input("-20.0", &self.rsrp)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::Rsrp, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Korekcia výšky antény:").size(16),
                    text_input("0.0", &self.antenna_height)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::AntennaHeight, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                row![
                    text("Korekcia vnútorného prostredia:").size(16),
                    text_input("0.0", &self.internal_environment)
                        .on_input(|text| Message::ProtocolInputChanged(ProtocolInputType::InternalEnv, text))
                        .padding(10)
                        .width(100),
                ].spacing(20).align_y(Alignment::Center),

                // Generate Button
                row![
                    button(text("GENERATE").size(16).font(bold_font))
                        .on_press(Message::GenerateClicked)
                        .padding([12, 40])
                        .style(button::primary)
                ].spacing(30).align_y(Alignment::Center)
            ]
            .spacing(20).padding(40) // <--- 1. PADDING DAJ SEM (obsah bude odsadený)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)},
            _ => {container(
                column![
                    text("Funkcia ešte nie je implementovaná.").size(24).font(bold_font),
                ]
                .spacing(20)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40)},
    };

        // --- TERMINÁL ---
        let terminal_section = if self.settings.is_terminal_open {
            let log_content = column(
                self.logs.iter()
                    .map(|(log, log_type)| {
                        let color = match log_type {
                            TerminalMessageType::Info => Color::from_rgb(0.6, 0.6, 1.0),
                            TerminalMessageType::Warning => Color::from_rgb(1.0, 0.9, 0.0),
                            TerminalMessageType::Error => Color::from_rgb(1.0, 0.2, 0.2),
                            TerminalMessageType::Success => Color::from_rgb(0.2, 1.0, 0.2),
                        };
                        text(format!("$ {}", log))
                            .color(color)
                            .font(Font::MONOSPACE)
                            .size(14)
                            .into()
                    })
                    .collect::<Vec<_>>()
            )
            .spacing(4)
            .padding(15); // Padding sme presunuli sem (aplikuje sa na text, nie na scrollbar)

            container(
                scrollable(log_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(220)
            // Tu sme odstránili .padding(15)
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(20, 20, 20))),
                border: iced::border::Border {
                    color: Color::from_rgb(0.1, 0.1, 0.1),
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
        } else {
            container(column![]).height(0)
        };

        // --- ZLOŽENIE ---
        row![
            sidebar,
            column![
                main_content,
                terminal_section
            ]
            .width(Length::Fill)
        ]
        .into()
    }

    fn theme(&self) -> Theme {
        if self.settings.is_dark_mode { Theme::KanagawaDragon } else { Theme::TokyoNightLight }
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.is_timer_running {
            // Každých 1000 milisekúnd (2 sekunda) pošle správu Message::Tick
            time::every(Duration::from_millis(2000))
                .map(Message::SaveTick)
        } else {
            Subscription::none()
        }
    }
}