use super::*;

const PANEL_BLUE: Color32 = Color32::from_rgb(0, 0, 160);
const RECEIVING_BLUE: Color32 = Color32::from_rgb(0, 0, 255);

impl IdxApp {
    pub(super) fn progress(&mut self, ctx: &egui::Context) {
        if !self.transfer_open {
            return;
        }
        let Some(mut download) = self
            .selected
            .and_then(|id| self.lib.downloads.iter().find(|download| download.id == id))
            .cloned()
        else {
            self.transfer_open = false;
            return;
        };
        let before = (
            download.speed_limit_kib,
            download.notify_on_complete,
            download.open_folder_on_complete,
            download.close_progress_on_complete,
        );
        let global_limit = self.lib.settings.speed_limit_kib;
        let percent = if download.status == Status::Complete {
            1.0
        } else {
            download.percent()
        };
        let mut open = true;
        let mut pause = false;
        let mut resume = false;
        let mut cancel = false;
        let title = format!("{:.0}% {} — IDX", percent * 100.0, download.name());
        egui::Window::new(title)
            .id(egui::Id::new("idx_download_progress"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_rgb(240, 240, 240)).corner_radius(0).inner_margin(10).stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(150))))
            .show(ctx, |ui| {
                progress_style(ui);
                ui.set_width(506.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (index, label) in ["Download status", "Speed Limiter", "Options on completion"].iter().enumerate() {
                        let active = self.detail_tab == index;
                        if ui.add(egui::Button::new(*label).fill(if active { Color32::WHITE } else { Color32::from_rgb(239, 239, 239) }).stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(205)))).clicked() { self.detail_tab = index; }
                    }
                });
                egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(210))).inner_margin(12).show(ui, |ui| {
                    ui.set_min_size(Vec2::new(480.0, 119.0));
                    match self.detail_tab {
                        0 => status_panel(ui, &download),
                        1 => {
                            let effective = download.speed_limit_kib.unwrap_or(global_limit);
                            if effective > 0 { self.progress_limit_input = effective; }
                            let mut enabled = effective > 0;
                            if ui.checkbox(&mut enabled, "Use speed limiter for this download").changed() {
                                download.speed_limit_kib = Some(if enabled { self.progress_limit_input.max(1) } else { 0 });
                            }
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.label("Maximum download speed:");
                                if ui.add_enabled(enabled, egui::DragValue::new(&mut self.progress_limit_input).range(1..=1_024_000).speed(1)).changed() { download.speed_limit_kib = Some(self.progress_limit_input.max(1)); }
                                ui.label("KB/sec");
                            });
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                if ui.button("Use global setting").clicked() { download.speed_limit_kib = None; }
                                ui.label(if global_limit == 0 { "Global setting: Unlimited".to_string() } else { format!("Global setting: {global_limit} KB/sec") });
                            });
                            ui.add_space(6.0);
                            ui.label(RichText::new("Changes take effect immediately.").small());
                        }
                        _ => {
                            ui.checkbox(&mut download.notify_on_complete, "Show download complete notification");
                            ui.add_space(6.0);
                            ui.checkbox(&mut download.open_folder_on_complete, "Open the containing folder when complete");
                            ui.add_space(6.0);
                            ui.checkbox(&mut download.close_progress_on_complete, "Close this progress window when complete");
                            ui.add_space(12.0);
                            ui.label(RichText::new("Completion actions wait for any requested checksum verification.").small());
                        }
                    }
                });
                ui.add_space(8.0);
                let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 15.0), egui::Sense::hover());
                ui.painter().rect_filled(bar, 0, Color32::from_rgb(230, 230, 230));
                if percent > 0.0 {
                    ui.painter().rect_filled(egui::Rect::from_min_size(bar.min, Vec2::new(bar.width() * percent.clamp(0.0, 1.0), bar.height())), 0, Color32::from_rgb(0, 180, 0));
                }
                ui.painter().rect_stroke(bar, 0, egui::Stroke::new(1.0_f32, Color32::from_gray(185)), egui::StrokeKind::Inside);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_sized([138.0, 23.0], egui::Button::new(if self.progress_show_details { "<< Hide details" } else { ">> Show details" })).clicked() { self.progress_show_details = !self.progress_show_details; }
                    ui.add_space((ui.available_width() - 186.0).max(0.0));
                    let resumable_state = matches!(download.status, Status::Paused | Status::Error);
                    let actionable = download.status.active() || download.status == Status::Queued || resumable_state;
                    if ui.add_enabled(actionable, egui::Button::new(if resumable_state { "Resume" } else { "Pause" }).min_size(Vec2::new(82.0, 23.0))).clicked() { if resumable_state { resume = true; } else { pause = true; } }
                    ui.add_space(10.0);
                    if ui.add_sized([75.0, 23.0], egui::Button::new(if download.status == Status::Complete { "Close" } else { "Cancel" })).clicked() { cancel = true; }
                });
                if self.progress_show_details {
                    ui.add_space(3.0);
                    ui.vertical_centered(|ui| { ui.label(RichText::new("Start positions and download progress by connections").color(PANEL_BLUE)); });
                    segment_map(ui, &download);
                    ui.add_space(7.0);
                    connection_table(ui, &download);
                }
            });
        let after = (
            download.speed_limit_kib,
            download.notify_on_complete,
            download.open_folder_on_complete,
            download.close_progress_on_complete,
        );
        if before != after {
            if let Some(saved) = self
                .lib
                .downloads
                .iter_mut()
                .find(|item| item.id == download.id)
            {
                saved.speed_limit_kib = download.speed_limit_kib;
                saved.notify_on_complete = download.notify_on_complete;
                saved.open_folder_on_complete = download.open_folder_on_complete;
                saved.close_progress_on_complete = download.close_progress_on_complete;
            }
            if before.0 != after.0 {
                if let Some(control) = self.controls.get(&download.id) {
                    control.set_speed_limit(download.speed_limit_kib.unwrap_or(global_limit));
                }
            }
            self.save();
        }
        if pause {
            self.pause(download.id);
        }
        if resume {
            self.start(download.id);
        }
        if cancel {
            if download.status.active() || download.status == Status::Queued {
                self.pause(download.id);
            }
            self.transfer_open = false;
        }
        self.transfer_open &= open;
    }
}

fn progress_style(ui: &mut egui::Ui) {
    *ui.visuals_mut() = egui::Visuals::light();
    ui.spacing_mut().item_spacing = Vec2::new(5.0, 3.0);
    ui.spacing_mut().button_padding = Vec2::new(5.0, 2.0);
    ui.spacing_mut().interact_size.y = 14.0;
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(11.5));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(11.5));
    let visuals = ui.visuals_mut();
    visuals.override_text_color = Some(PANEL_BLUE);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(228, 228, 228);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(228, 228, 228);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(170));
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

fn status_panel(ui: &mut egui::Ui, download: &Download) {
    ui.add(egui::Label::new(RichText::new(&download.url).color(PANEL_BLUE)).truncate())
        .on_hover_text(&download.url);
    ui.add_space(3.0);
    let status = match download.status {
        Status::Queued => "Waiting in queue…",
        Status::Connecting => "Connecting…",
        Status::Downloading => "Receiving data…",
        Status::Paused => "Download paused.",
        Status::Verifying => "Verifying SHA-256 checksum…",
        Status::Complete => "Download complete.",
        Status::Error => &download.error,
    };
    let time_left = if download.speed > 0.0 {
        download
            .total
            .map(|total| {
                remaining_text(total.saturating_sub(download.downloaded) as f64 / download.speed)
            })
            .unwrap_or_else(|| "Unknown".into())
    } else if download.status == Status::Complete {
        "0 sec".into()
    } else {
        "Unknown".into()
    };
    let downloaded = format!(
        "{}  ( {:.2} % )",
        transfer_bytes(download.downloaded),
        if download.status == Status::Complete {
            100.0
        } else {
            100.0 * download.percent()
        }
    );
    egui::Grid::new("progress_status_grid")
        .num_columns(2)
        .min_col_width(89.0)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            ui.label("Status");
            ui.add(
                egui::Label::new(RichText::new(status).color(
                    if download.status == Status::Error {
                        Color32::from_rgb(180, 0, 0)
                    } else {
                        RECEIVING_BLUE
                    },
                ))
                .truncate(),
            )
            .on_hover_text(status);
            ui.end_row();
            ui.label("File size");
            ui.label(
                download
                    .total
                    .map(transfer_bytes)
                    .unwrap_or_else(|| "Unknown".into()),
            );
            ui.end_row();
            ui.label("Downloaded");
            ui.label(downloaded);
            ui.end_row();
            ui.label("Transfer rate");
            ui.label(format!(
                "{}/sec",
                transfer_bytes(download.speed.max(0.0) as u64)
            ));
            ui.end_row();
            ui.label("Time left");
            ui.label(time_left);
            ui.end_row();
            ui.label("Resume capability");
            ui.label(if download.resumable {
                "Yes"
            } else if matches!(download.status, Status::Connecting | Status::Queued) {
                "Unknown"
            } else {
                "No"
            });
            ui.end_row();
        });
}

fn transfer_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.3} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.3} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.3} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} Bytes")
    }
}

fn remaining_text(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "Unknown".into();
    }
    let seconds = seconds.ceil() as u64;
    if seconds >= 3600 {
        format!("{} hour {:02} min", seconds / 3600, seconds % 3600 / 60)
    } else if seconds >= 60 {
        format!("{} min {:02} sec", seconds / 60, seconds % 60)
    } else {
        format!("{seconds} sec")
    }
}

fn segment_map(ui: &mut egui::Ui, download: &Download) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), egui::Sense::hover());
    let inner = rect.shrink(1.0);
    ui.painter()
        .rect_filled(rect, 0, Color32::from_rgb(244, 244, 248));
    let total: u64 = download.parts.iter().map(|(_, total)| *total).sum();
    if total > 0 {
        let mut position = 0u64;
        for (received, size) in &download.parts {
            let start = inner.left() + inner.width() * position as f32 / total as f32;
            let filled = inner.width() * (*received).min(*size) as f32 / total as f32;
            if filled > 0.0 {
                let colors = [
                    Color32::from_rgb(147, 214, 237),
                    Color32::from_rgb(94, 181, 225),
                    Color32::from_rgb(53, 139, 207),
                    Color32::from_rgb(49, 105, 175),
                    Color32::from_rgb(84, 143, 184),
                    Color32::from_rgb(73, 191, 129),
                ];
                for (band, color) in colors.iter().enumerate() {
                    let y = inner.top() + band as f32 * inner.height() / colors.len() as f32;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(start, y),
                            Vec2::new(filled, inner.height() / colors.len() as f32 + 0.25),
                        ),
                        0,
                        *color,
                    );
                }
            }
            ui.painter().line_segment(
                [
                    egui::pos2(start, inner.top()),
                    egui::pos2(start, inner.bottom()),
                ],
                egui::Stroke::new(0.6_f32, Color32::from_rgb(234, 99, 103)),
            );
            position += *size;
        }
    }
    ui.painter().rect_stroke(
        rect,
        0,
        egui::Stroke::new(1.0_f32, Color32::from_rgb(119, 120, 150)),
        egui::StrokeKind::Inside,
    );
}

fn connection_table(ui: &mut egui::Ui, download: &Download) {
    egui::Frame::new()
        .fill(Color32::from_rgb(190, 184, 194))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(160)))
        .show(ui, |ui| {
            let header_width = ui.available_width();
            egui::Frame::new()
                .fill(Color32::WHITE)
                .inner_margin(egui::Margin::symmetric(5, 3))
                .show(ui, |ui| {
                    ui.set_min_width((header_width - 10.0).max(0.0));
                    ui.horizontal(|ui| {
                        ui.add_sized([23.0, 16.0], egui::Label::new("N."));
                        ui.add_sized([96.0, 16.0], egui::Label::new("Downloaded"));
                        ui.label("Info");
                    });
                });
            egui::ScrollArea::vertical()
                .min_scrolled_height(100.0)
                .max_height(100.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if download.parts.is_empty() {
                        ui.label(if download.status.active() {
                            "  Connecting to the server…"
                        } else {
                            "  Connection details are available while downloading."
                        });
                    }
                    for (index, (received, total)) in download.parts.iter().enumerate() {
                        let info = if *total > 0 && received >= total {
                            "Complete"
                        } else {
                            match download.status {
                                Status::Downloading if *received > 0 => "Receiving data…",
                                Status::Downloading | Status::Connecting => "Waiting for data…",
                                Status::Paused => "Paused",
                                Status::Error => "Stopped",
                                Status::Verifying | Status::Complete => "Complete",
                                Status::Queued => "Queued",
                            }
                        };
                        ui.horizontal(|ui| {
                            ui.add_space(5.0);
                            ui.add_sized([23.0, 14.0], egui::Label::new((index + 1).to_string()));
                            ui.add_sized([96.0, 14.0], egui::Label::new(transfer_bytes(*received)));
                            ui.label(info);
                        });
                    }
                });
        });
}
