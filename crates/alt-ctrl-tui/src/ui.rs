use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use sidecar_core::{
    AdapterConnectionState, AgentKind, AgentView, NavigationScreen, PermissionProfile, SessionState,
};

use crate::model::{PlanItemState, SessionView, TuiState};

const BG: Color = Color::Rgb(8, 11, 16);
const PANEL: Color = Color::Rgb(16, 21, 29);
const LINE: Color = Color::Rgb(48, 59, 71);
const TEXT: Color = Color::Rgb(235, 241, 243);
const MUTED: Color = Color::Rgb(137, 153, 166);
const CYAN: Color = Color::Rgb(98, 227, 207);
const AMBER: Color = Color::Rgb(255, 203, 107);
const RED: Color = Color::Rgb(255, 114, 124);
const GREEN: Color = Color::Rgb(131, 228, 142);

pub fn render(frame: &mut Frame<'_>, state: &TuiState) {
    frame.render_widget(Block::new().style(Style::default().bg(BG)), frame.area());
    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, shell[0], state);
    match &state.navigation.screen {
        NavigationScreen::MissionControl => render_mission(frame, shell[1], state),
        NavigationScreen::Agent { session_id, view } => {
            render_agent(frame, shell[1], state, session_id.as_str(), *view);
        }
        NavigationScreen::GlobalEmergency => render_emergency(frame, shell[1], state),
        NavigationScreen::CommandPalette => render_placeholder(
            frame,
            shell[1],
            "COMMAND PALETTE",
            "Structured commands arrive in the next TUI slice.",
        ),
        NavigationScreen::TaskComposer { .. } => render_placeholder(
            frame,
            shell[1],
            "TASK COMPOSER",
            "Build a structured instruction, then preview it before sending.",
        ),
        NavigationScreen::RawTerminal { session_id } => {
            render_raw_terminal(frame, shell[1], state, session_id.as_str());
        }
    }
    render_footer(frame, shell[2], state);

    if state.approval_session.is_some() {
        render_approval(frame, state);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let left = Line::from(vec![
        Span::styled(
            " ALT ",
            Style::default()
                .fg(BG)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " CTRL ",
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  LOCAL / ALT-CTRL", Style::default().fg(MUTED)),
    ]);
    let right = Line::from(vec![
        Span::styled("INTERACTIVE FIXTURE", Style::default().fg(AMBER)),
        Span::raw(format!("   {} events ", state.audit_count)),
    ])
    .alignment(Alignment::Right);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);
    frame.render_widget(
        Paragraph::new(left).block(Block::new().borders(Borders::BOTTOM).border_style(LINE)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(right)
            .style(Style::default().fg(MUTED))
            .block(Block::new().borders(Borders::BOTTOM).border_style(LINE)),
        columns[1],
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let controls = if state.approval_session.is_some() {
        "[Enter] approve once   [r] reject   [Esc] keep pending"
    } else {
        match state.navigation.screen {
            NavigationScreen::MissionControl => {
                "[arrows] focus  [Enter] open  [s] inspect  [q/e] agent  [g] emergency  [x] quit"
            }
            NavigationScreen::Agent { .. } => {
                "[z/c] view  [s] inspect  [i] interrupt  [m] mission control  [Esc] back  [x] quit"
            }
            NavigationScreen::GlobalEmergency if !state.global_stop_committed => {
                "[Enter] CONFIRM GLOBAL STOP   [Esc] cancel"
            }
            NavigationScreen::GlobalEmergency => "[Esc] return to previous screen   [x] quit",
            NavigationScreen::CommandPalette
            | NavigationScreen::TaskComposer { .. }
            | NavigationScreen::RawTerminal { .. } => "[Esc] back   [m] mission control   [x] quit",
        }
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" STATUS  ", Style::default().fg(CYAN)),
            Span::styled(&state.notice, Style::default().fg(MUTED)),
        ]))
        .block(Block::new().borders(Borders::TOP).border_style(LINE)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(controls)
            .alignment(Alignment::Center)
            .style(Style::default().fg(TEXT)),
        rows[1],
    );
}

fn render_mission(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(8)])
        .split(area);
    let counts = mission_counts(state);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                " MISSION CONTROL",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    " Supervise every agent. Intervene only when it matters.  ",
                    MUTED,
                ),
                Span::styled(format!("{} running  ", counts.0), CYAN),
                Span::styled(format!("{} action required  ", counts.1), AMBER),
                Span::styled(format!("{} failed", counts.2), RED),
            ]),
        ]),
        sections[0],
    );

    if sections[1].width >= 106 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(68), Constraint::Length(35)])
            .split(sections[1]);
        render_session_grid(frame, columns[0], state);
        render_attention(frame, columns[1], state);
    } else {
        render_session_grid(frame, sections[1], state);
    }
}

fn render_session_grid(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    for (row_index, row) in rows.iter().enumerate() {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*row);
        for (column_index, card) in columns.iter().enumerate() {
            let index = row_index * 2 + column_index;
            if let Some(session) = state.sessions.get(index) {
                render_session_card(frame, *card, session, index == state.focus_index);
            }
        }
    }
}

fn render_session_card(frame: &mut Frame<'_>, area: Rect, session: &SessionView, selected: bool) {
    let state_color = session_state_color(session.state);
    let border_color = if selected { CYAN } else { LINE };
    let title = if selected {
        format!(
            " SELECTED / {} / {} ",
            session.name,
            session_state_label(session.state)
        )
    } else {
        format!(
            " {} / {} ",
            session.name,
            session_state_label(session.state)
        )
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(agent_label(&session.agent), Style::default().fg(CYAN)),
            Span::styled(
                format!("  {}  ", connection_label(session.connection)),
                Style::default().fg(MUTED),
            ),
            Span::styled(
                session_state_label(session.state),
                Style::default()
                    .fg(state_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(&session.objective, Style::default().fg(TEXT))),
        Line::raw(""),
        Line::from(vec![
            Span::styled("NOW  ", Style::default().fg(CYAN)),
            Span::styled(&session.current_tool, Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("BRANCH  ", Style::default().fg(MUTED)),
            Span::raw(&session.branch),
        ]),
        Line::from(vec![
            Span::styled("GIT     ", Style::default().fg(MUTED)),
            Span::raw(format!(
                "{} files / {}",
                session.changed_files, session.git_summary
            )),
        ]),
        Line::from(vec![
            Span::styled("TESTS   ", Style::default().fg(MUTED)),
            Span::styled(&session.test_summary, Style::default().fg(state_color)),
        ]),
        Line::from(vec![
            Span::styled("POLICY  ", Style::default().fg(MUTED)),
            Span::raw(permission_label(session.permission_profile)),
            Span::styled(
                format!("     {}", session.elapsed),
                Style::default().fg(MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("CONTEXT ", Style::default().fg(MUTED)),
            Span::styled(
                context_bar(session.context_percent),
                Style::default().fg(CYAN),
            ),
            Span::raw(format!(" {}%", session.context_percent)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(TEXT).bg(PANEL))
            .block(
                Block::new()
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(if selected { CYAN } else { state_color })
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color)),
            ),
        inset(area, 0, 1),
    );
}

fn render_attention(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let mut lines = vec![
        Line::from(Span::styled(
            "NEEDS ATTENTION",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];
    for session in state
        .sessions
        .iter()
        .filter(|session| session.attention_count() > 0)
    {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", session.attention_count()),
                Style::default()
                    .fg(BG)
                    .bg(session_state_color(session.state)),
            ),
            Span::styled(format!(" {}", session.name), Style::default().fg(TEXT)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("    {}", attention_summary(session)),
            Style::default().fg(MUTED),
        )));
        lines.push(Line::raw(""));
    }
    lines.extend([
        Line::raw(""),
        Line::from(Span::styled("SYSTEM", Style::default().fg(CYAN))),
        Line::from("  [OK] daemon"),
        Line::from(format!("  [OK] event store / {}", state.audit_count)),
        Line::from("  [--] adapters / 3 of 4"),
        Line::raw(""),
        Line::from(Span::styled(
            "FIXTURE ONLY",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "No commands or processes are executed.",
            Style::default().fg(MUTED),
        )),
    ]);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::new()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(LINE)),
        ),
        inset(area, 0, 1),
    );
}

fn render_agent(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    session_id: &str,
    view: AgentView,
) {
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.id.as_str() == session_id)
    else {
        render_placeholder(frame, area, "SESSION MISSING", session_id);
        return;
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" AGENT VIEW / {} ", session.name),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    " {} / {} / {} ",
                    agent_label(&session.agent),
                    session.branch,
                    session.elapsed
                ),
                Style::default().fg(MUTED),
            ),
            Span::styled(
                session_state_label(session.state),
                Style::default().fg(session_state_color(session.state)),
            ),
        ])),
        sections[0],
    );
    render_view_tabs(frame, sections[1], view, session);

    match view {
        AgentView::Output | AgentView::Plan => {
            let direction = if sections[2].width >= 96 {
                Direction::Horizontal
            } else {
                Direction::Vertical
            };
            let panels = Layout::default()
                .direction(direction)
                .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
                .split(sections[2]);
            if view == AgentView::Output {
                render_output(frame, panels[0], session);
                render_plan(frame, panels[1], session);
            } else {
                render_plan(frame, panels[0], session);
                render_output(frame, panels[1], session);
            }
        }
        AgentView::Diff => render_detail_view(
            frame,
            sections[2],
            "DIFF VIEW",
            &format!(
                "{} changed files / {}",
                session.changed_files, session.git_summary
            ),
            "File and hunk navigation will bind to the repository observer.",
        ),
        AgentView::Tests => render_detail_view(
            frame,
            sections[2],
            "TEST VIEW",
            &session.test_summary,
            &session.current_tool,
        ),
        AgentView::Approvals => render_detail_view(
            frame,
            sections[2],
            "APPROVAL VIEW",
            if session.pending_action.is_some() {
                "1 exact action pending"
            } else {
                "No pending approvals"
            },
            permission_label(session.permission_profile),
        ),
    }
}

fn render_view_tabs(frame: &mut Frame<'_>, area: Rect, active: AgentView, session: &SessionView) {
    let views = [
        (AgentView::Output, "OUTPUT"),
        (AgentView::Plan, "PLAN"),
        (AgentView::Diff, "DIFF"),
        (AgentView::Tests, "TESTS"),
        (AgentView::Approvals, "APPROVALS"),
    ];
    let mut spans = vec![Span::raw(" ")];
    for (view, label) in views {
        let label = if view == AgentView::Approvals && session.pending_action.is_some() {
            format!(" {label} 1 ")
        } else {
            format!(" {label} ")
        };
        let style = if view == active {
            Style::default()
                .fg(BG)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .block(Block::new().borders(Borders::BOTTOM).border_style(LINE)),
        area,
    );
}

fn render_output(frame: &mut Frame<'_>, area: Rect, session: &SessionView) {
    let mut lines = vec![Line::from(vec![
        Span::styled("LIVE ", Style::default().fg(CYAN)),
        Span::styled(&session.worktree, Style::default().fg(MUTED)),
    ])];
    lines.push(Line::raw(""));
    for (index, output) in session.output.iter().enumerate() {
        let color = if output.starts_with("FAIL") {
            RED
        } else if output.starts_with("OK") {
            GREEN
        } else if output.starts_with("WAIT") {
            AMBER
        } else {
            TEXT
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:02}  ", index + 1), Style::default().fg(LINE)),
            Span::styled(output, Style::default().fg(color)),
        ]));
    }
    lines.push(Line::from(Span::styled(
        " > _",
        Style::default().fg(CYAN).add_modifier(Modifier::SLOW_BLINK),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" LIVE OUTPUT ", CYAN)),
        inset(area, 0, 1),
    );
}

fn render_plan(frame: &mut Frame<'_>, area: Rect, session: &SessionView) {
    let mut lines = vec![
        Line::from(Span::styled(&session.objective, Style::default().fg(TEXT))),
        Line::raw(""),
    ];
    for item in &session.plan {
        let (marker, color) = match item.state {
            PlanItemState::Done => ("[x]", GREEN),
            PlanItemState::Active => ("[>]", CYAN),
            PlanItemState::Queued => ("[ ]", MUTED),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), Style::default().fg(color)),
            Span::styled(&item.label, Style::default().fg(TEXT)),
        ]));
    }
    lines.extend([
        Line::raw(""),
        Line::from(vec![
            Span::styled("REPOSITORY  ", Style::default().fg(MUTED)),
            Span::raw(&session.repository),
        ]),
        Line::from(vec![
            Span::styled("POLICY      ", Style::default().fg(MUTED)),
            Span::raw(permission_label(session.permission_profile)),
        ]),
        Line::from(vec![
            Span::styled("CONTEXT     ", Style::default().fg(MUTED)),
            Span::styled(
                context_bar(session.context_percent),
                Style::default().fg(CYAN),
            ),
        ]),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" EXECUTION PLAN ", CYAN)),
        inset(area, 0, 1),
    );
}

fn render_detail_view(frame: &mut Frame<'_>, area: Rect, title: &str, value: &str, hint: &str) {
    let panel_title = format!(" {title} ");
    let block = panel(&panel_title, CYAN);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let center = centered(inner, 72.min(inner.width), 7.min(inner.height));
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                value,
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(hint, Style::default().fg(MUTED))),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        center,
    );
}

fn render_raw_terminal(frame: &mut Frame<'_>, area: Rect, state: &TuiState, session_id: &str) {
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.id.as_str() == session_id)
    else {
        render_placeholder(frame, area, "RAW TERMINAL", "Session unavailable");
        return;
    };
    let lines = session
        .output
        .iter()
        .map(|line| Line::from(format!("> {line}")))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(TEXT))
            .block(panel(" RAW TERMINAL / FIXTURE ", AMBER))
            .wrap(Wrap { trim: false }),
        inset(area, 1, 1),
    );
}

fn render_emergency(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let interrupted = state
        .sessions
        .iter()
        .filter(|session| session.state == SessionState::Interrupted)
        .count();
    let (title, body, color) = if state.global_stop_committed {
        (
            "ALL ACTIVE FIXTURE AGENTS STOPPED",
            format!(
                "{interrupted} sessions interrupted. Worktrees, output, and audit history preserved."
            ),
            GREEN,
        )
    } else {
        (
            "GLOBAL EMERGENCY",
            "This demonstration requires a separate confirmation. The production controller service emits StopAllAgents only after its validated hold."
                .to_owned(),
            RED,
        )
    };
    let target = centered(area, 84.min(area.width), 15.min(area.height));
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                " ALT CTRL / SAFETY ",
                Style::default().fg(BG).bg(color),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                title,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(body, Style::default().fg(TEXT))),
            Line::raw(""),
            Line::from(Span::styled(
                if state.global_stop_committed {
                    "[OK] mutations blocked  [OK] interrupts dispatched  [OK] history preserved"
                } else {
                    "Press Enter to confirm the fixture stop, or Esc to cancel."
                },
                Style::default().fg(if state.global_stop_committed {
                    GREEN
                } else {
                    AMBER
                }),
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(panel(" EMERGENCY OVERVIEW ", color)),
        target,
    );
}

fn render_approval(frame: &mut Frame<'_>, state: &TuiState) {
    let Some((session, pending)) = state.approval() else {
        return;
    };
    let area = centered(
        frame.area(),
        88.min(frame.area().width),
        19.min(frame.area().height),
    );
    frame.render_widget(Clear, area);
    let action = &pending.request.action;
    let lines = vec![
        Line::from(vec![
            Span::styled("APPROVAL REQUIRED  ", Style::default().fg(AMBER)),
            Span::styled(&session.name, Style::default().fg(TEXT)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("ACTION  ", Style::default().fg(MUTED)),
            Span::raw(&action.operation),
        ]),
        Line::from(vec![
            Span::styled("TARGET  ", Style::default().fg(MUTED)),
            Span::styled(&action.resolved_target, Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("RISK    ", Style::default().fg(MUTED)),
            Span::styled(format!("{:?}", action.risk), Style::default().fg(AMBER)),
        ]),
        Line::from(vec![
            Span::styled("SCOPE   ", Style::default().fg(MUTED)),
            Span::raw(format!("{:?}", pending.requested_scope)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            pending
                .expected_side_effects
                .first()
                .map_or("No side-effect description.", String::as_str),
            Style::default().fg(MUTED),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" [Enter] APPROVE ONCE ", Style::default().fg(BG).bg(CYAN)),
            Span::styled("  [r] REJECT  ", Style::default().fg(RED)),
            Span::styled("  [Esc] KEEP PENDING", Style::default().fg(MUTED)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "FIXTURE ONLY / no command will run",
            Style::default().fg(AMBER),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" EXACT ACTION / MEDIUM RISK ", AMBER)),
        area,
    );
}

fn render_placeholder(frame: &mut Frame<'_>, area: Rect, title: &str, message: &str) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                title,
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(message, Style::default().fg(MUTED))),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(panel(&format!(" {title} "), CYAN)),
        inset(area, 2, 1),
    );
}

fn panel(title: &str, color: Color) -> Block<'_> {
    Block::new()
        .title(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(LINE))
        .style(Style::default().bg(PANEL))
}

fn mission_counts(state: &TuiState) -> (usize, usize, usize) {
    let running = state
        .sessions
        .iter()
        .filter(|session| session.state == SessionState::Running)
        .count();
    let action_required = state
        .sessions
        .iter()
        .filter(|session| {
            matches!(
                session.state,
                SessionState::WaitingForApproval
                    | SessionState::WaitingForAnswer
                    | SessionState::Blocked
            )
        })
        .count();
    let failed = state
        .sessions
        .iter()
        .filter(|session| matches!(session.state, SessionState::Failing | SessionState::Failed))
        .count();
    (running, action_required, failed)
}

fn attention_summary(session: &SessionView) -> &str {
    if session.pending_action.is_some() {
        "exact action approval"
    } else if session.state == SessionState::Failed {
        &session.test_summary
    } else {
        session_state_label(session.state)
    }
}

fn session_state_label(state: SessionState) -> &'static str {
    match state {
        SessionState::Created => "CREATED",
        SessionState::Starting => "STARTING",
        SessionState::Running => "RUNNING",
        SessionState::WaitingForApproval => "APPROVAL",
        SessionState::WaitingForAnswer => "ANSWER",
        SessionState::Blocked => "BLOCKED",
        SessionState::Interrupting => "INTERRUPTING",
        SessionState::Interrupted => "INTERRUPTED",
        SessionState::Resuming => "RESUMING",
        SessionState::Failing => "FAILING",
        SessionState::Failed => "FAILED",
        SessionState::Terminating => "TERMINATING",
        SessionState::Terminated => "TERMINATED",
        SessionState::Completed => "COMPLETE",
    }
}

const fn session_state_color(state: SessionState) -> Color {
    match state {
        SessionState::WaitingForApproval
        | SessionState::WaitingForAnswer
        | SessionState::Blocked => AMBER,
        SessionState::Failing | SessionState::Failed => RED,
        SessionState::Completed => GREEN,
        SessionState::Running | SessionState::Starting | SessionState::Resuming => CYAN,
        SessionState::Created
        | SessionState::Interrupting
        | SessionState::Interrupted
        | SessionState::Terminating
        | SessionState::Terminated => MUTED,
    }
}

fn connection_label(state: AdapterConnectionState) -> &'static str {
    match state {
        AdapterConnectionState::Unknown => "unknown",
        AdapterConnectionState::Connecting => "connecting",
        AdapterConnectionState::Connected => "connected",
        AdapterConnectionState::Reconnecting => "reconnecting",
        AdapterConnectionState::Disconnected => "disconnected",
        AdapterConnectionState::Incompatible => "incompatible",
    }
}

fn agent_label(agent: &AgentKind) -> &str {
    match agent {
        AgentKind::Codex => "CODEX",
        AgentKind::ClaudeCode => "CLAUDE CODE",
        AgentKind::GenericPty => "GENERIC PTY",
        AgentKind::Other(name) => name,
    }
}

const fn permission_label(profile: PermissionProfile) -> &'static str {
    match profile {
        PermissionProfile::Observe => "OBSERVE",
        PermissionProfile::Edit => "EDIT",
        PermissionProfile::Build => "BUILD",
        PermissionProfile::Elevated => "ELEVATED",
    }
}

fn context_bar(percent: u8) -> String {
    let filled = usize::from(percent.min(100) / 10);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(10 - filled))
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use sidecar_core::UiAction;

    use crate::{fixtures::fixture_state, model::Command, snapshot_from_terminal};

    use super::render;

    #[test]
    fn mission_control_snapshot_contains_ranked_sessions_and_safety_label() {
        let state = fixture_state();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        terminal.draw(|frame| render(frame, &state)).expect("draw");

        let snapshot = snapshot_from_terminal(&terminal);
        assert!(snapshot.contains("MISSION CONTROL"));
        assert!(snapshot.contains("API Sentinel"));
        assert!(snapshot.contains("Test Pilot"));
        assert!(snapshot.contains("FIXTURE ONLY"));
    }

    #[test]
    fn agent_and_approval_views_render_contract_data() {
        let mut state = fixture_state();
        state.apply(&Command::Semantic(UiAction::Confirm));
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &state))
            .expect("draw agent");
        let agent = snapshot_from_terminal(&terminal);
        assert!(agent.contains("AGENT VIEW / API Sentinel"));
        assert!(agent.contains("LIVE OUTPUT"));
        assert!(agent.contains("EXECUTION PLAN"));

        state.apply(&Command::Semantic(UiAction::Inspect));
        terminal
            .draw(|frame| render(frame, &state))
            .expect("draw approval");
        let approval = snapshot_from_terminal(&terminal);
        assert!(approval.contains("APPROVAL REQUIRED"));
        assert!(approval.contains("cargo test -p auth-core"));
        assert!(approval.contains("APPROVE ONCE"));
    }

    #[test]
    fn narrow_terminal_render_does_not_panic() {
        let state = fixture_state();
        let mut terminal = Terminal::new(TestBackend::new(72, 28)).expect("test terminal");
        terminal.draw(|frame| render(frame, &state)).expect("draw");
        assert!(snapshot_from_terminal(&terminal).contains("MISSION CONTROL"));
    }
}
