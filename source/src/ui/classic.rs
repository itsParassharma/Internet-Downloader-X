use super::*;

pub(super) fn install_font(ctx: &egui::Context) {
    #[cfg(windows)]
    if let Some(windows) = std::env::var_os("WINDIR") {
        if let Ok(bytes) = std::fs::read(PathBuf::from(windows).join("Fonts/segoeui.ttf")) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Windows UI".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Windows UI".into());
            ctx.set_fonts(fonts);
        }
    }
}

pub(super) fn style(ctx: &egui::Context, dark: bool) {
    let mut style = egui::Style::default();
    style.visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.spacing.item_spacing = Vec2::new(5.0, 4.0);
    style.spacing.button_padding = Vec2::new(10.0, 3.0);
    style.spacing.interact_size = Vec2::new(32.0, 22.0);
    style.spacing.window_margin = egui::Margin::same(10);
    style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.selection.bg_fill = if dark {
        Color32::from_rgb(31, 73, 112)
    } else {
        Color32::from_rgb(204, 232, 255)
    };
    style.visuals.selection.stroke =
        egui::Stroke::new(1.0_f32, if dark { Color32::WHITE } else { Color32::BLACK });
    if !dark {
        style.visuals.override_text_color = Some(Color32::BLACK);
        style.visuals.panel_fill = Color32::from_rgb(240, 240, 240);
        style.visuals.window_fill = Color32::from_rgb(240, 240, 240);
        style.visuals.window_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(150));
        style.visuals.extreme_bg_color = Color32::WHITE;
        style.visuals.faint_bg_color = Color32::WHITE;
        style.visuals.widgets.noninteractive.bg_fill = Color32::from_gray(240);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_gray(185));
        style.visuals.widgets.inactive.bg_fill = Color32::from_gray(240);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(240);
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_gray(160));
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(229, 241, 251);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
        style.visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 215));
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(204, 228, 247);
        style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(204, 228, 247);
        style.visuals.widgets.active.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 215));
    }
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::ZERO;
    }
    for text_style in [
        egui::TextStyle::Body,
        egui::TextStyle::Button,
        egui::TextStyle::Small,
    ] {
        style
            .text_styles
            .insert(text_style, egui::FontId::proportional(12.0));
    }
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(14.0));
    ctx.set_style(style);
}

pub(super) fn toolbar_button(
    ui: &mut egui::Ui,
    art: &crate::ui_assets::UiAssets,
    label: &str,
    index: usize,
    enabled: bool,
) -> bool {
    let width = if label == "Delete Completed" {
        99.0
    } else {
        77.0
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 68.0), egui::Sense::click());
    if enabled && response.hovered() {
        ui.painter()
            .rect_filled(rect.shrink(1.0), 0.0, ui.visuals().widgets.hovered.bg_fill);
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            0.0,
            ui.visuals().widgets.hovered.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let texture = if enabled {
        art.toolbar(index, response.hovered())
    } else {
        art.toolbar_disabled(index)
    };
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 25.0),
        Vec2::new(56.0, 42.0),
    );
    ui.painter().image(
        texture.id(),
        image,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 6.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        egui::FontId::proportional(12.0),
        if enabled {
            ui.visuals().text_color()
        } else {
            Color32::from_gray(150)
        },
    );
    response.on_hover_text(label).clicked() && enabled
}

pub(super) fn tree_row(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    label: &str,
    selected: bool,
    depth: usize,
    branch: Option<bool>,
) -> (egui::Response, bool) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), egui::Sense::click());
    let x = rect.left() + depth as f32 * 18.0;
    let center = rect.center().y;
    let toggle = egui::Rect::from_center_size(egui::pos2(x + 8.0, center), Vec2::splat(9.0));
    if depth > 0 {
        let line = egui::Stroke::new(1.0_f32, Color32::from_gray(190));
        for n in (0..20).step_by(3) {
            ui.painter().line_segment(
                [
                    egui::pos2(x - 10.0, rect.top() + n as f32),
                    egui::pos2(x - 10.0, rect.top() + n as f32 + 1.0),
                ],
                line,
            );
        }
        ui.painter().line_segment(
            [egui::pos2(x - 10.0, center), egui::pos2(x + 4.0, center)],
            line,
        );
    }
    if let Some(expanded) = branch {
        ui.painter()
            .rect_filled(toggle, 0.0, ui.visuals().extreme_bg_color);
        ui.painter().rect_stroke(
            toggle,
            0.0,
            egui::Stroke::new(1.0_f32, Color32::from_gray(145)),
            egui::StrokeKind::Inside,
        );
        let stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(80));
        ui.painter().line_segment(
            [
                toggle.center() - Vec2::new(3.0, 0.0),
                toggle.center() + Vec2::new(3.0, 0.0),
            ],
            stroke,
        );
        if !expanded {
            ui.painter().line_segment(
                [
                    toggle.center() - Vec2::new(0.0, 3.0),
                    toggle.center() + Vec2::new(0.0, 3.0),
                ],
                stroke,
            );
        }
    }
    let image =
        egui::Rect::from_min_size(egui::pos2(x + 17.0, center - 6.5), Vec2::new(16.0, 13.0));
    ui.painter().image(
        texture.id(),
        image,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    let textrect = egui::Rect::from_min_max(egui::pos2(x + 36.0, rect.top()), rect.right_bottom());
    if selected {
        ui.painter()
            .rect_filled(textrect, 0.0, ui.visuals().selection.bg_fill);
    }
    ui.painter().with_clip_rect(textrect).text(
        textrect.left_center(),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    let toggled = response.clicked()
        && response
            .interact_pointer_pos()
            .is_some_and(|pos| toggle.expand(3.0).contains(pos));
    (response, toggled)
}
