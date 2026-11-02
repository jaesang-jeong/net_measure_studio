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
            let page_label = format!("Page {}", i + 1);

            let btn_text = if is_selected {
                RichText::new(&page_label).strong().color(Color32::from_rgb(255, 255, 255))
            } else {
                RichText::new(&page_label).color(Color32::from_rgb(180, 180, 180))
            };

            let btn = egui::Button::new(btn_text).fill(if is_selected {
                Color32::from_rgb(40, 100, 180)
            } else {
                Color32::from_rgb(45, 45, 50)
            });

            if ui.add(btn).clicked() {
                *current_page = i;
            }
        }

        if *current_page < pages.len() {
            ui.add_space(16.0);
            ui.label(
                RichText::new(&pages[*current_page].title)
                    .color(Color32::from_rgb(100, 180, 255))
                    .italics(),
            );
        }
    });
}
