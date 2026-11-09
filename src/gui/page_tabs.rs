use crate::model::page::{PageLayoutConfig, TOTAL_PAGES};
use eframe::egui::{self, Color32, RichText, Ui};

pub fn render_page_tabs(
    ui: &mut Ui,
    current_page: &mut usize,
    pages: &[PageLayoutConfig],
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("페이지 선택:").strong());
        for i in 0..TOTAL_PAGES {
            let is_selected = *current_page == i;
            let tab_label = if i < pages.len() {
                pages[i].display_tab_name()
            } else {
                format!("Page {}", i + 1)
            };

            let btn_text = if is_selected {
                RichText::new(&tab_label).strong().color(Color32::from_rgb(255, 255, 255))
            } else {
                RichText::new(&tab_label).color(Color32::from_rgb(190, 190, 190))
            };

            let btn = egui::Button::new(btn_text).fill(if is_selected {
                Color32::from_rgb(40, 100, 180)
            } else {
                Color32::from_rgb(45, 45, 50)
            });

            let tooltip = if i < pages.len() {
                format!("Page {}: {}", i + 1, pages[i].title)
            } else {
                tab_label
            };

            if ui.add(btn).on_hover_text(tooltip).clicked() {
                *current_page = i;
            }
        }

        if *current_page < pages.len() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(
                RichText::new(&pages[*current_page].title)
                    .color(Color32::from_rgb(180, 215, 255))
                    .italics(),
            );
        }
    });
}
