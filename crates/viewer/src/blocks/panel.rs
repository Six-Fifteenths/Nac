//! 面板：盒子、表格、提示框、页签。

use crate::*;

impl Tab {
    pub(crate) fn render_box(&mut self, ui: &mut egui::Ui, style: &Style, children: Vec<Block>, text: Text) {
        let sheet = self.sheet.clone();
        let class = style
            .class
            .as_ref()
            .and_then(|name| sheet.get(name))
            .cloned()
            .unwrap_or_default();
        let merged = style.over(&class);
        let inner = text.inherit(&merged);

        let available = ui.available_width();
        let rect = ui.available_rect_before_wrap();
        // 内边距是客户端的事：有底色或描边的盒子自己知道要让文字离边远一点，
        // 页面不用（也不能）说这个数。
        let pad = if merged.bg.is_some() || merged.border.is_some() {
            BOX_PAD
        } else {
            (0.0, 0.0)
        };
        let mut frame = egui::Frame::new().inner_margin(egui::Margin {
            left: pad.0 as i8,
            right: pad.0 as i8,
            top: pad.1 as i8,
            bottom: pad.1 as i8,
        });
        if let Some(bg) = merged.bg {
            frame = frame.fill(bg);
        }
        if let Some((width, colour)) = merged.border {
            frame = frame.stroke(egui::Stroke::new(width, colour));
        }
        if let Some(radius) = merged.radius {
            frame = frame.corner_radius(egui::CornerRadius::same(radius as u8));
        }
        if let Some(shadow) = merged.shadow {
            frame = frame.shadow(egui::epaint::Shadow {
                offset: [0, shadow.offset as i8],
                blur: shadow.blur as u8,
                spread: 0,
                color: shadow.color,
            });
        }

        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                let width = available;
                ui.set_min_width(width);
                ui.set_max_width(width);
                if let Some(amount) = merged.opacity {
                    ui.set_opacity(amount);
                }
                frame.show(ui, |ui| {
                    let inside = (width - pad.0 * 2.0).max(20.0);
                    ui.set_min_width(inside);
                    ui.set_max_width(inside);
                    {
                        let cross = match inner.align {
                            Some(style::Align::Center) => egui::Align::Center,
                            Some(style::Align::Right) => egui::Align::Max,
                            _ => egui::Align::Min,
                        };
                        if cross == egui::Align::Min {
                            self.render_list(ui, children, inner);
                        } else {
                            ui.with_layout(Layout::top_down(cross), |ui| {
                                self.render_list(ui, children, inner)
                            });
                        }
                    }
                })
            },
        );
    }

    /// A monogram avatar: the name picks a colour, the name's first letter is
    /// drawn on it. No network, no file, and the same name always looks the
    /// same, so two visits to the same profile show the same face.

    pub(crate) fn render_table(
        &mut self,
        ui: &mut egui::Ui,
        header: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
    ) {
        let columns = header.len().max(1);
        let id = ui.next_auto_id();
        egui::Grid::new(id)
            .num_columns(columns)
            .spacing([18.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                for cell in header {
                    ui.label(self.job(
                        cell,
                        15.5,
                        Color32::from_gray(15),
                        FontFamily::Name(Arc::from("bold")),
                    ));
                }
                ui.end_row();
                for row in rows {
                    for index in 0..columns {
                        match row.get(index) {
                            Some(cell) => self.render_flow_with(
                                ui,
                                cell,
                                16.0,
                                Color32::from_gray(28),
                                FontFamily::Proportional,
                                false,
                            ),
                            None => {
                                ui.label("");
                            }
                        };
                    }
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
    }

    pub(crate) fn render_note(
        &mut self,
        ui: &mut egui::Ui,
        text: Text,
        kind: NoteKind,
        title: &[Inline],
        body: Vec<Block>,
    ) {
        let (bar, fill) = match kind {
            NoteKind::Info => (Color32::from_rgb(74, 132, 199), Color32::from_rgb(241, 246, 253)),
            NoteKind::Ok => (Color32::from_rgb(64, 152, 101), Color32::from_rgb(241, 250, 244)),
            NoteKind::Warn => (Color32::from_rgb(206, 156, 60), Color32::from_rgb(254, 250, 240)),
            NoteKind::Danger => (Color32::from_rgb(198, 82, 82), Color32::from_rgb(253, 243, 243)),
        };
        let frame = egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin::symmetric(10, 8));
        let full_width = ui.available_width();
        frame.show(ui, |ui| {
            // Without this the box shrinks to whatever the text happens to need,
            // so a page with several notes gets boxes of different widths.
            ui.set_min_width((full_width - 20.0).max(60.0));
            if !title.is_empty() {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(4.0, 15.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, bar);
                    ui.add_space(6.0);
                    ui.label(self.job(
                        title,
                        16.0,
                        bar,
                        FontFamily::Name(Arc::from("bold")),
                    ));
                });
                ui.add_space(4.0);
            }
            self.render_list(ui, body, text);
        });
        ui.add_space(8.0);
    }

    pub(crate) fn render_tabs(
        &mut self,
        ui: &mut egui::Ui,
        titles: &[String],
        bodies: Vec<Vec<Block>>,
        text: Text,
    ) {
        let id = ui.next_auto_id();
        let mut selected = ui.data_mut(|d| *d.get_temp_mut_or_default::<usize>(id));
        if selected >= titles.len() {
            selected = 0;
        }
        ui.horizontal_wrapped(|ui| {
            for (index, title) in titles.iter().enumerate() {
                if ui.selectable_label(index == selected, title.as_str()).clicked() {
                    selected = index;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(id, selected));
        ui.add_space(6.0);
        if let Some(body) = bodies.into_iter().nth(selected) {
            self.render_list(ui, body, text);
        }
        ui.add_space(6.0);
    }
}
