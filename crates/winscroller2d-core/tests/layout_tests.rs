use winscroller2d_core::{
    Action, Config, Coord, Direction, FocusCentering, GridWorkspace, IRect, LayoutConfig,
    MonitorInfo, WindowInfo, WindowRule, WorkspaceManager,
};

fn area_1080p() -> IRect {
    IRect::new(0, 0, 1920, 1040)
}

fn find_rect(placements: &[winscroller2d_core::WindowPlacement], id: u64) -> &winscroller2d_core::WindowPlacement {
    placements
        .iter()
        .find(|p| p.window_id == id)
        .expect("placement present")
}

#[test]
fn example_toml_parses_cleanly_and_validates() {
    let cfg = Config::from_toml_str(Config::default_toml_template())
        .expect("winscroller2d.example.toml should parse");
    assert_eq!(cfg.layout.peek_x, 0);
    assert_eq!(cfg.layout.peek_y, 0);
    assert_eq!(cfg.layout.default_width_step, 3);
    assert_eq!(cfg.layout.default_height_step, 3);
    assert!(!cfg.bindings.is_empty());

    assert!(Config::from_toml_str("[layout]\nwidth_steps = []").is_err());
    assert!(Config::from_toml_str("[layout]\nheight_steps = [5.0]").is_err());
}

#[test]
fn hyprscroll2d_placement_peek_and_band_packing() {
    let cfg = LayoutConfig {
        peek_x: 48,
        peek_y: 48,
        gap_x: 12,
        gap_y: 12,
        default_width_step: 1,
        default_height_step: 2,
        auto_expand_focused: false,
        ..Default::default()
    };
    let mut ws = GridWorkspace::new(0, "1");

    // Open 3 windows: placed at (0,0), (1,0), (2,0) with focus following rightmost.
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);
    ws.insert_window(3, false, None, None, &cfg);
    assert_eq!(ws.loc[&1], Coord::new(0, 0));
    assert_eq!(ws.loc[&2], Coord::new(1, 0));
    assert_eq!(ws.loc[&3], Coord::new(2, 0));
    assert_eq!(ws.focused, Some(3));

    // Focus window 2 (middle) and check edge peek math.
    assert_eq!(ws.focus_direction(Direction::Left), Some(2));
    let (placements, _) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        false,
        false,
        0.016,
        280.0,
        0.94,
        0.5,
    );

    let p1 = find_rect(&placements, 1);
    let p2 = find_rect(&placements, 2);
    let p3 = find_rect(&placements, 3);

    // Centered cell (window 2) should be centered in 1920x1040 viewport.
    assert_eq!(p2.rect.center().0, 1920 / 2);
    assert_eq!(p2.rect.center().1, 1040 / 2);
    // Gap between cells equals config.gap_x (within 1px rounding).
    assert!((p2.rect.x - p1.rect.right() - cfg.gap_x).abs() <= 1);
    assert!((p3.rect.x - p2.rect.right() - cfg.gap_x).abs() <= 1);
    assert!(p1.visible && p2.visible && p3.visible);
}

#[test]
fn infinite_2d_vertical_and_horizontal_move_swap_and_holes() {
    let cfg = LayoutConfig::default();
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);

    // Move window 2 down into empty cell (1, 1), then another step down to (1, 2).
    assert!(ws.move_focused(Direction::Down, true));
    assert_eq!(ws.loc[&2], Coord::new(1, 1));
    assert!(ws.move_focused(Direction::Down, true));
    assert_eq!(ws.loc[&2], Coord::new(1, 2));

    // Move window 2 up twice -> returns to (1,0), then left into occupied (0,0) -> swaps 1 & 2!
    assert!(ws.move_focused(Direction::Up, true));
    assert!(ws.move_focused(Direction::Up, true));
    assert!(ws.move_focused(Direction::Left, true));
    assert_eq!(ws.loc[&2], Coord::new(0, 0));
    assert_eq!(ws.loc[&1], Coord::new(1, 0));

    // Move 2 up to (0, -1) (negative infinite vertical coordinate).
    assert!(ws.move_focused(Direction::Up, true));
    assert_eq!(ws.loc[&2], Coord::new(0, -1));

    // Directional focus down from (0, -1) finds (1, 0) even across hole (0, 0).
    assert_eq!(ws.focus_direction(Direction::Down), Some(1));
}

#[test]
fn niri_consume_expel_stack_and_tabbed_cloaking() {
    let cfg = LayoutConfig::default();
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(10, false, None, None, &cfg);
    ws.insert_window(20, false, None, None, &cfg);
    ws.set_focus(10);

    // Consume window 20 from (1,0) into cell (0,0).
    assert!(ws.consume_into_cell(Some(Direction::Right)));
    assert_eq!(ws.loc[&10], Coord::new(0, 0));
    assert_eq!(ws.loc[&20], Coord::new(0, 0));
    assert_eq!(ws.cells.len(), 1);

    let (placements, _) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        false,
        false,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    let p10 = find_rect(&placements, 10);
    let p20 = find_rect(&placements, 20);
    assert!(p10.visible && p20.visible);
    assert!(p20.rect.x > p10.rect.x);

    // Focusing right or down steps within the cell split from 10 to 20.
    assert_eq!(ws.focus_direction(Direction::Down), Some(20));

    // Toggle Tabbed stack mode: only active window (20) is uncloaked.
    assert!(ws.toggle_stack_mode());
    let (tab_placements, _) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        false,
        false,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    assert!(!find_rect(&tab_placements, 10).visible);
    assert!(find_rect(&tab_placements, 20).visible);

    // Expel 20 back out to the right.
    assert!(ws.expel_from_cell(Direction::Right));
    assert_eq!(ws.loc[&10], Coord::new(0, 0));
    assert_eq!(ws.loc[&20], Coord::new(1, 0));
}

#[test]
fn offscreen_windows_cloak_and_cross_monitor_peeks_clip() {
    let cfg = LayoutConfig {
        peek_x: 48,
        peek_y: 48,
        gap_x: 12,
        gap_y: 12,
        default_width_step: 1,
        default_height_step: 2,
        auto_expand_focused: false,
        ..Default::default()
    };
    let mut ws = GridWorkspace::new(0, "1");
    // Create 5 windows across cols 0..=4.
    for id in 1..=5 {
        ws.insert_window(id, false, None, None, &cfg);
    }
    // Focus window 1 at col 0. Window 5 at col 4 is way off-screen to the right.
    ws.set_focus(1);
    let right_monitor = IRect::new(1920, 0, 1920, 1080);
    let (placements, _) = ws.compute_placements(
        area_1080p(),
        &[right_monitor],
        &cfg,
        false,
        false,
        0.016,
        280.0,
        0.94,
        0.5,
    );

    let p1 = find_rect(&placements, 1);
    let p2 = find_rect(&placements, 2);
    let p5 = find_rect(&placements, 5);
    assert!(p1.visible);
    assert!(p2.visible);
    // Window 2 peeks past x=1920 onto right_monitor, so it gets clipped to area_1080p().
    assert_eq!(p2.clip_to, Some(area_1080p()));
    // Window 5 is completely off-screen -> visible = false (should be DWM-cloaked).
    assert!(!p5.visible);
}

#[test]
fn smooth_camera_converges_and_keeps_focused_put_on_offscreen_resize() {
    let cfg = LayoutConfig {
        focus_centering: FocusCentering::Always,
        ..Default::default()
    };
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);
    ws.insert_window(3, false, None, None, &cfg);

    // Snap at col 2 (window 3).
    let _ = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        false,
        false,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    // Pan camera to col 0 with animation enabled.
    assert!(ws.pan_grid(Direction::Left));
    assert!(ws.pan_grid(Direction::Left));
    // Focused window did NOT change during camera pan!
    assert_eq!(ws.focused, Some(3));

    let (mid_placements, still_animating) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        true,
        true,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    assert!(still_animating);
    let mid_x = find_rect(&mid_placements, 1).rect.center().0;
    assert!(mid_x < 1920 / 2);

    // Step frames until animation settles on col 0.
    let mut animating = true;
    for _ in 0..160 {
        let (_, a) = ws.compute_placements(
            area_1080p(),
            &[],
            &cfg,
            true,
            true,
            0.016,
            280.0,
            0.94,
            0.5,
        );
        animating = a;
        if !animating {
            break;
        }
    }
    assert!(!animating);
    let (settled, _) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        true,
        true,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    assert!((find_rect(&settled, 1).rect.center().0 - 1920 / 2).abs() <= 1);
}

#[test]
fn workspace_manager_dynamic_workspaces_and_rules() {
    let mut cfg = Config::default();
    cfg.rules.push(WindowRule {
        exe: Some("CustomFloatApp.exe".into()),
        float: true,
        ..Default::default()
    });
    let mon = MonitorInfo {
        id: 1,
        name: "PRIMARY".into(),
        work_area: area_1080p(),
        bounds: IRect::new(0, 0, 1920, 1080),
    };
    let mut mgr = WorkspaceManager::new(cfg, vec![mon]);

    // Ignored rule window should not be added.
    assert!(!mgr.add_window(WindowInfo {
        id: 99,
        title: "Snipping".into(),
        process_name: "ScreenClippingHost.exe".into(),
        class_name: "CoreWindow".into(),
        rect: IRect::new(0, 0, 500, 500),
        monitor_id: Some(1),
        ..Default::default()
    }));

    // Window matching custom floating rule should float.
    assert!(mgr.add_window(WindowInfo {
        id: 100,
        title: "Floating Tool".into(),
        process_name: "CustomFloatApp.exe".into(),
        class_name: "FloatingWindowClass".into(),
        rect: IRect::new(100, 100, 800, 600),
        monitor_id: Some(1),
        ..Default::default()
    }));

    // Normal tiled app (including Task Manager, following identical tiling pattern).
    assert!(mgr.add_window(WindowInfo {
        id: 101,
        title: "Task Manager".into(),
        process_name: "Taskmgr.exe".into(),
        class_name: "TaskManagerWindow".into(),
        rect: IRect::new(0, 0, 800, 600),
        monitor_id: Some(1),
        ..Default::default()
    }));

    let snap = mgr.snapshot();
    assert_eq!(snap.workspaces[0].window_count, 2);
    assert!(snap.workspaces[0]
        .windows
        .iter()
        .any(|w| w.id == 100 && w.floating));
    assert!(snap.workspaces[0]
        .windows
        .iter()
        .any(|w| w.id == 101 && !w.floating));

    // Switching workspace cloaks workspace 0 windows in frame plan.
    mgr.dispatch(Action::SwitchWorkspace { index: 1 });
    let plan = mgr.compute_frame(0.016);
    assert!(plan.commands.iter().all(|c| !c.visible));
}

#[test]
fn preserve_holes_false_collapses_empty_column() {
    let cfg = LayoutConfig::default();
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);
    // Window 1 is at (0,0) and Window 2 is at (1,0).
    // Moving Window 1 Left to (-1,0) empties column 0; with preserve_holes = false,
    // column 0 collapses and pulls Window 2 from (1,0) to (0,0).
    ws.set_focus(1);
    assert!(ws.move_focused(Direction::Left, false));
    assert_eq!(ws.loc[&1], Coord::new(-1, 0));
    assert_eq!(ws.loc[&2], Coord::new(0, 0));
}

#[test]
fn auto_expand_focused_grows_active_and_shrinks_previous_on_switch() {
    let cfg = LayoutConfig {
        peek_x: 48,
        peek_y: 48,
        gap_x: 12,
        gap_y: 12,
        default_width_step: 1,
        default_height_step: 2,
        auto_expand_focused: true,
        focused_peek_x: 24,
        focused_peek_y: 24,
        ..Default::default()
    };
    assert!(cfg.auto_expand_focused);

    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);
    // Window 2 is currently focused -> should fill almost entire 1920x1040 screen (minus focused_peek).
    let (p_init, _) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        false,
        false,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    let r1 = find_rect(&p_init, 1).rect;
    let r2 = find_rect(&p_init, 2).rect;

    let expected_expanded_w = 1920 - cfg.focused_peek_x * 2;
    let expected_expanded_h = 1040 - cfg.focused_peek_y * 2;
    assert_eq!(r2.w, expected_expanded_w);
    assert_eq!(r2.h, expected_expanded_h);
    // Unfocused window 1 stays at compact preset width (< expanded).
    assert!(r1.w < r2.w);
    assert!(r1.h < r2.h);
    // Still leaves a visible edge peek of window 1 on the left!
    assert!(find_rect(&p_init, 1).visible);

    // Move focus Left to window 1 with smooth animation enabled:
    // window 1 grows to full expanded size and window 2 shrinks back to preset!
    assert_eq!(ws.focus_direction(Direction::Left), Some(1));
    let (mid, still_anim) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        true,
        true,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    assert!(still_anim);
    let mid_1 = find_rect(&mid, 1).rect;
    let mid_2 = find_rect(&mid, 2).rect;
    assert!(mid_1.w > r1.w && mid_1.w < expected_expanded_w);
    assert!(mid_2.w < r2.w && mid_2.w > r1.w);
    // Ensure zero overlap during animated resize + scroll!
    assert!(mid_2.x >= mid_1.right() + cfg.gap_x - 1);

    // Let animation settle.
    for _ in 0..200 {
        let (_, anim) = ws.compute_placements(
            area_1080p(),
            &[],
            &cfg,
            true,
            true,
            0.016,
            280.0,
            0.94,
            0.5,
        );
        if !anim {
            break;
        }
    }
    let (done, anim_done) = ws.compute_placements(
        area_1080p(),
        &[],
        &cfg,
        true,
        true,
        0.016,
        280.0,
        0.94,
        0.5,
    );
    assert!(!anim_done);
    let final_1 = find_rect(&done, 1).rect;
    let final_2 = find_rect(&done, 2).rect;
    assert!((final_1.w - expected_expanded_w).abs() <= 1);
    assert!((final_1.h - expected_expanded_h).abs() <= 1);
    assert!((final_2.w - r1.w).abs() <= 1);
    assert!((final_2.h - r1.h).abs() <= 1);
}

#[test]
fn place_window_at_supports_drag_move_swap_and_stack() {
    let cfg = LayoutConfig::default();
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);
    ws.insert_window(3, false, None, None, &cfg);

    // Drag window 3 from (2,0) into empty cell (0, 2) preserving holes.
    assert!(ws.place_window_at(3, Coord::new(0, 2), false, true, &cfg));
    assert_eq!(ws.loc[&3], Coord::new(0, 2));
    assert_eq!(ws.loc[&1], Coord::new(0, 0));

    // Drag window 1 onto occupied cell (1, 0) without stack -> swaps 1 & 2!
    assert!(ws.place_window_at(1, Coord::new(1, 0), false, true, &cfg));
    assert_eq!(ws.loc[&1], Coord::new(1, 0));
    assert_eq!(ws.loc[&2], Coord::new(0, 0));

    // Shift+Drag window 3 onto occupied cell (1, 0) with stack=true -> consumes into cell!
    assert!(ws.place_window_at(3, Coord::new(1, 0), true, true, &cfg));
    assert_eq!(ws.loc[&3], Coord::new(1, 0));
    assert_eq!(ws.cells[&Coord::new(1, 0)].windows, vec![1, 3]);
}

#[test]
fn drag_drop_split_window_horizontal_creates_side_by_side_halves_and_unpacks() {
    let cfg = LayoutConfig::default();
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(101, false, None, None, &cfg);
    ws.insert_window(102, false, None, None, &cfg);

    // Window 101 is at (0, 0), Window 102 is at (1, 0)
    assert_eq!(ws.loc[&101], Coord::new(0, 0));
    assert_eq!(ws.loc[&102], Coord::new(1, 0));

    // Drag Window 102 onto Window 101 on the right half -> snaps horizontally into one view!
    assert!(ws.split_window_horizontal(102, 101, false, &cfg));
    assert_eq!(ws.loc[&101], Coord::new(0, 0));
    assert_eq!(ws.loc[&102], Coord::new(0, 0));
    assert_eq!(ws.cells[&Coord::new(0, 0)].windows, vec![101, 102]);

    // Verify placements combine to 100% width and full height side-by-side in one view
    let area = IRect::new(0, 0, 1920, 1080);
    let (placements, _) = ws.compute_placements(area, &[], &cfg, false, false, 0.016, 260.0, 0.85, 0.5);
    let p101 = placements.iter().find(|p| p.window_id == 101).unwrap();
    let p102 = placements.iter().find(|p| p.window_id == 102).unwrap();
    assert!(p101.visible && p102.visible);
    // They are side by side: p101 is on left, p102 is on right
    assert!(p101.rect.x < p102.rect.x);
    assert_eq!(p101.rect.h, p102.rect.h);
    // Widths are each ~50% of the viewport (not squashed vertically!)
    assert!(p101.rect.w >= 850 && p102.rect.w >= 850);
    assert_eq!(p101.rect.h, 1080);
    assert_eq!(p102.rect.h, 1080);
}

#[test]
fn underdamped_default_spring_overshoots_and_settles() {
    let cfg = Config::default();
    assert!(cfg.animation.damping_ratio < 1.0);

    let mut spring = winscroller2d_core::Spring1D::default();
    spring.snap_to(0.0);
    spring.set_target(1.0, true);

    let mut max_val = 0.0_f64;
    let mut settled = false;
    for _ in 0..240 {
        let moving = spring.step(
            1.0 / 60.0,
            cfg.animation.stiffness,
            cfg.animation.damping_ratio,
            0.0005,
        );
        max_val = max_val.max(spring.current);
        if !moving {
            settled = true;
            break;
        }
    }

    assert!(max_val > 1.01, "expected underdamped spring overshoot bounce, max={max_val}");
    assert!(settled, "expected spring to settle cleanly");
    assert!((spring.current - 1.0).abs() <= 0.001);
}

#[test]
fn neighbor_4way_placement_and_row_col_compaction() {
    let mut cfg = LayoutConfig::default();
    cfg.new_window_placement = winscroller2d_core::NewWindowPlacement::Neighbor4Way;
    cfg.auto_compact_empty = true;

    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg); // (0,0)
    assert_eq!(ws.loc[&1], Coord::new(0, 0));

    // Keep focus pinned on win 1 and verify right -> down -> left -> up filling.
    ws.set_focus(1);
    ws.insert_window(2, false, None, None, &cfg); // right -> (1,0)
    assert_eq!(ws.loc[&2], Coord::new(1, 0));

    ws.set_focus(1);
    ws.insert_window(3, false, None, None, &cfg); // down -> (0,1)
    assert_eq!(ws.loc[&3], Coord::new(0, 1));

    ws.set_focus(1);
    ws.insert_window(4, false, None, None, &cfg); // left -> (-1,0)
    assert_eq!(ws.loc[&4], Coord::new(-1, 0));

    ws.set_focus(1);
    ws.insert_window(5, false, None, None, &cfg); // up -> (0,-1)
    assert_eq!(ws.loc[&5], Coord::new(0, -1));

    // All 4 neighbors around win 1 are occupied: inserts right of win 1 and shifts old (1,0) -> (2,0).
    ws.set_focus(1);
    ws.insert_window(6, false, None, None, &cfg);
    assert_eq!(ws.loc[&6], Coord::new(1, 0));
    assert_eq!(ws.loc[&2], Coord::new(2, 0));

    // Removing the only window at row -1 compacts row -1 away? Check row compaction above:
    // row -1 removal shifts rows > -1 up (-1) so (0, 0) becomes (0, -1) or closes gap between row -1 and 0.
    // Removing win 4 at col -1 compacts col >= 0 left by 1 so win 1 moves from col 0 -> col -1.
    assert!(ws.remove_window_with_compact(4, true));
    assert_eq!(ws.loc[&1], Coord::new(-1, 0));
}

#[test]
fn continuous_pan_commit_snaps_back_under_threshold_or_focuses_neighbor() {
    let cfg = LayoutConfig::default();
    let mut ws = GridWorkspace::new(0, "1");
    ws.insert_window(1, false, None, None, &cfg);
    ws.insert_window(2, false, None, None, &cfg);
    ws.set_focus(1);
    let _ = ws.compute_placements(area_1080p(), &[], &cfg, false, false, 0.016, 260.0, 0.72, 0.5);

    // Small pan < 8px snaps cleanly back to win 1.
    ws.pan_pixels(4.0, 2.0);
    assert_eq!(ws.finish_pixel_pan(0.0, 0.0), None);
    assert_eq!(ws.focused, Some(1));

    // Substantial rightward pan commits focus to win 2.
    ws.pan_pixels(420.0, 10.0);
    let _ = ws.compute_placements(area_1080p(), &[], &cfg, false, false, 0.016, 260.0, 0.72, 0.5);
    assert_eq!(ws.finish_pixel_pan(200.0, 0.0), Some(2));
    assert_eq!(ws.focused, Some(2));
}

#[test]
fn persisted_state_restores_coordinates_sizes_and_workspace_layout_toggle() {
    let mon = MonitorInfo {
        id: 1,
        name: "DISPLAY1".into(),
        work_area: area_1080p(),
        bounds: IRect::new(0, 0, 1920, 1080),
    };
    let cfg = Config::default();
    let mut mgr = WorkspaceManager::new(cfg.clone(), vec![mon.clone()]);

    mgr.add_window(WindowInfo {
        id: 10,
        title: "Terminal".into(),
        process_name: "wt.exe".into(),
        class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        ..Default::default()
    });
    mgr.add_window(WindowInfo {
        id: 20,
        title: "Editor".into(),
        process_name: "code.exe".into(),
        class_name: "Chrome_WidgetWin_1".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        ..Default::default()
    });
    mgr.dispatch(Action::MoveWindow { direction: Direction::Down });
    mgr.dispatch(Action::ResizeWidth { delta: -1 });
    mgr.dispatch(Action::ToggleWorkspaceLayout);
    assert!(mgr.active_workspace_layout_disabled());

    let exported = mgr.export_persisted_state();
    let json = exported.to_json_pretty().expect("serialize persisted state");
    let decoded = winscroller2d_core::PersistedState::from_json_str(&json).expect("deserialize");

    let mut mgr2 = WorkspaceManager::new(cfg, vec![mon]);
    mgr2.import_persisted_state(decoded);
    assert!(mgr2.active_workspace_layout_disabled());

    // Reopen windows with brand-new HWNDs (ids 101, 202) and verify exact grid restore.
    mgr2.add_window(WindowInfo {
        id: 101,
        title: "Terminal".into(),
        process_name: "wt.exe".into(),
        class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        ..Default::default()
    });
    mgr2.add_window(WindowInfo {
        id: 202,
        title: "Editor".into(),
        process_name: "code.exe".into(),
        class_name: "Chrome_WidgetWin_1".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        ..Default::default()
    });
    let ws = mgr2.active_workspace_snapshot().expect("ws snapshot");
    let w_editor = ws.windows.iter().find(|w| w.id == 202).expect("editor restored");
    assert_eq!((w_editor.col, w_editor.row), (1, 1));
    assert_eq!(w_editor.width_step, 2);

    // Toggle back to 2D lattice and enable resize overview fallback test.
    mgr2.dispatch(Action::ToggleWorkspaceLayout);
    assert!(!mgr2.active_workspace_layout_disabled());
    let resize_rects = {
        let mut g = GridWorkspace::new(0, "1");
        g.insert_window(1, false, None, None, &LayoutConfig::default());
        g.insert_window(2, false, None, None, &LayoutConfig::default());
        g.compute_overview_resize_placements(area_1080p(), &LayoutConfig::default())
    };
    assert_eq!(resize_rects.len(), 2);
    assert_ne!(resize_rects[0].rect, resize_rects[1].rect);
}

#[test]
fn per_desktop_per_monitor_registers_independent_scroll_instances() {
    let mon1 = MonitorInfo {
        id: 1,
        name: "DISPLAY1".into(),
        work_area: IRect::new(0, 0, 1920, 1080),
        bounds: IRect::new(0, 0, 1920, 1080),
    };
    let mon2 = MonitorInfo {
        id: 2,
        name: "DISPLAY2".into(),
        work_area: IRect::new(1920, 0, 1920, 1080),
        bounds: IRect::new(1920, 0, 1920, 1080),
    };

    let mut cfg = Config::default();
    cfg.workspaces.names = vec!["1".into()];
    let mut mgr = WorkspaceManager::new(cfg, vec![mon1.clone(), mon2.clone()]);
    mgr.sync_os_desktops(vec!["desk-a".into()], "desk-a".into());

    // Desktop A registers independent scroll instances for Mon 1 and Mon 2.
    mgr.add_window(WindowInfo {
        id: 1001,
        title: "A-M1-1".into(),
        process_name: "wt.exe".into(),
        class_name: "C".into(),
        rect: IRect::new(0, 0, 800, 600),
        monitor_id: Some(1),
        desktop_id: Some("desk-a".into()),
    });
    mgr.add_window(WindowInfo {
        id: 1002,
        title: "A-M2-1".into(),
        process_name: "code.exe".into(),
        class_name: "C".into(),
        rect: IRect::new(2000, 0, 800, 600),
        monitor_id: Some(2),
        desktop_id: Some("desk-a".into()),
    });

    // Creating Desktop B registers brand-new scroll instances on both Mon 1 and Mon 2.
    assert!(mgr.sync_os_desktops(
        vec!["desk-a".into(), "desk-b".into()],
        "desk-b".into(),
    ));
    let active_ws_b = mgr.active_workspace_snapshot().expect("desk-b ws");
    assert_eq!(active_ws_b.desktop_id, "desk-b");
    assert_eq!(active_ws_b.window_count, 0);

    mgr.add_window(WindowInfo {
        id: 2001,
        title: "B-M1-1".into(),
        process_name: "chrome.exe".into(),
        class_name: "C".into(),
        rect: IRect::new(0, 0, 800, 600),
        monitor_id: Some(1),
        desktop_id: Some("desk-b".into()),
    });

    // Frame on Desktop B must only command Desktop B windows (never Desktop A).
    let plan_b = mgr.compute_frame(0.016);
    assert!(plan_b.commands.iter().any(|c| c.window_id == 2001));
    assert!(!plan_b.commands.iter().any(|c| c.window_id == 1001 || c.window_id == 1002));

    // Adding a 3rd monitor registers instances for BOTH desk-a and desk-b.
    let mon3 = MonitorInfo {
        id: 3,
        name: "DISPLAY3".into(),
        work_area: IRect::new(3840, 0, 1920, 1080),
        bounds: IRect::new(3840, 0, 1920, 1080),
    };
    mgr.sync_monitors(vec![mon1, mon2, mon3]);
    let snap = mgr.snapshot();
    assert!(snap
        .workspaces
        .iter()
        .any(|w| w.desktop_id == "desk-a" && w.monitor_id == Some(3)));
    assert!(snap
        .workspaces
        .iter()
        .any(|w| w.desktop_id == "desk-b" && w.monitor_id == Some(3)));

    // Switch back to Desktop A: Desktop A instances and focus remain intact; Desktop B untouched.
    assert!(mgr.sync_os_desktops(
        vec!["desk-a".into(), "desk-b".into()],
        "desk-a".into(),
    ));
    let plan_a = mgr.compute_frame(0.016);
    assert!(plan_a.commands.iter().any(|c| c.window_id == 1001));
    assert!(plan_a.commands.iter().any(|c| c.window_id == 1002));
    assert!(!plan_a.commands.iter().any(|c| c.window_id == 2001));

    // Migrating a window across OS desktops moves it cleanly to Desktop B on the same monitor.
    assert!(mgr.move_window_to_os_desktop(1001, "desk-b"));
    assert_eq!(mgr.window_desktop_id(1001), Some("desk-b"));
}

#[test]
fn snapped_window_sizes_persist_across_sibling_snaps_overview_and_restart() {
    let mut cfg = Config::default();
    cfg.layout.auto_expand_focused = true;
    cfg.layout.expand_single_window = true;
    cfg.layout.persist_state = true;
    cfg.animation.enabled = false;

    let mon = MonitorInfo {
        id: 1,
        name: "DISP1".into(),
        work_area: area_1080p(),
        bounds: area_1080p(),
    };
    let mut mgr = WorkspaceManager::new(cfg.clone(), vec![mon.clone()]);

    mgr.add_window(WindowInfo {
        id: 101,
        title: "Editor".into(),
        process_name: "code.exe".into(),
        class_name: "Chrome_WidgetWin_1".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        desktop_id: None,
    });
    mgr.add_window(WindowInfo {
        id: 102,
        title: "Terminal".into(),
        process_name: "wt.exe".into(),
        class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        desktop_id: None,
    });

    // Snap window 101 to custom half width (0.50) and full height (1.0), marking state dirty.
    assert!(mgr.record_window_snap(101, 0.50, 1.0));
    assert!(mgr.state_dirty);

    // Snap window 102 to custom 0.35 width x 0.75 height -> must NOT overwrite window 101's snap!
    assert!(mgr.record_window_snap(102, 0.35, 0.75));

    // Toggle Win+O overview open and close via JumpToCell -> both snaps must remain intact!
    mgr.dispatch(Action::ToggleOverview);
    mgr.dispatch(Action::JumpToCell {
        col: 1,
        row: 0,
        window_id: Some(102),
    });

    let plan = mgr.compute_frame(0.016);
    let r101 = plan
        .commands
        .iter()
        .find(|c| c.window_id == 101)
        .and_then(|c| c.rect)
        .expect("101 rect");
    let r102 = plan
        .commands
        .iter()
        .find(|c| c.window_id == 102)
        .and_then(|c| c.rect)
        .expect("102 rect");
    let (max_w, max_h) = mgr.window_max_cell_size(101).unwrap();
    assert!((r101.w as f64 - max_w * 0.50).abs() <= 2.0);
    assert!((r101.h as f64 - max_h * 1.00).abs() <= 2.0);
    assert!((r102.w as f64 - max_w * 0.35).abs() <= 2.0);
    assert!((r102.h as f64 - max_h * 0.75).abs() <= 2.0);

    // Export and restore into a brand new manager instance: snapped fractions & overrides survive!
    let json = mgr
        .export_persisted_state()
        .to_json_pretty()
        .expect("serialize");
    let decoded = winscroller2d_core::PersistedState::from_json_str(&json).expect("deserialize");
    let mut mgr2 = WorkspaceManager::new(cfg, vec![mon]);
    mgr2.import_persisted_state(decoded);
    mgr2.add_window(WindowInfo {
        id: 201,
        title: "Editor".into(),
        process_name: "code.exe".into(),
        class_name: "Chrome_WidgetWin_1".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        desktop_id: None,
    });
    mgr2.add_window(WindowInfo {
        id: 202,
        title: "Terminal".into(),
        process_name: "wt.exe".into(),
        class_name: "CASCADIA_HOSTING_WINDOW_CLASS".into(),
        rect: area_1080p(),
        monitor_id: Some(1),
        desktop_id: None,
    });

    let plan2 = mgr2.compute_frame(0.016);
    let r201 = plan2
        .commands
        .iter()
        .find(|c| c.window_id == 201)
        .and_then(|c| c.rect)
        .expect("201 rect");
    let r202 = plan2
        .commands
        .iter()
        .find(|c| c.window_id == 202)
        .and_then(|c| c.rect)
        .expect("202 rect");
    assert_eq!((r201.w, r201.h), (r101.w, r101.h));
    assert_eq!((r202.w, r202.h), (r102.w, r102.h));
}
