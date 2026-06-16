//! 색상 테마. 전체 UI 색상·여백·라운드를 결정한다.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    /// 카탈로그형 라이트(기본): 흰 배경 + 옅은 구분선 + 부드러운 라운드.
    #[default]
    Catalog,
    Dark,
    Light,
    PostmanOrange,
    Ocean,
}

impl Theme {
    pub const ALL: [Theme; 5] = [
        Theme::Catalog,
        Theme::Dark,
        Theme::Light,
        Theme::PostmanOrange,
        Theme::Ocean,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Catalog => "카탈로그",
            Theme::Dark => "다크",
            Theme::Light => "라이트",
            Theme::PostmanOrange => "Postman 오렌지",
            Theme::Ocean => "오션",
        }
    }

    /// 테마를 컨텍스트에 적용 (색상 + 라운드 + 여백).
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.global_style()).clone();
        style.visuals = self.visuals();

        // 공통: 부드러운 라운드 + 약간 넉넉한 여백 (카탈로그 룩).
        let r = egui::CornerRadius::same(6);
        let w = &mut style.visuals.widgets;
        for ws in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            ws.corner_radius = r;
        }
        style.visuals.window_corner_radius = r;
        style.visuals.menu_corner_radius = r;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);

        ctx.set_global_style(style);
    }

    /// 이 테마의 `egui::Visuals`.
    pub fn visuals(&self) -> egui::Visuals {
        match self {
            Theme::Catalog => catalog_visuals(),
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
            // Postman 시그니처 오렌지(#FF6C37)를 강조색으로.
            Theme::PostmanOrange => {
                accented(egui::Visuals::dark(), egui::Color32::from_rgb(255, 108, 55))
            }
            Theme::Ocean => accented(egui::Visuals::dark(), egui::Color32::from_rgb(64, 169, 220)),
        }
    }
}

/// 레퍼런스(API 카탈로그)풍 라이트 테마.
fn catalog_visuals() -> egui::Visuals {
    use egui::Color32 as C;
    let mut v = egui::Visuals::light();

    // 배경
    v.panel_fill = C::from_rgb(255, 255, 255);
    v.window_fill = C::from_rgb(255, 255, 255);
    v.faint_bg_color = C::from_rgb(247, 248, 250); // 줄무늬/약한 배경
    v.extreme_bg_color = C::from_rgb(246, 247, 249); // 입력란 배경
    v.code_bg_color = C::from_rgb(244, 245, 247); // 코드/응답 본문

    // 선택(행 하이라이트) — 옅은 블루그레이
    v.selection.bg_fill = C::from_rgb(233, 238, 246);
    v.selection.stroke = egui::Stroke::new(1.0, C::from_rgb(201, 214, 229));

    // 링크/포커스 강조
    v.hyperlink_color = C::from_rgb(37, 99, 235);

    // 구분선/테두리 (옅은 회색)
    let border = C::from_rgb(236, 237, 239);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, border);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, C::from_rgb(55, 65, 81));

    // 버튼 등 위젯
    v.widgets.inactive.bg_fill = C::from_rgb(242, 243, 245);
    v.widgets.inactive.weak_bg_fill = C::from_rgb(242, 243, 245);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, C::from_rgb(31, 41, 55));
    v.widgets.hovered.bg_fill = C::from_rgb(233, 235, 238);
    v.widgets.hovered.weak_bg_fill = C::from_rgb(233, 235, 238);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, C::from_rgb(209, 213, 219));
    v.widgets.active.bg_fill = C::from_rgb(224, 227, 231);
    v.widgets.active.weak_bg_fill = C::from_rgb(224, 227, 231);

    v
}

fn with_alpha(c: egui::Color32, a: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// 기본 Visuals에 강조색(accent)을 입혀 선택·호버·링크 등을 통일한다.
fn accented(mut v: egui::Visuals, accent: egui::Color32) -> egui::Visuals {
    v.selection.bg_fill = with_alpha(accent, 90);
    v.selection.stroke = egui::Stroke::new(1.0, accent);
    v.hyperlink_color = accent;

    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.4, accent);
    v.widgets.active.bg_fill = with_alpha(accent, 70);
    v.widgets.active.weak_bg_fill = with_alpha(accent, 70);
    v.widgets.hovered.weak_bg_fill = with_alpha(accent, 40);

    v
}

/// 상태 코드에 대응하는 pill 배지 색상 (배경, 글자).
pub fn status_pill_colors(status: u16) -> (egui::Color32, egui::Color32) {
    use egui::Color32 as C;
    match status {
        200..=299 => (C::from_rgb(214, 245, 221), C::from_rgb(31, 122, 61)), // 초록 (Passing)
        300..=399 => (C::from_rgb(219, 234, 254), C::from_rgb(30, 64, 175)), // 파랑
        400..=499 => (C::from_rgb(254, 243, 199), C::from_rgb(146, 100, 12)), // 주황
        _ => (C::from_rgb(252, 228, 228), C::from_rgb(185, 49, 49)),         // 빨강 (Failing)
    }
}
