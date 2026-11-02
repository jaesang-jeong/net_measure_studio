use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const TOTAL_PAGES: usize = 10;
pub const CONFIG_FILE_NAME: &str = "layout_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDisplayConfig {
    pub auto_scale_y: bool,
    pub custom_y_min: f64,
    pub custom_y_max: f64,
    pub x_window_secs: f64,                      // X축 시간 길이 (초 단위, 예: 10.0초)
    pub multi_y_overlay: bool,                   // 다중 Y축 오버레이 모드 (채널별 스케일 정규화)
    pub hidden_channels: Vec<usize>,             // 감춘 채널 인덱스 목록
    pub channel_y_limits: HashMap<usize, (f64, f64)>, // 채널별 사용자 지정 (Y-min, Y-max)
    #[serde(skip)]
    pub hovered_channel: Option<usize>,          // 플롯 범례 마우스 호버 강조 채널
}

impl Default for GraphDisplayConfig {
    fn default() -> Self {
        Self {
            auto_scale_y: true,
            custom_y_min: -15.0,
            custom_y_max: 15.0,
            x_window_secs: 10.0,
            multi_y_overlay: false,
            hidden_channels: Vec::new(),
            channel_y_limits: HashMap::new(),
            hovered_channel: None,
        }
    }
}

impl GraphDisplayConfig {
    pub fn is_channel_hidden(&self, ch_idx: usize) -> bool {
        self.hidden_channels.contains(&ch_idx)
    }

    pub fn toggle_channel_visibility(&mut self, ch_idx: usize) {
        if let Some(pos) = self.hidden_channels.iter().position(|&x| x == ch_idx) {
            self.hidden_channels.remove(pos);
        } else {
            self.hidden_channels.push(ch_idx);
        }
    }

    pub fn get_channel_y_limit(&self, ch_idx: usize, default_min: f64, default_max: f64) -> (f64, f64) {
        if let Some(&(min, max)) = self.channel_y_limits.get(&ch_idx) {
            if min < max {
                return (min, max);
            }
        }
        (default_min, default_max)
    }

    pub fn set_channel_y_limit(&mut self, ch_idx: usize, min: f64, max: f64) {
        if min < max {
            self.channel_y_limits.insert(ch_idx, (min, max));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayoutConfig {
    pub page_index: usize,
    pub title: String,
    pub top_channels: Vec<usize>,     // Page 1: 상단 그래프 채널, Page 2~10: 단일 그래프 채널
    pub bottom_channels: Vec<usize>,  // Page 1: 하단 서브 그래프 채널
    #[serde(default)]
    pub top_graph_config: GraphDisplayConfig,
    #[serde(default)]
    pub bottom_graph_config: GraphDisplayConfig,
}

impl PageLayoutConfig {
    /// 메인 페이지(Page 1, index 0)만 2그래프 구성, 다음 페이지(Page 2~10)부터는 1그래프 구성
    pub fn is_dual_plot(&self) -> bool {
        self.page_index == 0
    }

    pub fn default_pages() -> Vec<Self> {
        let mut pages = Vec::with_capacity(TOTAL_PAGES);
        for i in 0..TOTAL_PAGES {
            if i == 0 {
                // Page 1: 메인 페이지 - 2그래프 구성 (상단 전압, 하단 전류)
                let mut top_cfg = GraphDisplayConfig::default();
                top_cfg.x_window_secs = 10.0;
                top_cfg.custom_y_min = -12.0;
                top_cfg.custom_y_max = 12.0;

                let mut bot_cfg = GraphDisplayConfig::default();
                bot_cfg.x_window_secs = 10.0;
                bot_cfg.custom_y_min = -1.0;
                bot_cfg.custom_y_max = 20.0;

                pages.push(Self {
                    page_index: 0,
                    title: "Page 1: 메인 종합 모니터링".to_string(),
                    top_channels: vec![0, 1],       // CH 1, CH 2
                    bottom_channels: vec![2, 3],    // CH 3, CH 4
                    top_graph_config: top_cfg,
                    bottom_graph_config: bot_cfg,
                });
            } else {
                // Page 2~10: 1그래프 구성
                let base_ch = (i * 10) % 100;
                let title = match i {
                //    1 => "Page 2: 온도/열 모니터링 (1 그래프)".to_string(),
                //    2 => "Page 3: 압력/유압 모니터링 (1 그래프)".to_string(),
                //    3 => "Page 4: 모터 RPM 및 동적 특성 (1 그래프)".to_string(),
                    _ => format!("Page {}: 상세 단일 그래프", i + 1),
                };

                let mut single_cfg = GraphDisplayConfig::default();
                single_cfg.x_window_secs = 15.0;

                pages.push(Self {
                    page_index: i,
                    title,
                    top_channels: vec![base_ch, (base_ch + 1) % 100, (base_ch + 2) % 100, (base_ch + 3) % 100],
                    bottom_channels: Vec::new(),
                    top_graph_config: single_cfg,
                    bottom_graph_config: GraphDisplayConfig::default(),
                });
            }
        }
        pages
    }

    /// 파일로부터 10개 페이지 레이아웃 로드, 없으면 기본값 반환
    pub fn load_or_default(path: &Path) -> Vec<Self> {
        if path.exists() {
            if let Ok(mut file) = File::open(path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(pages) = serde_json::from_str::<Vec<Self>>(&content) {
                        if pages.len() == TOTAL_PAGES {
                            return pages;
                        }
                    }
                }
            }
        }
        Self::default_pages()
    }

    /// 10개 페이지 레이아웃 설정을 파일로 영구 저장
    pub fn save_to_file(path: &Path, pages: &[Self]) -> std::io::Result<()> {
        let json_str = serde_json::to_string_pretty(pages)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut file = File::create(path)?;
        file.write_all(json_str.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}
