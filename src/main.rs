use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    io::{self, stdin, stdout, BufReader, Read},
    rc::Rc,
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use models::{BoneWand, Spell};
use ratatui::{
    backend::CrosstermBackend,
    layout::Constraint,
    style::{Style, Stylize},
    widgets::{Block, Borders, List, ListState, Paragraph, Row, Table},
    Frame, Terminal,
};

mod models;

#[cfg(windows)]
const BONES_DIRECTORY: &'static str = "/../LocalLow/Nolla_Games_Noita/save00/persistent/bones_new";

const TRANSLATION_FILE: &'static str = "common.csv";

#[derive(PartialEq, Eq)]
enum UiState {
    BoneList,
    Bone(usize),
}

impl Default for UiState {
    fn default() -> Self {
        Self::BoneList
    }
}

#[derive(Default)]
struct State {
    temporary_list_state: Option<ListState>,
    list_state: ListState,
    list: Vec<String>,
    wands: Vec<BoneWand>,
    ui_state: UiState,
}

fn load_wands() -> Vec<BoneWand> {
    let translation = load_translation(1);

    let wands: Vec<BoneWand> =
        std::fs::read_dir(std::env::var("APPDATA").unwrap() + BONES_DIRECTORY)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|directory| {
                if let Some(extension) = directory.path().extension() {
                    extension == "xml"
                } else {
                    false
                }
            })
            .map(|directory| directory.path())
            .map(|path| BoneWand::load_from(&path, &translation))
            .collect();

    wands
}

fn main() {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    let wands = load_wands();
    let mut file_names: Vec<String> = wands
        .iter()
        .map(|bone_wand| {
            format!(
                "{} (Tier {}){}",
                bone_wand.filename,
                bone_wand.wand.tier,
                if bone_wand.wand.spells.iter().any(|f| f.always_cast) {
                    " (A)"
                } else {
                    ""
                }
            )
        })
        .collect();

    let state = Rc::new(RefCell::new(State::default()));
    state.borrow_mut().list_state.select(Some(0));
    state.borrow_mut().list.append(&mut file_names);
    state.borrow_mut().wands = wands;

    let state2 = state.clone();
    let ui_bound = move |frame: &mut Frame| {
        let state = state2;
        ui(frame, &mut state.borrow_mut());
    };

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(ui_bound.clone()).unwrap();

        let state = state.clone();
        should_quit = handle_events(&mut state.borrow_mut()).unwrap();
    }

    disable_raw_mode().unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
}

fn ui(frame: &mut Frame, state: &mut State) {
    let list_data = state.list.clone();

    match state.ui_state {
        UiState::BoneList => {
            let list = List::new(list_data)
                .highlight_symbol(">> ")
                .highlight_style(Style::default().light_green())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Noita Graveyard ({} bones)", state.wands.len())),
                );

            frame.render_stateful_widget(list, frame.size(), &mut state.list_state);
        }
        UiState::Bone(id) => {
            let bone_wand = state.wands.get(id).unwrap();

            let rows = vec![
                Row::new(vec!["", ""]),
                Row::new(vec!["Shuffle", {
                    if bone_wand.wand.shuffle {
                        "Yes"
                    } else {
                        "No"
                    }
                }]),
                Row::new(vec![
                    "Spells/Cast".to_owned(),
                    format!("{}", bone_wand.wand.spells_per_cast),
                ]),
                Row::new(vec![
                    "Cast delay".to_owned(),
                    format!("{:.2} s", bone_wand.wand.cast_delay),
                ]),
                Row::new(vec![
                    "Rechrg. Time".to_owned(),
                    format!("{:.2} s", bone_wand.wand.recharge_time),
                ]),
                Row::new(vec![
                    "Mana max".to_owned(),
                    format!("{}", bone_wand.wand.mana_max),
                ]),
                Row::new(vec![
                    "Mana chg. Spd".to_owned(),
                    format!("{}", bone_wand.wand.mana_charge_speed),
                ]),
                Row::new(vec![
                    "Capacity".to_owned(),
                    format!("{}", bone_wand.wand.capacity),
                ]),
                Row::new(vec![
                    "Spread".to_owned(),
                    format!("{:.2} DEG", bone_wand.wand.spread),
                ]),
                Row::new(vec![
                    "Speed".to_owned(),
                    format!("x{:.2}", bone_wand.wand.speed),
                ]),
            ];

            let rows_len = rows.len();
            let always_cast_text = Paragraph::new("Always cast");
            let always_cast_spells: Vec<String> = bone_wand
                .wand
                .spells
                .iter()
                .filter(|spell| spell.always_cast)
                .enumerate()
                .map(|(idx, spell)| format!("{}. {}", idx + 1, spell.name))
                .collect();

            let always_cast_list = List::new(always_cast_spells);

            let spells_text = Paragraph::new("Spells");
            let spells: Vec<String> = bone_wand
                .wand
                .spells
                .iter()
                .filter(|spell| !spell.always_cast)
                .enumerate()
                .map(|(idx, spell)| format!("{}. {}", idx + 1, spell.name))
                .collect();

            let spells_list = List::new(spells).highlight_style(Style::default().light_green());

            let wand_name = Paragraph::new(bone_wand.wand.name.to_owned());
            let table = Table::new(rows, [Constraint::Length(15), Constraint::Length(10)]).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{}", bone_wand.filename)),
            );

            let mut wand_name_frame = frame.size();
            wand_name_frame.x += 1;
            wand_name_frame.y += 1;
            wand_name_frame.width -= 1;
            wand_name_frame.height -= 1;

            let mut spells_frame = frame.size();
            spells_frame.x += 1;
            spells_frame.y += 1 + rows_len as u16;
            spells_frame.width -= 1;
            spells_frame.height -= 1 + rows_len as u16;

            frame.render_widget(wand_name, wand_name_frame);
            if always_cast_list.len() > 0 {
                let len = always_cast_list.len() as u16;
                frame.render_widget(always_cast_text, spells_frame);
                spells_frame.y += 1;
                spells_frame.height -= 1;
                frame.render_widget(always_cast_list, spells_frame);
                spells_frame.y += len;
                spells_frame.height -= len;
            }

            frame.render_widget(spells_text, spells_frame);
            spells_frame.y += 1;
            spells_frame.height -= 2;
            if let Some(state) = &mut state.temporary_list_state {
                frame.render_stateful_widget(spells_list, spells_frame, state);
            }

            frame.render_widget(table, frame.size());
        }
    }
}

fn handle_events(state: &mut State) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        let event = event::read().unwrap();

        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                return match key.code {
                    KeyCode::Enter => {
                        let selected = state.list_state.selected().unwrap_or(0);

                        state.ui_state = UiState::Bone(selected);

                        let mut new_state = ListState::default();
                        new_state.select(Some(0));

                        state.temporary_list_state = Some(new_state);

                        Ok(false)
                    }
                    KeyCode::Backspace => {
                        if state.ui_state != UiState::BoneList {
                            state.ui_state = UiState::BoneList;
                            state.temporary_list_state = None;
                        }

                        Ok(false)
                    }
                    KeyCode::Down => {
                        if state.ui_state == UiState::BoneList {
                            let selected = state.list_state.selected().unwrap_or(0);
                            if selected + 1 < state.list.len() {
                                state.list_state.select(Some(selected + 1));
                            }
                        } else {
                            let selected_global = state.list_state.selected().unwrap_or(0);
                            let len = state
                                .wands
                                .get(selected_global)
                                .unwrap()
                                .wand
                                .spells
                                .iter()
                                .filter(|spell| !spell.always_cast)
                                .count();

                            if let Some(state) = &mut state.temporary_list_state {
                                let selected = state.selected().unwrap_or(0);
                                if selected + 1 < len {
                                    state.select(Some(selected + 1));
                                }
                            }
                        }

                        Ok(false)
                    }
                    KeyCode::Up => {
                        if state.ui_state == UiState::BoneList {
                            let selected = state.list_state.selected().unwrap_or(0);
                            if selected as i32 - 1 >= 0 {
                                state.list_state.select(Some(selected - 1));
                            }
                        } else {
                            if let Some(state) = &mut state.temporary_list_state {
                                let selected = state.selected().unwrap_or(0);
                                if selected as i32 - 1 >= 0 {
                                    state.select(Some(selected - 1));
                                }
                            }
                        }

                        Ok(false)
                    }
                    KeyCode::Char('q') | KeyCode::Esc => match state.ui_state {
                        UiState::BoneList => Ok(true),
                        UiState::Bone(_) => {
                            state.temporary_list_state = None;
                            state.ui_state = UiState::BoneList;
                            Ok(false)
                        }
                    },
                    _ => Ok(false),
                };
            }
        }
    }

    Ok(false)
}

fn load_translation(language_index: usize) -> HashMap<String, String> {
    let mut translation_map: HashMap<String, String> = HashMap::new();

    if std::fs::metadata(TRANSLATION_FILE).is_err() {
        println!("common.csv missing");
        println!("Press any key to continue...");
        stdin().read(&mut [0u8]).unwrap();
        std::process::exit(-1);
    }

    let mut reader =
        csv::Reader::from_reader(BufReader::new(File::open(TRANSLATION_FILE).unwrap()));
    for result in reader.records() {
        if let Ok(record) = result {
            let id = record.get(0).unwrap().to_owned();
            let translation = record.get(language_index).unwrap().to_owned();

            translation_map.insert(id, translation);
        }
    }

    translation_map
}
