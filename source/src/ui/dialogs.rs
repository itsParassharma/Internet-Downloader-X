use super::*;

fn classic_frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::window(&ctx.style())
        .fill(Color32::from_rgb(240, 240, 240))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(145)))
        .corner_radius(0)
        .inner_margin(12)
}

fn classic_controls(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing = Vec2::new(7.0, 6.0);
    ui.spacing_mut().button_padding = Vec2::new(10.0, 4.0);
    *ui.visuals_mut() = egui::Visuals::light();
    let visuals = ui.visuals_mut();
    visuals.panel_fill = Color32::from_rgb(240, 240, 240);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(228, 228, 228);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(228, 228, 228);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(160));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(225, 235, 245);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(225, 235, 245);
    visuals.widgets.active.bg_fill = Color32::from_rgb(205, 220, 235);
    let widgets = &mut visuals.widgets;
    for widget in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::ZERO;
    }
}

fn classic_tab(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    ui.add(
        egui::Button::new(label)
            .fill(if active {
                Color32::WHITE
            } else {
                Color32::from_rgb(232, 232, 232)
            })
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(160))),
    )
    .clicked()
}

impl IdxApp {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        if self.add.open {
            let mut open = true;
            let mut accept = false;
            let mut cancel = false;
            let edit = self.add.edit_id.is_some();
            egui::Window::new(if edit { "Download properties" } else { "Download File Info" })
                .open(&mut open).collapsible(false).resizable(true).default_width(600.0)
                .frame(classic_frame(ctx)).show(ctx, |ui| {
                    classic_controls(ui);
                    egui::Grid::new("download_file_info").num_columns(2).spacing([12.0, 9.0]).show(ui, |ui| {
                        ui.label("URL");
                        ui.add(egui::TextEdit::multiline(&mut self.add.urls).desired_rows(if edit { 2 } else { 3 }).desired_width(475.0).hint_text("https://example.com/file.zip"));
                        ui.end_row();
                        ui.label("Category");
                        egui::ComboBox::from_id_salt("add_category").width(215.0).selected_text(if self.add.category == 0 { "Automatic" } else { self.lib.settings.categories.get(self.add.category - 1).map_or("Other", |c| c.name.as_str()) }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.add.category, 0, "Automatic");
                            for (index, category) in self.lib.settings.categories.iter().enumerate() { ui.selectable_value(&mut self.add.category, index + 1, category.name.as_str()); }
                        });
                        ui.end_row();
                        ui.label("Save to");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.add.folder).desired_width(388.0));
                            if ui.button("Browse…").clicked() {
                                if let Some(path) = rfd::FileDialog::new().set_directory(&self.add.folder).pick_folder() { self.add.folder = path.to_string_lossy().into(); }
                            }
                        });
                        ui.end_row();
                        ui.label("File name");
                        ui.add(egui::TextEdit::singleline(&mut self.add.filename).hint_text("Automatic from URL").desired_width(475.0));
                        ui.end_row();
                        ui.label("Queue");
                        egui::ComboBox::from_id_salt("add_queue").width(300.0).selected_text(self.lib.settings.queues.get(self.add.queue).map(|queue| queue.name.as_str()).unwrap_or("Main download queue")).show_ui(ui, |ui| {
                            for (index, queue) in self.lib.settings.queues.iter().enumerate() { ui.selectable_value(&mut self.add.queue, index, &queue.name); }
                        });
                        ui.end_row();
                        ui.label("Description");
                        ui.add(egui::TextEdit::singleline(&mut self.add.description).desired_width(475.0));
                        ui.end_row();
                    });
                    if !edit { ui.label(RichText::new("Batch: one URL per line, or a range such as file[001-025].zip or [a-z].").small()); }
                    ui.add_space(7.0);
                    ui.checkbox(&mut self.add.advanced, "Authorization and other download options");
                    if self.add.advanced {
                        ui.group(|ui| {
                            egui::Grid::new("advanced").num_columns(2).spacing([12.0, 7.0]).show(ui, |ui| {
                                ui.label("User name"); ui.text_edit_singleline(&mut self.add.username); ui.end_row();
                                ui.label("Password"); ui.add(egui::TextEdit::singleline(&mut self.add.password).password(true)); ui.end_row();
                                ui.label("Referer"); ui.add(egui::TextEdit::singleline(&mut self.add.referer).desired_width(405.0)); ui.end_row();
                                ui.label("Expected SHA-256"); ui.add(egui::TextEdit::singleline(&mut self.add.sha256).desired_width(405.0)); ui.end_row();
                            });
                            ui.label(RichText::new("Website credentials are kept in memory only.").small());
                        });
                    }
                    if !self.add.error.is_empty() { ui.colored_label(Color32::from_rgb(170, 0, 0), &self.add.error); }
                    if edit { ui.checkbox(&mut self.add.start, "Start after saving changes"); }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([85.0, 26.0], egui::Button::new("Cancel")).clicked() { cancel = true; }
                        if ui.add_sized([125.0, 26.0], egui::Button::new(if edit { "Save changes" } else { "Start Download" })).clicked() { if !edit { self.add.start = true; } accept = true; }
                        if !edit && ui.add_sized([125.0, 26.0], egui::Button::new("Download later")).clicked() { self.add.start = false; accept = true; }
                    });
                });
            self.add.open = open && !cancel;
            if cancel || !open {
                self.add.password.clear();
            }
            if accept {
                self.accept_add();
            }
        }
        if self.options {
            let mut open = true;
            let mut apply = false;
            let mut close = false;
            let old_limit = self.lib.settings.speed_limit_kib;
            egui::Window::new("IDX Configuration").open(&mut open).collapsible(false).resizable(false).default_width(620.0)
                .frame(classic_frame(ctx)).show(ctx, |ui| {
                    classic_controls(ui);
                    ui.horizontal(|ui| {
                        for (index, label) in ["General", "Save to", "Downloads", "Connection", "Proxy"].iter().enumerate() {
                            if classic_tab(ui, label, self.options_tab == index) { self.options_tab = index; }
                        }
                    });
                    ui.group(|ui| {
                        ui.set_min_size(Vec2::new(586.0, 235.0));
                        match self.options_tab {
                            0 => {
                                ui.label(RichText::new("General settings").strong());
                                ui.add_space(10.0);
                                ui.checkbox(&mut self.lib.settings.confirm_delete, "Confirm removal from the download list");
                                if ui.checkbox(&mut self.lib.settings.dark, "Use dark interface").changed() { style(ctx, self.lib.settings.dark); }
                                ui.add_space(18.0);
                                ui.group(|ui| {
                                    ui.label("Browser integration");
                                    ui.label("The optional browser extension adds a Download with IDX command.");
                                    if ui.button("Installation instructions…").clicked() {
                                        self.message = Some("Open IDX once to configure the browser connection automatically. Install or enable the bundled extension, then use Download with IDX. See BROWSER-INTEGRATION.md for details. Automatic downloads start turned off.".into());
                                    }
                                });
                            }
                            1 => {
                                ui.label(RichText::new("Default download directory").strong());
                                ui.add_space(9.0);
                                ui.horizontal(|ui| {
                                    let mut value = self.lib.settings.download_dir.to_string_lossy().to_string();
                                    if ui.add(egui::TextEdit::singleline(&mut value).desired_width(465.0)).changed() { self.lib.settings.download_dir = value.into(); }
                                    if ui.button("Browse…").clicked() { if let Some(path) = rfd::FileDialog::new().pick_folder() { self.lib.settings.download_dir = path; } }
                                });
                                ui.add_space(12.0);
                                ui.checkbox(&mut self.lib.settings.category_folders, "Save downloads in category subfolders");
                                ui.add_space(12.0);
                                ui.group(|ui| {
                                    ui.label("Automatic categories");
                                    ui.label(self.lib.settings.categories.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("  ·  "));
                                    ui.label(RichText::new("The file extension determines the category for new downloads.").small());
                                });
                            }
                            2 => {
                                ui.label(RichText::new("Download settings").strong());
                                ui.add_space(12.0);
                                egui::Grid::new("download_options").spacing([22.0, 14.0]).show(ui, |ui| {
                                    ui.label("Automatic retries"); ui.add(egui::DragValue::new(&mut self.lib.settings.retries).range(0..=10)); ui.end_row();
                                    ui.label("Speed limit per download");
                                    ui.horizontal(|ui| { ui.add(egui::DragValue::new(&mut self.lib.settings.speed_limit_kib).range(0..=1_024_000)); ui.label("KB/s"); }); ui.end_row();
                                });
                                ui.add_space(10.0);
                                ui.label("Set the speed limit to 0 for unlimited speed.");
                                ui.label("Speed-limit changes apply to running downloads immediately.");
                                ui.add_space(18.0);
                                ui.label(RichText::new("Existing destination files are preserved. Interrupted transfers resume when the server supports safe byte ranges.").small());
                            }
                            3 => {
                                ui.label(RichText::new("Connection settings").strong());
                                ui.add_space(12.0);
                                egui::Grid::new("connection_options").spacing([22.0, 14.0]).show(ui, |ui| {
                                    ui.label("Maximum connections per file"); ui.add(egui::DragValue::new(&mut self.lib.settings.connections).range(1..=16)); ui.end_row();
                                    ui.label("User agent"); ui.add(egui::TextEdit::singleline(&mut self.lib.settings.user_agent).desired_width(355.0)); ui.end_row();
                                });
                                ui.add_space(16.0);
                                ui.label("Connection settings apply to newly started transfers.");
                                ui.label(RichText::new("Servers that do not support byte ranges use a single connection.").small());
                            }
                            _ => {
                                ui.label(RichText::new("Proxy server").strong());
                                ui.add_space(12.0);
                                ui.label("Proxy URL");
                                ui.add(egui::TextEdit::singleline(&mut self.lib.settings.proxy).hint_text("http://host:port or socks5://host:port").desired_width(550.0));
                                ui.add_space(12.0);
                                ui.label("Leave this field empty to use the system proxy settings.");
                                ui.label("HTTP and SOCKS5 proxy URLs are supported.");
                                ui.add_space(16.0);
                                ui.label(RichText::new("A proxy URL containing credentials is kept for this session only.").small());
                            }
                        }
                    });
                    ui.add_space(7.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([85.0, 25.0], egui::Button::new("Close")).clicked() { close = true; apply = true; }
                        if ui.add_sized([85.0, 25.0], egui::Button::new("Apply")).clicked() { apply = true; }
                    });
                });
            self.options = open && !close;
            if old_limit != self.lib.settings.speed_limit_kib {
                for (id, control) in &self.controls {
                    let limit = self
                        .lib
                        .downloads
                        .iter()
                        .find(|d| d.id == *id)
                        .and_then(|d| d.speed_limit_kib)
                        .unwrap_or(self.lib.settings.speed_limit_kib);
                    control.set_speed_limit(limit);
                }
            }
            if apply || !open {
                self.save();
            }
        }
        if self.scheduler {
            let mut open = true;
            let mut close = false;
            let mut start = None;
            let mut stop = None;
            let mut save = false;
            egui::Window::new("Scheduler").open(&mut open).collapsible(false).resizable(false).default_width(755.0)
                .frame(classic_frame(ctx)).show(ctx, |ui| {
                    classic_controls(ui);
                    ui.horizontal_top(|ui| {
                        egui::Frame::group(ui.style()).fill(Color32::WHITE).show(ui, |ui| {
                            ui.set_width(190.0);
                            egui::ScrollArea::vertical().min_scrolled_height(290.0).max_height(290.0).show(ui, |ui| {
                                for (index, queue) in self.lib.settings.queues.iter().enumerate() {
                                    if ui.selectable_label(self.queue_index == index, &queue.name).clicked() { self.queue_index = index; }
                                }
                            });
                        });
                        ui.vertical(|ui| {
                            ui.set_width(505.0);
                            ui.horizontal(|ui| {
                                if classic_tab(ui, "Schedule", self.scheduler_tab == 0) { self.scheduler_tab = 0; }
                                if classic_tab(ui, "Files in the queue", self.scheduler_tab == 1) { self.scheduler_tab = 1; }
                            });
                            ui.group(|ui| {
                                ui.set_min_size(Vec2::new(483.0, 262.0));
                                if let Some(queue) = self.lib.settings.queues.get_mut(self.queue_index) {
                                    ui.horizontal(|ui| { ui.label(RichText::new(&queue.name).strong()); ui.label(if queue.active { "(running)" } else { "(stopped)" }); });
                                    ui.add_space(10.0);
                                    if self.scheduler_tab == 0 {
                                        ui.checkbox(&mut queue.scheduled, "Start download at");
                                        ui.add_enabled_ui(queue.scheduled, |ui| {
                                            ui.horizontal(|ui| { ui.add_space(22.0); ui.add(egui::DragValue::new(&mut queue.start_hour).range(0..=23).min_decimals(0)); ui.label(":"); ui.add(egui::DragValue::new(&mut queue.start_minute).range(0..=59)); ui.label("Local time"); });
                                            ui.add_space(12.0);
                                            ui.label("Run on these days:");
                                            ui.horizontal(|ui| { for (index, day) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().enumerate() { ui.checkbox(&mut queue.weekdays[index], *day); } });
                                            ui.add_space(14.0);
                                            ui.checkbox(&mut queue.stop_enabled, "Stop downloads at");
                                            ui.horizontal(|ui| { ui.add_space(22.0); ui.add_enabled(queue.stop_enabled, egui::DragValue::new(&mut queue.stop_hour).range(0..=23)); ui.label(":"); ui.add_enabled(queue.stop_enabled, egui::DragValue::new(&mut queue.stop_minute).range(0..=59)); ui.label("Local time"); });
                                        });
                                        ui.add_space(16.0);
                                        ui.label(RichText::new("IDX must be running at the scheduled time. The schedule does not wake a sleeping computer.").small());
                                    } else {
                                        ui.horizontal(|ui| { ui.label("Download"); ui.add(egui::DragValue::new(&mut queue.parallel).range(1..=8)); ui.label("files at the same time"); });
                                        let queue_name = queue.name.clone();
                                        let queue_indices: Vec<_> = self.lib.downloads.iter().enumerate().filter(|(_, download)| download.queue == queue_name && download.status != Status::Complete).map(|(index, _)| index).collect();
                                        let mut move_item = None;
                                        ui.add_space(8.0);
                                        egui::Frame::group(ui.style()).fill(Color32::WHITE).show(ui, |ui| {
                                            egui::ScrollArea::vertical().min_scrolled_height(168.0).max_height(168.0).show(ui, |ui| {
                                                if queue_indices.is_empty() { ui.label("No unfinished files in this queue."); }
                                                for (position, &index) in queue_indices.iter().enumerate() {
                                                    ui.horizontal(|ui| {
                                                        if ui.add_enabled(position > 0, egui::Button::new("↑")).clicked() { move_item = Some((index, queue_indices[position - 1])); }
                                                        if ui.add_enabled(position + 1 < queue_indices.len(), egui::Button::new("↓")).clicked() { move_item = Some((index, queue_indices[position + 1])); }
                                                        ui.add_sized([265.0, 19.0], egui::Label::new(self.lib.downloads[index].name()).truncate()).on_hover_text(self.lib.downloads[index].name());
                                                        ui.label(self.lib.downloads[index].status.label());
                                                    });
                                                }
                                            });
                                        });
                                        if let Some((first, second)) = move_item { self.lib.downloads.swap(first, second); }
                                        ui.label(RichText::new("Use the arrows to change download order.").small());
                                    }
                                }
                            });
                        });
                    });
                    ui.add_space(9.0);
                    ui.horizontal(|ui| {
                        ui.label("New queue:");
                        ui.add(egui::TextEdit::singleline(&mut self.new_queue).desired_width(250.0));
                        if ui.button("Create queue").clicked() {
                            let name = self.new_queue.trim();
                            if !name.is_empty() && !self.lib.settings.queues.iter().any(|queue| queue.name.eq_ignore_ascii_case(name)) { self.lib.settings.queues.push(Queue { name: name.into(), ..Default::default() }); self.queue_index = self.lib.settings.queues.len() - 1; self.new_queue.clear(); }
                        }
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.add_sized([90.0, 25.0], egui::Button::new("Start now")).clicked() { start = Some(self.queue_index); }
                        if ui.add_sized([90.0, 25.0], egui::Button::new("Stop")).clicked() { stop = Some(self.queue_index); }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_sized([85.0, 25.0], egui::Button::new("Close")).clicked() { close = true; save = true; }
                            if ui.add_sized([85.0, 25.0], egui::Button::new("Apply")).clicked() { save = true; }
                        });
                    });
                });
            self.scheduler = open && !close;
            if let Some(index) = start {
                self.start_queue(index);
            }
            if let Some(index) = stop {
                self.stop_queue(index);
            }
            if save || !open {
                self.save();
            }
        }

        if self.grabber {
            let mut open = true;
            let mut add_urls = None;
            egui::Window::new("Site grabber — page links").open(&mut open).default_width(700.0).default_height(460.0).show(ctx,|ui|{
                ui.label("Find downloadable links and media referenced by a web page.");
                ui.horizontal(|ui|{ui.add(egui::TextEdit::singleline(&mut self.grab_url).hint_text("https://example.com/downloads").desired_width(540.0));if ui.add_enabled(self.grab_rx.is_none(),egui::Button::new("Scan page")).clicked(){self.start_grabber(crate::grabber::ScanOptions::default());}});
                ui.horizontal(|ui|{ui.label("Filter");ui.add(egui::TextEdit::singleline(&mut self.grab_filter).hint_text(".zip, .pdf, .mp4 or any text").desired_width(370.0));if ui.button("Select visible").clicked(){for(on,url)in &mut self.grab_links{if self.grab_filter.is_empty()||url.to_lowercase().contains(&self.grab_filter.to_lowercase()){*on=true;}}}if ui.button("Clear").clicked(){for(on,_)in &mut self.grab_links{*on=false;}}});
                if self.grab_rx.is_some(){ui.horizontal(|ui|{ui.spinner();ui.label("Reading page…");});}
                if !self.grab_error.is_empty(){ui.colored_label(Color32::from_rgb(185,60,60),&self.grab_error);}
                ui.separator();egui::ScrollArea::vertical().max_height(290.0).show(ui,|ui|{for(on,url)in &mut self.grab_links{if self.grab_filter.is_empty()||url.to_lowercase().contains(&self.grab_filter.to_lowercase()){ui.checkbox(on,url.as_str());}}});
                ui.separator();ui.horizontal(|ui|{let count=self.grab_links.iter().filter(|(on,_)|*on).count();ui.label(format!("{} links found · {count} selected",self.grab_links.len()));if ui.add_enabled(count>0,egui::Button::new("Add selected downloads…")).clicked(){add_urls=Some(self.grab_links.iter().filter(|(on,_)|*on).map(|(_,u)|u.clone()).collect::<Vec<_>>().join("\n"));}});
                ui.label(RichText::new("Scans the page's HTML. JavaScript-generated links, protected video streams and recursive mirroring are not supported.").small().weak());
            });
            self.grabber = open;
            if !open {
                self.cancel_grabber();
            }
            if let Some(u) = add_urls {
                self.grabber = false;
                self.add_dialog(u);
                self.add.parent_page = self.grab_page.clone();
                self.add.parent_pages = self.grab_parents.clone();
            }
        }
        if self.about {
            egui::Window::new("About IDX").open(&mut self.about).collapsible(false).default_width(430.0).show(ctx,|ui|{
            ui.heading(RichText::new("IDX").size(38.0).color(BLUE));ui.label(RichText::new("Internet Downloader X").size(20.0));ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
            ui.separator();ui.label("A free, open-source download manager with segmented transfers, queues and a familiar desktop interface.");ui.label("No activation. No registration. No trial period.");
            ui.add_space(8.0);ui.label("Original Rust source: MIT license. Third-party dependencies retain their own licenses. See the source folder for build instructions and the feature coverage report.");
            ui.label(RichText::new("Built in Rust.").small().weak());
        });
        }
        if self.help {
            egui::Window::new("Getting started with IDX").open(&mut self.help).default_width(580.0).show(ctx,|ui|{
            ui.heading("Your first download");
            for s in ["1. Choose Add URL, paste a direct HTTP or HTTPS file URL, and select a destination.","2. Click Start Download to start the queue. Download later keeps it paused until you start it.","3. Select a row to inspect progress and individual connections.","4. Stop pauses a transfer. Resume continues when the server supports safe ranges.","5. Use Scheduler to control simultaneous downloads and weekly start/stop times.","6. Grabber finds links on a single HTML page. Import accepts one URL per line.","7. Browser integration is optional; installation instructions are in the supplied browser-extension folder."]{ui.label(s);ui.add_space(6.0);}
            ui.separator();ui.label("Files with identical names get a numeric suffix. Removing a row never deletes a downloaded file. Passwords are session-only. Download state is saved automatically.");
            ui.label("HTTP/HTTPS are supported. FTP, browser-cookie import, DRM/video stream capture and background system services are not implemented in this build.");
        });
        }
        if !self.delete_ids.is_empty() {
            let mut close = false;
            let mut remove = false;
            egui::Window::new("Remove download?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Remove {} selected entr{} from the download list?",
                        self.delete_ids.len(),
                        if self.delete_ids.len() == 1 {
                            "y"
                        } else {
                            "ies"
                        }
                    ));
                    ui.label("Downloaded files and partial data will remain on disk.");
                    ui.horizontal(|ui| {
                        if ui.button("Remove selected").clicked() {
                            remove = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
            if remove {
                let ids = self.delete_ids.clone();
                self.remove_many(&ids);
            }
            if remove || close {
                self.delete_ids.clear();
            }
        }
        if let Some(message) = self.message.clone() {
            let mut open = true;
            egui::Window::new("IDX")
                .open(&mut open)
                .collapsible(false)
                .default_width(480.0)
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("OK").clicked() {
                        self.message = None;
                    }
                });
            if !open {
                self.message = None;
            }
        }
    }
}
