현재 프로그램의 플롯 범례(Legend) 구현 원리와, 향후 범례 기능의 수정·확장을 원할 때 LLM에게 효과적으로 지시하는 방법 및 프롬프트 작성
  가이드입니다.
  ──────
  ### 1. 현재 플롯 범례의 구현 방식 및 동작 원리

  본 프로젝트는 Rust의 GUI 프레임워크인 egui 및 공식 차트 라이브러리인 egui_plot을 사용하고 있습니다.

  #### ① 범례 컨테이너 등록 (app.rs:2051-2062)

  플롯 위젯을 선언할 때 .legend(...) 메서드로 범례를 활성화합니다:

    let plot = Plot::new("waveform_plot")
        .width(plot_w)
        .height(plot_h)
        .legend(egui_plot::Legend::default().position(egui_plot::Corner::RightTop)) // 우측 상단 고정
        .show_axes([true, self.y_axis_grouping == YAxisGroupingMode::SingleAxis])
        .show_grid([self.show_grid, self.show_grid])
        .x_axis_label("Time (s)");

  • 위치 (Corner): Corner::RightTop(우측 상단)으로 지정되어 플롯 내부 우측 상단에 반투명 박스 형태로 렌더링됩니다. (LeftTop, RightBottom,
  LeftBottom 지원)
  • 내장 기능: egui_plot::Legend는 기본적으로 항목 클릭 시 해당 곡선 숨김/표시(Toggle) 및 마우스 오버 시 곡선 강조(Highlight) 기능을 자체
  지원합니다.

  #### ② 범례 항목 자동 생성 (src/app.rs:2225-L2280)

  플롯 내부의 범례는 별도의 리스트를 등록하는 것이 아니라, plot_ui.line(...)으로 라인을 그릴 때 전달하는 **.name(label)**과 .color(color)
  속성을 읽어 자동으로 생성됩니다:

    // 라벨 문자열 조합 (신호명 + 단위 + 필터 상태)
    let label = if filter_active {
        format!("{} [{}] [{}]", ch.name, ch.unit, filter_str) // 예: TCK-000 [K] [LP 30.0Hz]
    } else {
        format!("{} [{}]", ch.name, ch.unit)                   // 예: PT1201 [MPaA]
    };

    // 라인 등록 -> 범례에 해당 색상과 이름이 자동 등록됨
    plot_ui.line(
        Line::new(PlotPoints::new(points))
            .color(color)
            .name(label)
            .width(stroke_width)
    );

  #### ③ 사이드바 "채널 목록"과의 역할 분담

  • 플롯 내부 범례 (RightTop): 현재 화면에 그려진 신호의 색상과 이름을 확인하고 개별 곡선을 토글하는 미니멀 오버레이 범례 역할을 합니다.
  • 좌측 사이드바 (채널 목록): 전체 채널 검색, 색상 배지, 마우스 커서 위치의 실시간 측정값(Live Value), 채널 선택 체크박스를 제공하는
  대화형 종합 범례 역할을 분담하고 있습니다.
  ──────
  ### 2. LLM에게 어떻게 지시해야 할까? (프롬프트 작성 가이드)

  LLM에게 플롯 범례 관련 수정·개선 작업을 요청할 때는 다음 4가지 핵심 정보를 명시해주면 오류 없이 한 번에 정확한 코드를 작성합니다:

  1. 대상 라이브러리 및 파일 명시: egui_plot 0.29, src/app.rs의 waveform_plot 영역
  2. 구체적인 변경 목표: 범례 위치 변경, 표시/숨김 스위치, 텍스트 포맷, 커스텀 오버레이 등
  3. 상태(State) 연동 여부: 사용자 설정(ViewerSettings) 저장 여부, 체크박스/콤보박스 연동 여부
  4. 기존 기능 보존: 멀티 Y축 오버레이, 필터 라벨 표기, 마우스 제어 등이 깨지지 않도록 요청
  ──────
  ### 3. 상황별 추천 프롬프트 템플릿

  #### 📌 상황 1: 범례 위치 변경 또는 ON/OFF 설정 추가를 원할 때

    src/app.rs의 waveform_plot 플롯 범례(Legend)에 대해 다음 기능을 추가해줘:
    1. 툴바 또는 설정에 "범례 표시 (Show Legend)" 체크박스를 추가해서 범례를 켜고 끌 수 있게 해줘.
    2. 범례 위치를 선택할 수 있는 콤보박스(우측상단, 좌측상단, 우측하단, 좌측하단)를 추가해줘 (egui_plot::Corner 연동).
    3. 이 설정값들이 ViewerSettings에 저장되어 재실행 시에도 유지되게 해줘.

  #### 📌 상황 2: 범례 텍스트에 실시간 값(Live Value) 또는 통계값 표기를 원할 때

    src/app.rs에서 플롯 범례(Legend)의 라벨 표시 방식을 수정해줘:
    1. 현재 커서가 플롯 위에 있을 때(hover_frame_idx가 Some일 때), 범례 라벨에 신호명뿐만 아니라 현재 커서 값도 함께 표시해줘. (예:
    "PT1201: 0.1728 MPaA")
    2. 커서가 플롯 밖에 있을 때는 기존처럼 "PT1201 [MPaA]"로 표시해줘.

  #### 📌 상황 3: 범례 배경 투명도, 폰트 크기, 여백 조정

    egui_plot::Legend의 스타일을 다듬어줘:
    1. 다크모드와 라이트모드에 어울리도록 범례 배경과 테두리를 깔끔하게 맞춰줘.
    2. 범례 텍스트가 너무 길어 그래프를 가리지 않도록 최대 너비 제한이나 글자 크기를 설정해줘.

  #### 📌 상황 4: 플롯 내부가 아닌 별도의 커스텀 범례 창이나 패널을 원할 때

    egui_plot의 기본 범례 대신 플롯 우측 상단에 직접 반투명 UI Window/Area로 커스텀 범례를 띄워줘:
    - 각 채널의 색상 원(Badge), 채널명, 단위, 현재값, 그리고 화면에서 숨길 수 있는 눈 아이콘 버튼을 포함해줘.
    - 드래그해서 위치를 옮길 수 있게 egui::Window 또는 Area로 구현해줘.