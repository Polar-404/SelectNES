use crate::{engine::{config::EmulatorConfig, terminal::print_terminal::{LogType, print_logs}}, ppu::palettes::PaletteTheme};

pub fn render_settings(settings: &mut EmulatorConfig, ui: &mut egui_dock::egui::Ui) {
    change_palette(settings, ui);

    ui.separator();

    ui.checkbox(&mut settings.hide_overscan, "Hide hide_overscan");
    ui.separator();

    ui.menu_button("Audio", |ui| {
         ui.add(egui::Slider::new(&mut settings.volume, 0.0_f32..=200.0_f32))
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Theme: "); 
        egui::global_theme_preference_switch(ui);
    });
}

fn change_palette(settings: &mut EmulatorConfig, ui: &mut egui_dock::egui::Ui) {
    let selected_text = match &settings.palette {
        PaletteTheme::Custom(name, _) => name.clone(),
        other => format!("{:?}", other),
    };
    egui::ComboBox::from_label("Palette")
    .selected_text(selected_text)
    .show_ui(ui, |ui| {
        ui.selectable_value(&mut settings.palette, PaletteTheme::DefaultNtsc, "NTSC");
        ui.selectable_value(&mut settings.palette, PaletteTheme::Nestopia,    "Nestopia");
        ui.selectable_value(&mut settings.palette, PaletteTheme::Fceux,   "Fceux");
        ui.selectable_value(&mut settings.palette, PaletteTheme::ShovelKnight, "ShovelKnight");
        ui.selectable_value(&mut settings.palette, PaletteTheme::SonyCxa, "SonyCxa");
        ui.selectable_value(&mut settings.palette, PaletteTheme::InvertedNtsc, "InvertedNtsc");

        let mut palette_to_remove: Option<String> = None;

        if !settings.custom_palettes.is_empty() {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                ui.spacing_mut().button_padding = egui::vec2(2.0, 2.0);
                ui.spacing_mut().interact_size.y = 14.0;

                for (name, colors) in &settings.custom_palettes {
                    ui.horizontal(|ui| {
                        if ui.button("×").clicked() {
                            palette_to_remove = Some(name.clone());
                        }

                        let is_selected = match &settings.palette {
                            PaletteTheme::Custom(current_name, _) => current_name == name,
                            _ => false,
                        };
                        
                        if ui.selectable_label(is_selected, name).clicked() {
                            settings.palette = PaletteTheme::Custom(name.clone(), colors.clone());
                        }
                    });
                }
            });
        }

        if let Some(pal) = palette_to_remove {
            settings.custom_palettes.remove(&pal);
            if let PaletteTheme::Custom(active_pal, _) = &settings.palette {
                if active_pal == &pal {
                    settings.palette = PaletteTheme::DefaultNtsc;
                }
            }
            settings.save();
        }

        ui.separator();
        
        if ui.button("Add Custom Palette").clicked() {
            if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nes Palette", &["pal"])
            .pick_file() {
                match PaletteTheme::create_palette_from_dot_pal(&path) {
                    Ok(colors_array) => {
                        let colors_vec = colors_array.to_vec();

                        let name = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();

                        settings.custom_palettes.insert(name.clone(), colors_vec.clone());
                        settings.palette = PaletteTheme::Custom(name, colors_vec);
                        settings.save();
                    }
                    Err(e) => print_logs(LogType::Warning ,format!("Palette file invalid or corrupted [Error: {}]", e)),
                }
            }
        }
    });
}