//! Postwoman 메인 애플리케이션 (egui UI).

use std::sync::mpsc::{channel, Receiver, Sender};

use crate::http::{self, FetchResult, KeyValue, Method, RequestSpec, ResponseData};
use crate::persist::{self, AppState};
use crate::theme::Theme;

/// 요청 편집 영역의 하위 탭.
#[derive(PartialEq, Eq, Clone, Copy)]
enum RequestTab {
    Params,
    Headers,
    Body,
}

/// 응답 표시 영역의 하위 탭.
#[derive(PartialEq, Eq, Clone, Copy)]
enum ResponseTab {
    Body,
    Headers,
}

/// 현재 요청 상태.
enum Status {
    Idle,
    Loading,
    Done(ResponseData),
    Failed(String),
}

pub struct PostwomanApp {
    // 요청 입력
    method: Method,
    url: String,
    headers: Vec<KeyValue>,
    query: Vec<KeyValue>,
    body: String,

    // UI 상태
    request_tab: RequestTab,
    response_tab: ResponseTab,
    pretty: bool,
    /// 복사 버튼을 누른 시각(egui time, 초). "복사됨" 토스트 표시에 사용.
    copied_at: Option<f64>,

    // 영속 상태 (디스크 저장)
    theme: Theme,
    /// 최근 사용한 URL (가장 최근이 맨 앞).
    url_history: Vec<String>,

    // 실행 상태
    status: Status,
    tx: Sender<FetchResult>,
    rx: Receiver<FetchResult>,
}

impl Default for PostwomanApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            method: Method::Get,
            url: "https://httpbin.org/get".to_owned(),
            headers: vec![KeyValue::new()],
            query: vec![KeyValue::new()],
            body: String::new(),
            request_tab: RequestTab::Params,
            response_tab: ResponseTab::Body,
            pretty: true,
            copied_at: None,
            theme: Theme::default(),
            url_history: Vec::new(),
            status: Status::Idle,
            tx,
            rx,
        }
    }
}

impl PostwomanApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // D2Coding을 기본 폰트로 설치 (한글 + 코딩 글꼴).
        install_font(&cc.egui_ctx);
        // 저장된 테마/이력 복원.
        let state = persist::load();
        state.theme.apply(&cc.egui_ctx);
        Self {
            theme: state.theme,
            url_history: state.url_history,
            ..Default::default()
        }
    }

    /// 현재 테마/이력을 디스크에 저장.
    fn persist(&self) {
        persist::save(&AppState {
            theme: self.theme,
            url_history: self.url_history.clone(),
        });
    }

    /// 전송한 URL을 이력 맨 앞에 기록 (중복 제거, 최대 50개).
    fn record_history(&mut self) {
        let url = self.url.trim().to_string();
        if url.is_empty() {
            return;
        }
        self.url_history.retain(|u| u != &url);
        self.url_history.insert(0, url);
        self.url_history.truncate(50);
        self.persist();
    }

    fn send_request(&mut self, ctx: &egui::Context) {
        if self.url.trim().is_empty() {
            self.status = Status::Failed("URL을 입력하세요.".to_owned());
            return;
        }
        self.record_history();
        let spec = RequestSpec {
            method: self.method,
            url: self.url.clone(),
            headers: self.headers.clone(),
            query: self.query.clone(),
            body: self.body.clone(),
        };
        self.status = Status::Loading;
        http::execute(spec, self.tx.clone(), ctx.clone());
    }

    fn poll_result(&mut self) {
        if let Ok(result) = self.rx.try_recv() {
            self.status = match result {
                FetchResult::Ok(data) => Status::Done(data),
                FetchResult::Err(e) => Status::Failed(e),
            };
        }
    }

    /// URL 입력란 아래에 이력 자동완성 드롭다운을 띄운다.
    /// 입력값으로 필터링하며(최근 항목이 위), 항목을 고르면 `self.url`을 채우고 true.
    fn url_autocomplete(&mut self, url_resp: &egui::Response) -> bool {
        if self.url_history.is_empty() {
            return false;
        }
        let q = self.url.trim().to_lowercase();
        let matches: Vec<String> = self
            .url_history
            .iter()
            .filter(|u| q.is_empty() || u.to_lowercase().contains(&q))
            .filter(|u| u.as_str() != self.url) // 현재 입력과 동일한 항목은 제외
            .take(10)
            .cloned()
            .collect();

        let open = url_resp.has_focus() && !matches.is_empty();
        let mut picked: Option<String> = None;

        egui::Popup::from_response(url_resp)
            .open(open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(2.0)
            .show(|ui| {
                ui.set_min_width(url_resp.rect.width());
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for m in &matches {
                            if ui.selectable_label(false, m).clicked() {
                                picked = Some(m.clone());
                            }
                        }
                    });
            });

        if let Some(p) = picked {
            self.url = p;
            true
        } else {
            false
        }
    }
}

impl eframe::App for PostwomanApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_result();
        // 선택된 테마 적용 (다음 프레임에 반영).
        self.theme.apply(ui.ctx());

        self.menu_bar(ui);
        self.top_bar(ui);
        self.request_panel(ui);
        self.response_panel(ui);
    }
}

impl PostwomanApp {
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Postwoman").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut changed = false;
                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(self.theme.name())
                        .show_ui(ui, |ui| {
                            for t in Theme::ALL {
                                changed |= ui
                                    .selectable_value(&mut self.theme, t, t.name())
                                    .changed();
                            }
                        });
                    ui.label("🎨 테마:");
                    if changed {
                        self.persist();
                    }
                });
            });
            ui.add_space(2.0);
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("top_bar").show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // 메서드 선택
                egui::ComboBox::from_id_salt("method")
                    .selected_text(self.method.as_str())
                    .width(95.0)
                    .show_ui(ui, |ui| {
                        for m in Method::ALL {
                            ui.selectable_value(&mut self.method, m, m.as_str());
                        }
                    });

                // URL 입력 (남는 공간 채우되 Send 버튼 자리 확보)
                let send_clicked = ui
                    .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let send = ui
                            .add_sized([80.0, 24.0], egui::Button::new("Send"))
                            .clicked();
                        let url_resp = ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::TextEdit::singleline(&mut self.url)
                                .hint_text("https://api.example.com/endpoint"),
                        );

                        // 이력 자동완성 드롭다운 (항목 선택 시 입력란만 채움).
                        self.url_autocomplete(&url_resp);

                        let enter = url_resp.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        send || enter
                    })
                    .inner;

                if send_clicked {
                    let ctx = ui.ctx().clone();
                    self.send_request(&ctx);
                }
            });
            ui.add_space(4.0);
        });
    }

    fn request_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("request_panel")
            .resizable(true)
            .default_size(220.0)
            .min_size(120.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.request_tab, RequestTab::Params, "Params");
                    ui.selectable_value(&mut self.request_tab, RequestTab::Headers, "Headers");
                    let body_label = if self.method.allows_body() {
                        "Body"
                    } else {
                        "Body (미사용)"
                    };
                    ui.selectable_value(&mut self.request_tab, RequestTab::Body, body_label);
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.request_tab {
                        RequestTab::Params => key_value_editor(ui, &mut self.query, "param_grid"),
                        RequestTab::Headers => {
                            key_value_editor(ui, &mut self.headers, "header_grid")
                        }
                        RequestTab::Body => {
                            if !self.method.allows_body() {
                                ui.colored_label(
                                    egui::Color32::GRAY,
                                    format!("{} 요청은 보통 바디를 보내지 않습니다.", self.method.as_str()),
                                );
                                ui.add_space(4.0);
                            }
                            ui.label("Raw (JSON 등)");
                            ui.add_sized(
                                [ui.available_width(), ui.available_height().max(80.0)],
                                egui::TextEdit::multiline(&mut self.body)
                                    .code_editor()
                                    .hint_text("{\n  \"key\": \"value\"\n}"),
                            );
                        }
                    });
            });
    }

    fn response_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add_space(4.0);
            match &self.status {
                Status::Idle => {
                    ui.centered_and_justified(|ui| {
                        ui.label("요청을 보내면 여기에 응답이 표시됩니다.");
                    });
                }
                Status::Loading => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("요청 중...");
                    });
                }
                Status::Failed(err) => {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), "요청 실패");
                    ui.separator();
                    ui.label(err);
                }
                Status::Done(data) => {
                    response_meta_bar(ui, data);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.response_tab, ResponseTab::Body, "Body");
                        ui.selectable_value(
                            &mut self.response_tab,
                            ResponseTab::Headers,
                            format!("Headers ({})", data.headers.len()),
                        );
                        if self.response_tab == ResponseTab::Body && data.pretty_body.is_some() {
                            ui.separator();
                            ui.checkbox(&mut self.pretty, "Pretty (JSON)");
                        }
                    });
                    ui.separator();

                    match self.response_tab {
                        ResponseTab::Body => {
                            let shown = match (&data.pretty_body, self.pretty) {
                                (Some(p), true) => p,
                                _ => &data.body,
                            };
                            // 본문 영역(스크롤 영역)의 사각형을 미리 잡아 우상단에
                            // 복사 버튼을 오버레이한다.
                            let area_rect = ui.available_rect_before_wrap();

                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let mut text = shown.as_str();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut text)
                                            .code_editor()
                                            .desired_width(f32::INFINITY),
                                    );
                                });

                            // 우상단 복사 버튼 오버레이 (스크롤 영역 위에 떠 있음).
                            if copy_overlay(ui, area_rect, self.copied_at) {
                                ui.ctx().copy_text(shown.to_owned());
                                self.copied_at = Some(ui.input(|i| i.time));
                            }
                        }
                        ResponseTab::Headers => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    egui::Grid::new("resp_headers")
                                        .num_columns(2)
                                        .striped(true)
                                        .spacing([12.0, 4.0])
                                        .show(ui, |ui| {
                                            for (k, v) in &data.headers {
                                                ui.strong(k);
                                                ui.label(v);
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    }
                }
            }
        });
    }
}

/// 상태/소요시간/크기 요약 바.
fn response_meta_bar(ui: &mut egui::Ui, data: &ResponseData) {
    ui.horizontal(|ui| {
        let label = if data.status_text.is_empty() {
            data.status.to_string()
        } else {
            format!("{} {}", data.status, data.status_text)
        };
        status_pill(ui, &label, data.status);
        ui.separator();
        ui.weak(format!("⏱ {} ms", data.elapsed_ms));
        ui.separator();
        ui.weak(format!("📦 {}", human_size(data.size_bytes)));
    });
}

/// 상태 코드를 둥근 pill 배지로 그린다 (스크린샷의 Passing/Failing 스타일).
fn status_pill(ui: &mut egui::Ui, label: &str, status: u16) {
    let (fill, text_color) = crate::theme::status_pill_colors(status);
    let font = egui::FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, text_color);
    let pad = egui::vec2(10.0, 4.0);
    let (rect, _) = ui.allocate_exact_size(galley.size() + pad * 2.0, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, rect.height() * 0.5, fill); // 좌우 완전 둥근 pill
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, text_color);
}

fn human_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// 결과 영역(`area`)의 우상단에 복사 버튼을 오버레이로 그린다.
/// 아이콘은 폰트 글리프 대신 두 개의 겹친 둥근 사각형을 직접 그려 렌더링을 보장한다.
/// 클릭되면 `true`를 반환한다. `copied_at`이 최근이면 "복사됨" 토스트를 페이드로 표시.
fn copy_overlay(ui: &mut egui::Ui, area: egui::Rect, copied_at: Option<f64>) -> bool {
    let size = egui::vec2(26.0, 26.0);
    let margin = 8.0;
    let rect = egui::Rect::from_min_size(
        egui::pos2(area.right() - size.x - margin, area.top() + margin),
        size,
    );

    let resp = ui.interact(rect, ui.id().with("copy_overlay_btn"), egui::Sense::click());

    // 상태별 색상 (Copy 값으로 추출해 이후 painter 借用과 충돌 방지).
    let (bg, border) = {
        let v = ui.visuals();
        if resp.is_pointer_button_down_on() {
            (v.widgets.active.bg_fill, v.widgets.active.bg_stroke)
        } else if resp.hovered() {
            (v.widgets.hovered.bg_fill, v.widgets.hovered.bg_stroke)
        } else {
            (v.widgets.inactive.bg_fill, v.widgets.inactive.bg_stroke)
        }
    };
    let icon_color = ui.visuals().text_color();

    let painter = ui.painter().clone();
    painter.rect(rect, 5.0, bg, border, egui::StrokeKind::Inside);

    // 겹친 두 사각형 = 복사 아이콘.
    let stroke = egui::Stroke::new(1.6, icon_color);
    let c = rect.center();
    let s = 9.0_f32; // 사각형 한 변
    let off = 2.4_f32; // 겹침 오프셋
    let back = egui::Rect::from_min_size(
        egui::pos2(c.x - s / 2.0 + off, c.y - s / 2.0 - off),
        egui::vec2(s, s),
    );
    let front = egui::Rect::from_min_size(
        egui::pos2(c.x - s / 2.0 - off, c.y - s / 2.0 + off),
        egui::vec2(s, s),
    );
    painter.rect_stroke(back, 2.0, stroke, egui::StrokeKind::Middle);
    painter.rect_filled(front, 2.0, bg); // 앞 사각형이 뒤를 가리도록 배경색으로 채움
    painter.rect_stroke(front, 2.0, stroke, egui::StrokeKind::Middle);

    // "복사됨" 토스트 (1.2초간 페이드 아웃).
    if let Some(t) = copied_at {
        let age = ui.input(|i| i.time) - t;
        if (0.0..1.2).contains(&age) {
            let alpha = ((1.0 - age / 1.2) * 255.0) as u8;
            painter.text(
                egui::pos2(rect.left() - 8.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                "복사됨",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgba_unmultiplied(120, 200, 120, alpha),
            );
            ui.ctx().request_repaint();
        }
    }

    let clicked = resp.clicked();
    resp.on_hover_text("복사");
    clicked
}

/// 키-값 편집 그리드 (헤더/파라미터 공용). 마지막 행은 항상 빈 행으로 유지.
fn key_value_editor(ui: &mut egui::Ui, items: &mut Vec<KeyValue>, id: &str) {
    let mut to_remove: Option<usize> = None;

    egui::Grid::new(id)
        .num_columns(4)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (i, kv) in items.iter_mut().enumerate() {
                ui.checkbox(&mut kv.enabled, "");
                ui.add(
                    egui::TextEdit::singleline(&mut kv.key)
                        .hint_text("Key")
                        .desired_width(180.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut kv.value)
                        .hint_text("Value")
                        .desired_width(240.0),
                );
                if ui.small_button("🗑").clicked() {
                    to_remove = Some(i);
                }
                ui.end_row();
            }
        });

    if let Some(i) = to_remove {
        items.remove(i);
    }

    // 항상 끝에 빈 입력 행이 하나 있도록 유지.
    if items.last().map(|kv| !kv.key.is_empty() || !kv.value.is_empty()).unwrap_or(true) {
        items.push(KeyValue::new());
    }
}

/// D2Coding 폰트를 바이너리에 내장해 기본 글꼴로 등록한다.
/// 한글 + ASCII + 코딩 리거처를 모두 커버하므로 Proportional/Monospace 양쪽의
/// 최우선 폰트로 지정한다. (시스템에 D2Coding 미설치여도 동작)
fn install_font(ctx: &egui::Context) {
    const D2CODING: &[u8] = include_bytes!("../assets/D2Coding.ttf");

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "d2coding".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(D2CODING)),
    );

    // 두 패밀리 모두 D2Coding을 맨 앞에 둬 기본 폰트로 사용.
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "d2coding".to_owned());
    }

    ctx.set_fonts(fonts);
}
