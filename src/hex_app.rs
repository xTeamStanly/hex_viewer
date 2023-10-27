use std::{fs::{File, Metadata}, collections::HashMap, path::PathBuf, os::windows::prelude::FileExt};

use egui::{CentralPanel, LayerId, Order, Id, Color32, Align2, TextStyle, Modifiers, Layout, Align, RichText, Label, Sense, Key};
use egui_extras::{TableBuilder, Column};
use egui_notify::Toasts;

#[derive(Default)]
pub struct HexApp {
    pub debug: String,
    pub toasts: Toasts,

    pub file: Option<File>,
    pub file_chunks: HashMap<usize, Vec<u8>>,
    pub metadata: Option<Metadata>,

    pub row_count: usize,
    pub column_count: usize,
    pub trailing_bytes_count: usize,

    pub selected_byte: Option<(usize, usize)>, // (row, index)


    pub hovering: bool,
    pub hovered_filepath: Option<PathBuf>,
    pub dropped_filepath: Option<PathBuf>
}

impl HexApp {

    fn clear_file(&mut self) {
        self.file = None;
        self.file_chunks.clear();
        self.metadata = None;
        self.dropped_filepath = None;
        self.hovered_filepath = None;
        self.selected_byte = None;
    }

    fn load_file(&mut self, file: File, metadata: Metadata) {

        let file_size = metadata.len() as usize;

        let mut row_count = file_size / self.column_count;
        if row_count * self.column_count != file_size {
            self.trailing_bytes_count = file_size % self.column_count;
            row_count += 1;
        } else {
            self.trailing_bytes_count = 0;
        }

        self.metadata = Some(metadata);
        self.row_count = row_count;
        self.file = Some(file);
        self.file_chunks = HashMap::default();
        self.selected_byte = None;
    }

    fn get_hovered_filepath(&mut self) -> (&str, bool) { // (result, error)
        if let Some(hovered_filepath) = &self.hovered_filepath {
            match hovered_filepath.as_os_str().to_str() {
                None => {
                    self.toasts.error("Hovered path contains non UTF-8 characters");
                    ("! Error !", true)
                },
                Some(filepath) => {
                    (filepath, false)
                }
            }

        } else {
            self.toasts.error("Internal error. No hovered filepath.");
            ("! Error !", true)
        }
    }

    fn update_input(&mut self, ctx: &egui::Context) {
        // keyboard
        ctx.input_mut(|input| {
            if let Some(selected) = &mut self.selected_byte {
                if let Some(metadata) = &self.metadata {
                    let max_offset = metadata.len() as usize;

                    if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
                        // Arrow Right

                        if max_offset - 1 != selected.1 {
                            selected.1 += 1;

                            if selected.1 % self.column_count == 0 {
                                selected.0 += 1;
                            }
                        } else {
                            self.toasts.warning("End of file");
                        }

                    } else if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                        // Arrow Left

                        if selected.1 != 0 {
                            selected.1 -= 1;

                            if (selected.1 % self.column_count == 0) && (selected.1 != 0) {
                                selected.0 -= 1;
                            }
                        } else {
                            self.toasts.warning("Start of file");
                        }
                    } else if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        // Arrow Up

                        if selected.0 != 0 {
                            selected.0 -= 1;
                            selected.1 -= self.column_count;
                        } else {
                            self.toasts.warning("Start of file");
                        }
                    } else if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        // Arrow Down

                        if selected.0 != self.row_count - 1{
                            selected.0 += 1;
                            selected.1 += self.column_count;
                            selected.1 = selected.1.clamp(0, max_offset - 1);
                        } else {
                            self.toasts.warning("End of file");
                        }
                    }

                } else {
                    self.toasts.error("Internal selection error");
                }
            }
        });


        // drag & drop
        ctx.input(|input| {

            if let Some(hovered_file) = input.raw.hovered_files.first() {
                self.hovering = true;
                if let Some(filepath) = &hovered_file.path {
                    self.hovered_filepath = Some(filepath.to_owned());
                }
            }

            if input.raw.dropped_files.len() > 1 {
                self.toasts.warning("Dropped multiple files - taking first");
            }

            if let Some(dropped_file) = input.raw.dropped_files.first() {
                self.hovering = false;

                if let Some(filepath) = &dropped_file.path {
                    if filepath.is_dir() {
                        self.toasts.warning("Dropped directory instead of file");
                        return;
                    }
                    match filepath.file_name() {
                        Some(name) => {
                            if let Some(valid_name) = name.to_str() {
                                // self.toasts.info(format!("Dropped file: {}", valid_name));

                                let (file, metadata) = match File::open(filepath) {
                                    Ok(f) => {
                                        match f.metadata() {
                                            Ok(metadata) => {
                                                self.toasts.info(format!("Opened file: {}", valid_name));
                                                (f, metadata)
                                            },
                                            Err(err) => {
                                                self.toasts.error(err.to_string());
                                                return;
                                            }
                                        }
                                    },
                                    Err(err) => {
                                        self.toasts.error(err.to_string());
                                        return;
                                    }
                                };

                                self.load_file(file, metadata);

                            } else {
                                self.toasts.error("Non UTF-8 characters in file name");
                            }
                        },
                        None => {
                            self.toasts.error("Error converting item name");
                        }
                    }
                    self.dropped_filepath = Some(filepath.to_owned());
                } else {
                    self.toasts.error("Empty item path!");
                }
            }
        });
    }
}

impl eframe::App for HexApp {

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let window_title: String = match frame.info().cpu_usage {
            Some(time) => format!("HexView - frametime: {:.8}ms", time * 1000.0),
            None => "HexView - Error getting frametime".to_owned()
        };
        frame.set_window_title(&window_title);

        self.update_input(ctx);

        if self.hovering {
            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(128));
            let hovered_filepath = self.get_hovered_filepath();
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                format!("Release to drop file:\n{}", hovered_filepath.0),
                TextStyle::Heading.resolve(&ctx.style()),
                if hovered_filepath.1 { Color32::RED } else { Color32::WHITE }
            );
        }
        self.hovering = false;
        self.hovered_filepath = None;

        if self.file.is_some() {

            CentralPanel::default().show(ctx, |ui| {
                TableBuilder::new(ui)

                    .cell_layout(Layout::left_to_right(Align::LEFT))
                    .columns(Column::auto(), self.column_count + 2)
                    .striped(true)
                    .drag_to_scroll(true)
                    .min_scrolled_height(0.0)

                    .header(20.0, |mut header| {

                        // 'Offset'
                        header.col(|ui| { ui.strong("Offset"); });

                        // Column hex
                        for i in 0..self.column_count {
                            header.col(|ui| { ui.strong(format!("{:0>2X}", i)); });
                        }

                        // 'Content'
                        header.col(|ui| { ui.strong("Content"); });
                    })

                    .body(|body| {
                        body.rows(10.0, self.row_count, |row_index, mut row| {

                            // Row indices (Hex)
                            row.col(|ui| {
                                let mut rtflabel = RichText::new(format!("{:0>8X}", row_index * self.column_count))
                                    .monospace()
                                    .strong()
                                    .color(Color32::GOLD);

                                if let Some(i) = self.selected_byte {
                                    if i.0 == row_index {
                                        rtflabel = rtflabel.color(Color32::RED);
                                    }
                                }

                                ui.label(rtflabel);
                            });

                            // file data
                            if self.file_chunks.len() > 4048 { self.file_chunks.clear(); }

                            // deal with trailing row
                            let mut trailing = false;
                            let column_count = if (self.trailing_bytes_count != 0) && (self.row_count - 1 == row_index) {
                                trailing = true;
                                self.trailing_bytes_count
                                } else {
                                self.column_count
                            };


                            if let None = self.file_chunks.get(&row_index) {
                                // cache miss

                                if let Some(file) = &self.file {
                                    let mut buffer = vec![0; column_count];

                                    if let Err(err) = file.seek_read(&mut buffer, (self.column_count * row_index) as u64) {
                                        self.toasts.error(format!("File read error: {}", err));
                                        self.clear_file();
                                        return;
                                    }

                                    self.file_chunks.insert(row_index, buffer);
                                } else {
                                    self.toasts.error("Internal file error");
                                    self.clear_file();
                                    return;
                                }
                            }

                            let row_data = match self.file_chunks.get(&row_index) {
                                Some(rd) => rd,
                                None => {
                                    self.toasts.error("Internal file error");
                                    self.clear_file();
                                    return;
                                }
                            };

                            let mut printable_content = String::new();
                            let mut printable_content_rtf: Vec<RichText> = vec![];

                            let mut hex_element_color: Color32 = Color32::WHITE;
                            if let Some(selected) = self.selected_byte {
                                if selected.0 == row_index {
                                    hex_element_color = Color32::LIGHT_YELLOW;
                                }
                            }

                            for i in 0..column_count {
                                let character = row_data[i];
                                let mut out_char = character as char;

                                if !((out_char.is_alphanumeric() || out_char.is_ascii_graphic() || character.is_ascii_punctuation()) && !out_char.is_whitespace() && !out_char.is_control()) {
                                    out_char = '.';
                                }

                                let index = row_index * self.column_count + i;

                                printable_content.push(out_char);

                                let mut element_color = hex_element_color;
                                let mut output_color = Color32::GRAY;

                                if let Some(ind) = self.selected_byte {
                                    if row_index == ind.0 {
                                        output_color = Color32::LIGHT_YELLOW;
                                    }

                                    if index == ind.1 {
                                        element_color = Color32::RED;
                                        output_color = Color32::RED;
                                    }
                                }

                                let rtf = RichText::new(format!("{:0>2X}", character))
                                    .monospace()
                                    .color(element_color);
                                let output_content = RichText::new(out_char).monospace().color(output_color);
                                printable_content_rtf.push(output_content);

                                row.col(|ui| {
                                    if ui.add(Label::new(rtf).sense(Sense::click())).clicked() {
                                        self.selected_byte = Some((row_index, index));
                                    }
                                });
                            }

                            if trailing {
                                for _ in column_count..self.column_count {
                                    row.col(|ui| { ui.label("  "); });
                                }
                            }

                            // 'Content' row
                            row.col(|ui| {
                                let width = ui.fonts(|f| f.glyph_width(&TextStyle::Body.resolve(ui.style()), ' '));
                                ui.spacing_mut().item_spacing.x = width;
                                for c in printable_content_rtf { ui.label(c); }
                            });
                        });
                });
            });
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.heading("Drag & drop a file to open it");
                });
            });
        }

        self.toasts.show(ctx);
    }
}
