//! Pure full-frame shell rendering and render-authorized hit geometry.

use ratatui_core::text::Line;
use ratatui_core::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Style,
    terminal::Frame,
};
use termrock::{
    interaction::HitRegion,
    widgets::{
        Action, ActionBar, ActionBarState, CompletionCandidate, CompletionMenu, CompletionMenuSize,
        CompletionMenuState, Form, FormField, FormSection, FormState, GridCell, GridColumn,
        GridRow, Panel, PanelEmphasis, StatusBar, StatusBarState, StatusSlot, Tab, Tabs, TabsState,
        TextArea, TextAreaState, Tree, TreeNode, TreeNodeStatus, TreeState, VirtualGrid,
        VirtualGridState, render_hint_bar,
    },
};

use crate::{
    ActionId, FocusRegion, LayoutMode, Model, Screen, ShellKeyAction, ShellTarget,
    model::editor::EditorField,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ShellView;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellGeometry {
    regions: Vec<HitRegion<ShellTarget>>,
}

impl ShellGeometry {
    #[must_use]
    pub fn target_at(&self, column: u16, row: u16) -> Option<ShellTarget> {
        let position = Position::new(column, row);
        self.regions
            .iter()
            .rev()
            .find(|region| region.area.contains(position))
            .map(|region| region.id)
    }

    fn push(&mut self, id: ShellTarget, area: Rect) {
        if !area.is_empty() {
            self.regions.push(HitRegion { id, area });
        }
    }

    pub(crate) fn focus_area(&self, focus: FocusRegion) -> Option<Rect> {
        self.regions.iter().find_map(|region| {
            let owns_focus = match region.id {
                ShellTarget::Focus(candidate) => candidate == focus,
                ShellTarget::Action(_) => focus == FocusRegion::Actions,
            };
            owns_focus.then_some(region.area)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellTab {
    Connections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusId {
    State,
    Focus,
}

impl ShellView {
    pub fn render(&self, model: &Model, frame: &mut Frame<'_>, area: Rect) {
        let _ = self.render_with_geometry(model, frame, area);
    }

    #[must_use]
    pub fn render_with_geometry(
        &self,
        model: &Model,
        frame: &mut Frame<'_>,
        area: Rect,
    ) -> ShellGeometry {
        let mut geometry = ShellGeometry::default();
        if model.layout_mode() == LayoutMode::TooSmall {
            render_panel(model, frame, area, "TableRock — Too Small", true);
            return geometry;
        }

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);
        render_panel(
            model,
            frame,
            rows[0],
            "TableRock — Connections",
            model.focus() == Some(FocusRegion::Context),
        );
        geometry.push(ShellTarget::Focus(FocusRegion::Context), rows[0]);
        render_tabs(model, frame, rows[1], &mut geometry);
        render_body(model, frame, rows[2], &mut geometry);
        render_actions(model, frame, rows[3], &mut geometry);
        render_hints(model, frame, rows[4]);
        render_status(model, frame, rows[5], &mut geometry);
        if model.password_prompt().is_some() {
            render_password_prompt_overlay(model, frame, rows[2]);
        } else if model.confirm().is_some() {
            render_confirm_overlay(model, frame, rows[2]);
        }
        geometry
    }
}

fn render_confirm_overlay(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    use ratatui_core::widgets::Widget;
    let Some(confirm) = model.confirm() else {
        return;
    };
    let (title, body) = match confirm {
        crate::model::ConfirmDialog::RemoveProfile { name, id_hex } => (
            "Remove profile?",
            format!("Remove '{name}' ({id_hex})? Active sessions must be reviewed."),
        ),
        crate::model::ConfirmDialog::RemoveGroup { name } => (
            "Remove group?",
            format!("Remove group '{name}'? Members become ungrouped."),
        ),
        crate::model::ConfirmDialog::RenameGroup {
            old_name,
            confirm_buffer,
        } => (
            "Rename group?",
            format!("Rename group '{old_name}' to new name. Paste name [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::CloseDirtyTab { title, .. } => (
            "Close tab?",
            format!("Close '{title}' with unsaved changes?"),
        ),
        crate::model::ConfirmDialog::TruncateTable {
            schema,
            table,
            confirm_buffer,
        } => (
            "Truncate table?",
            format!(
                "TRUNCATE {schema}.{table}. Paste table name '{table}' to confirm [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::DropTable {
            schema,
            table,
            confirm_buffer,
        } => (
            "Drop table?",
            format!(
                "DROP TABLE {schema}.{table}. Paste table name '{table}' to confirm [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::VacuumTable {
            schema,
            table,
            confirm_buffer,
        } => (
            "Vacuum table?",
            format!(
                "VACUUM {schema}.{table}. Paste table name '{table}' to confirm [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::AnalyzeTable {
            schema,
            table,
            confirm_buffer,
        } => (
            "Analyze table?",
            format!(
                "ANALYZE {schema}.{table}. Paste table name '{table}' to confirm [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::OptimizeTable {
            schema,
            table,
            confirm_buffer,
        } => (
            "Optimize table?",
            format!(
                "OPTIMIZE TABLE {schema}.{table}. Paste table name '{table}' to confirm [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::CancelBackend { confirm_buffer, .. } => (
            "Cancel backend?",
            format!("Paste pid digits to pg_cancel_backend [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::TerminateBackend { confirm_buffer, .. } => (
            "Terminate backend?",
            format!("Paste pid digits to pg_terminate_backend [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::RedisSubscribe {
            pattern,
            confirm_buffer,
        } => {
            if *pattern {
                (
                    "Pattern subscribe?",
                    format!(
                        "PSUBSCRIBE pattern (isolated connection). Paste pattern [{confirm_buffer}]"
                    ),
                )
            } else {
                (
                    "Subscribe?",
                    format!(
                        "SUBSCRIBE channel (isolated connection). Paste channel [{confirm_buffer}]"
                    ),
                )
            }
        }
        crate::model::ConfirmDialog::KillMutation {
            database,
            table,
            confirm_buffer,
        } => (
            "Kill ClickHouse mutation?",
            format!(
                "KILL MUTATION on {database}.{table}. Paste exact mutation_id [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::SaveFilter {
            schema,
            table,
            confirm_buffer,
        } => (
            "Save filter preset?",
            format!(
                "Save current filters for {schema}.{table} as named preset. Paste name [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::ApplyFilter {
            schema,
            table,
            known_names,
            confirm_buffer,
        } => {
            let ranked = crate::model::saved_filter::rank_preset_names(
                known_names,
                confirm_buffer,
                8,
            );
            let known = if known_names.is_empty() {
                "(none saved)".into()
            } else if confirm_buffer.trim().is_empty() {
                known_names.join(", ")
            } else if ranked.is_empty() {
                "(no fuzzy match)".into()
            } else {
                ranked.join(", ")
            };
            (
                "Apply filter preset?",
                format!(
                    "Load preset for {schema}.{table}. Matches: {known}. Paste name [{confirm_buffer}]"
                ),
            )
        }
        crate::model::ConfirmDialog::EditRawWhere { confirm_buffer } => (
            "Edit raw WHERE?",
            format!(
                "Paste predicate only (no semicolon). Empty clears. [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::EditQuickFilter { confirm_buffer } => (
            "Page-local filter?",
            format!(
                "Filter resident rows only (no server I/O). Empty clears. [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::GoToRow { confirm_buffer } => (
            "Go to row?",
            format!("Paste absolute 0-based row index [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::GoToColumn { confirm_buffer } => (
            "Go to column?",
            format!("Paste column name or unique prefix [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::RenameTab { confirm_buffer } => (
            "Rename tab?",
            format!("Paste new tab title (1–128 chars) [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::GoToTab { confirm_buffer } => (
            "Go to tab?",
            format!("Paste tab title or unique prefix [{confirm_buffer}]"),
        ),
        crate::model::ConfirmDialog::PickDate {
            year,
            month,
            confirm_buffer,
            calendar_text,
            ..
        } => (
            "Pick date?",
            format!(
                "{calendar_text}Paste day 1-31 or YYYY-MM-DD for {year:04}-{month:02} [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::CopyPick { confirm_buffer } => (
            "Copy format?",
            format!(
                "scope format: row|loaded + csv|tsv|json|md|insert|update [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::EditInsertValues {
            draft_id,
            confirm_buffer,
        } => (
            "Edit insert values?",
            format!(
                "draft #{draft_id}: paste col=value lines (empty value → NULL) [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::StageRedis {
            op,
            logical_db,
            key,
            confirm_buffer,
        } => {
            let hint = match op.as_str() {
                "hset" => "field=value",
                "zadd" => "score=member",
                "hdel" | "sadd" | "srem" | "zrem" => "field-or-member",
                _ => "payload",
            };
            (
                "Stage Redis collection change?",
                format!(
                    "{op} on db={logical_db} key={key}. Paste {hint} [{confirm_buffer}]"
                ),
            )
        }
        crate::model::ConfirmDialog::DdlReview {
            kind,
            schema,
            table,
            preview,
            confirm_buffer,
        } => (
            "Review DDL?",
            format!(
                "{preview}. Type object details for {kind} on {schema}.{table} [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::RenameTable {
            schema,
            table,
            confirm_buffer,
        } => (
            "Rename table?",
            format!(
                "RENAME {schema}.{table}. Paste new table name [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::StartupReview {
            items,
            confirm_buffer,
        } => {
            let preview: Vec<String> = items
                .iter()
                .take(4)
                .map(|(safety, stmt)| {
                    let short = if stmt.len() > 48 {
                        format!("{}…", &stmt[..48])
                    } else {
                        stmt.clone()
                    };
                    format!("[{safety}] {short}")
                })
                .collect();
            let more = if items.len() > 4 {
                format!(" (+{} more)", items.len() - 4)
            } else {
                String::new()
            };
            (
                "Authorize startup writes?",
                format!(
                    "{} skipped action(s): {}. Paste RUN to execute [{confirm_buffer}]{more}",
                    items.len(),
                    preview.join("; ")
                ),
            )
        }
        crate::model::ConfirmDialog::PgTool {
            kind,
            confirm_buffer,
        } => (
            if kind == "restore" {
                "pg_restore?"
            } else {
                "pg_dump?"
            },
            format!(
                "Paste file path (default tablerock.dump) then Submit [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::ImportUrl { confirm_buffer } => (
            "Import connection URL?",
            format!(
                "Paste postgres/redis/clickhouse URL then Submit [{confirm_buffer}]"
            ),
        ),
        crate::model::ConfirmDialog::OpenExternalUrl {
            summary,
            confirm_buffer,
            matched_profile_id_hex,
            url,
        } => {
            if summary.is_empty() {
                (
                    "Open external URL?",
                    format!(
                        "Paste connection URL then Submit; then paste OPEN to connect [{confirm_buffer}]"
                    ),
                )
            } else {
                let _ = url; // never display raw URL (may embed credentials)
                let title = if matched_profile_id_hex.is_some() {
                    "Confirm open matched profile?"
                } else {
                    "Confirm temporary connect?"
                };
                (
                    title,
                    format!("{summary}. Paste OPEN to proceed [{confirm_buffer}]"),
                )
            }
        }
        crate::model::ConfirmDialog::QuickSwitch { confirm_buffer } => {
            use crate::model::profiles::ProfileListState;
            use crate::model::saved_query::SavedQueryPanel;
            let mut preview = Vec::new();
            match model.screen() {
                crate::model::Screen::Workbench => {
                    for (i, t) in model.workbench().tabs.iter().enumerate().take(6) {
                        preview.push(format!("t{}:{}", i + 1, t.title));
                    }
                    if let SavedQueryPanel::Open { entries, .. } = &model.workbench().saved_queries {
                        for q in entries.iter().take(4) {
                            preview.push(format!("q:{}", q.name));
                        }
                    }
                }
                crate::model::Screen::Connections
                | crate::model::Screen::ConnectionPicker => {
                    if let ProfileListState::Loaded { rows, .. } = model.profiles() {
                        for r in rows.iter().take(8) {
                            preview.push(format!("p:{}", r.name));
                        }
                    }
                }
                _ => {}
            }
            (
                "Quick switch?",
                format!(
                    "{}. Paste index/name substring [{confirm_buffer}]",
                    if preview.is_empty() {
                        "no candidates".into()
                    } else {
                        preview.join(" ")
                    }
                ),
            )
        }
        crate::model::ConfirmDialog::BindParams {
            names,
            confirm_buffer,
            ..
        } => (
            "Bind parameters?",
            format!(
                "Need: {}. Paste name=value;… then Submit [{}]",
                names.join(", "),
                confirm_buffer
            ),
        ),
        crate::model::ConfirmDialog::FindReplace { confirm_buffer } => (
            "Find/replace?",
            format!(
                "Paste find=>replace[=>all][=>i] then Submit [{confirm_buffer}]"
            ),
        ),
    };
    let panel = Panel::new(&model.theme)
        .title(title)
        .emphasis(PanelEmphasis::Focused);
    frame.render_widget(&panel, area);
    if area.height > 2 && area.width > 2 {
        let inner = Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        Line::from(body).render(inner, frame.buffer_mut());
    }
}

fn render_password_prompt_overlay(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    use ratatui_core::widgets::Widget;
    let Some(prompt) = model.password_prompt() else {
        return;
    };
    let panel = Panel::new(&model.theme)
        .title("Password required")
        .emphasis(PanelEmphasis::Focused);
    frame.render_widget(&panel, area);
    if area.height > 2 && area.width > 2 {
        let inner = Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        let masked = "•".repeat(prompt.buffer.chars().count().min(64));
        Line::from(format!(
            "profile {} password: {masked}",
            prompt.profile_id_hex
        ))
        .render(inner, frame.buffer_mut());
    }
}

fn render_tabs(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let label = if model.focus() == Some(FocusRegion::Tabs) {
        "> Connections"
    } else {
        "Connections"
    };
    let tabs = [Tab {
        id: ShellTab::Connections,
        label,
        glyph: None,
        active: true,
        enabled: true,
    }];
    let mut state = TabsState {
        selected: Some(ShellTab::Connections),
        hovered: None,
        focused: model.focus() == Some(FocusRegion::Tabs),
        regions: Vec::new(),
    };
    frame.render_stateful_widget(Tabs::new(&tabs, &model.theme).gap(1), area, &mut state);
    for region in state.regions {
        geometry.push(ShellTarget::Focus(FocusRegion::Tabs), region.area);
    }
}

fn render_body(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    if model.screen() == Screen::ConnectionPicker {
        render_panel(model, frame, area, "Connection Picker", true);
        geometry.push(ShellTarget::Focus(FocusRegion::Content), area);
        return;
    }
    match model.layout_mode() {
        LayoutMode::Wide | LayoutMode::Medium => {
            let catalog = if model.layout_mode() == LayoutMode::Wide {
                Constraint::Length(30)
            } else {
                Constraint::Percentage(32)
            };
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([catalog, Constraint::Min(20)])
                .split(area);
            render_panel(
                model,
                frame,
                columns[0],
                "Catalog",
                model.focus() == Some(FocusRegion::Catalog),
            );
            geometry.push(ShellTarget::Focus(FocusRegion::Catalog), columns[0]);
            render_panel(
                model,
                frame,
                columns[1],
                "Workspace",
                model.focus() == Some(FocusRegion::Content),
            );
            geometry.push(ShellTarget::Focus(FocusRegion::Content), columns[1]);
        }
        LayoutMode::Narrow => {
            let (title, focused, target) = match model.focus() {
                Some(FocusRegion::Catalog) => ("Catalog", true, FocusRegion::Catalog),
                Some(FocusRegion::Content) => ("Workspace", true, FocusRegion::Content),
                _ => ("Connections", false, FocusRegion::Context),
            };
            render_panel(model, frame, area, title, focused);
            geometry.push(ShellTarget::Focus(target), area);
        }
        LayoutMode::TooSmall => {}
    }
}

fn render_actions(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let open = action_label(model, ActionId::Open, "Open");
    let new = action_label(model, ActionId::New, "New");
    let import_url = action_label(model, ActionId::ImportUrl, "URL");
    let open_ext = action_label(model, ActionId::OpenExternalUrl, "OpenURL");
    let quick_conn = action_label(model, ActionId::QuickSwitch, "Switch");
    let save = action_label(model, ActionId::Save, "Save");
    let test = action_label(model, ActionId::Test, "Test");
    let connect = action_label(model, ActionId::Connect, "Connect");
    let disconnect = action_label(model, ActionId::Disconnect, "Disconnect");
    let next_db = action_label(model, ActionId::NextDatabase, "Next DB");
    let next_tab = action_label(model, ActionId::NextTab, "Next Tab");
    let prev_tab = action_label(model, ActionId::PrevTab, "Prev Tab");
    let quick = action_label(model, ActionId::QuickSwitch, "Switch");
    let pin_tab = action_label(model, ActionId::PinTab, "Pin");
    let new_sql = action_label(model, ActionId::NewSql, "SQL");
    let run_sql = action_label(model, ActionId::RunSql, "Run");
    let run_script = action_label(model, ActionId::RunScript, "Script");
    let explain = action_label(model, ActionId::Explain, "Explain");
    let find_rep = action_label(model, ActionId::FindReplace, "FindRep");
    let format_sql = action_label(model, ActionId::FormatSql, "Format");
    let complete = action_label(model, ActionId::Complete, "Complete");
    let history = action_label(model, ActionId::History, "History");
    let restore_hist = action_label(model, ActionId::RestoreHistory, "Restore");
    let saved_q = action_label(model, ActionId::SavedQueries, "Queries");
    let save_q = action_label(model, ActionId::SaveQuery, "SaveQ");
    let load_q = action_label(model, ActionId::LoadQuery, "LoadQ");
    let save_file = action_label(model, ActionId::SaveFile, "SaveFile");
    let save_intent = action_label(model, ActionId::SaveIntent, "SaveIntent");
    let save_filter = action_label(model, ActionId::SaveFilter, "SaveFilt");
    let apply_filter = action_label(model, ActionId::ApplyFilter, "LoadFilt");
    let filt_null = action_label(model, ActionId::FilterIsNull, "IsNull");
    let filt_nn = action_label(model, ActionId::FilterIsNotNull, "NotNull");
    let filt_empty = action_label(model, ActionId::FilterEmpty, "Empty");
    let filt_not_empty = action_label(model, ActionId::FilterNotEmpty, "NotEmpty");
    let filt_loc = action_label(model, ActionId::FilterByLocator, "FiltLoc");
    let filt_pop = action_label(model, ActionId::RemoveLastFilter, "PopFilt");
    let filt_col = action_label(model, ActionId::RemoveColumnFilters, "ClrColF");
    let filt_like = action_label(model, ActionId::FilterLike, "Like");
    let filt_ilike = action_label(model, ActionId::FilterILike, "ILike");
    let filt_ne = action_label(model, ActionId::FilterNe, "NotEq");
    let filt_lt = action_label(model, ActionId::FilterLt, "Lt");
    let filt_le = action_label(model, ActionId::FilterLe, "Le");
    let filt_gt = action_label(model, ActionId::FilterGt, "Gt");
    let filt_ge = action_label(model, ActionId::FilterGe, "Ge");
    let raw_where = action_label(model, ActionId::EditRawWhere, "RawWhere");
    let clear_raw = action_label(model, ActionId::ClearRawWhere, "ClrRaw");
    let copy_filt_bar = action_label(model, ActionId::CopyFilterBar, "CopyBar");
    let clear_sort = action_label(model, ActionId::ClearSort, "ClrSort");
    let cycle_sort = action_label(model, ActionId::CycleSort, "Sort");
    let push_sort = action_label(model, ActionId::PushSort, "Sort+");
    let pop_sort = action_label(model, ActionId::PopSort, "Sort-");
    let inv_sort = action_label(model, ActionId::InvertPrimarySort, "SortInv");
    let rot_sort = action_label(model, ActionId::RotateSort, "SortRot");
    let keep_sort = action_label(model, ActionId::KeepPrimarySort, "Sort1");
    let quick_filt = action_label(model, ActionId::EditQuickFilter, "PgFilt");
    let clear_quick = action_label(model, ActionId::ClearQuickFilter, "ClrPgF");
    let go_row = action_label(model, ActionId::GoToRow, "GoRow");
    let go_first = action_label(model, ActionId::GoToFirstRow, "First");
    let go_last = action_label(model, ActionId::GoToLastRow, "Last");
    let go_col = action_label(model, ActionId::GoToColumn, "GoCol");
    let home_cur = action_label(model, ActionId::HomeCursor, "Home");
    let end_cur = action_label(model, ActionId::EndCursor, "End");
    let refresh = action_label(model, ActionId::RefreshTable, "Refresh");
    let col_left = action_label(model, ActionId::MoveColumnLeft, "ColL");
    let col_right = action_label(model, ActionId::MoveColumnRight, "ColR");
    let col_narrow = action_label(model, ActionId::NarrowColumn, "Col-");
    let col_widen = action_label(model, ActionId::WidenColumn, "Col+");
    let col_fit = action_label(model, ActionId::FitColumn, "ColFit");
    let col_fit_all = action_label(model, ActionId::FitAllColumns, "ColFitA");
    let col_toggle = action_label(model, ActionId::ToggleColumn, "ColVis");
    let col_reset = action_label(model, ActionId::ResetColumns, "ColRst");
    let col_solo = action_label(model, ActionId::SoloColumn, "ColSolo");
    let col_all = action_label(model, ActionId::ShowAllColumns, "ColAll");
    let col_inv = action_label(model, ActionId::InvertColumns, "ColInv");
    let col_save = action_label(model, ActionId::SaveColumns, "ColSave");
    let undo_staged = action_label(model, ActionId::UndoStaged, "UndoEdit");
    let discard_staged = action_label(model, ActionId::DiscardStaged, "DiscardEdits");
    let review_mut = action_label(model, ActionId::ReviewMutations, "Review");
    let edit_cell = action_label(model, ActionId::EditCell, "Edit");
    let copy_cell = action_label(model, ActionId::CopyCell, "CopyCell");
    let copy_cell_hex = action_label(model, ActionId::CopyCellHex, "CopyHex");
    let copy_row = action_label(model, ActionId::CopyRow, "CopyRow");
    let copy_row_csv = action_label(model, ActionId::CopyRowCsv, "RowCsv");
    let copy_row_json = action_label(model, ActionId::CopyRowJson, "RowJson");
    let copy_row_md = action_label(model, ActionId::CopyRowMarkdown, "RowMd");
    let copy_row_ins = action_label(model, ActionId::CopyRowSqlInsert, "RowIns");
    let copy_row_upd = action_label(model, ActionId::CopyRowSqlUpdate, "RowUpd");
    let copy_pick = action_label(model, ActionId::CopyPick, "CopyPick");
    let copy_cols = action_label(model, ActionId::CopyColumnNames, "CopyCols");
    let copy_col = action_label(model, ActionId::CopyColumn, "CopyCol");
    let copy_status = action_label(model, ActionId::CopyStatus, "CopyStat");
    let copy_table = action_label(model, ActionId::CopyTableName, "CopyTbl");
    let copy_pk = action_label(model, ActionId::CopyPkNames, "CopyPk");
    let copy_loc = action_label(model, ActionId::CopyLocator, "CopyLoc");
    let copy_where = action_label(model, ActionId::CopyWhere, "CopyWhere");
    let toggle_bool = action_label(model, ActionId::ToggleBool, "TogBool");
    let set_null = action_label(model, ActionId::SetNull, "SetNull");
    let set_today = action_label(model, ActionId::SetToday, "Today");
    let set_now = action_label(model, ActionId::SetNow, "Now");
    let inc_day = action_label(model, ActionId::IncDay, "Day+");
    let dec_day = action_label(model, ActionId::DecDay, "Day-");
    let inc_month = action_label(model, ActionId::IncMonth, "Mon+");
    let dec_month = action_label(model, ActionId::DecMonth, "Mon-");
    let pick_date = action_label(model, ActionId::PickDate, "Cal");
    let inc_num = action_label(model, ActionId::IncNumber, "Num+");
    let dec_num = action_label(model, ActionId::DecNumber, "Num-");
    let fmt_json = action_label(model, ActionId::FormatJson, "FmtJson");
    let compact_json = action_label(model, ActionId::CompactJson, "CmpJson");
    let delete_row = action_label(model, ActionId::DeleteRow, "DelRow");
    let insert_row = action_label(model, ActionId::InsertRow, "InsRow");
    let dup_row = action_label(model, ActionId::DuplicateRow, "DupRow");
    let edit_insert = action_label(model, ActionId::EditInsert, "EditIns");
    let discard_last_ins = action_label(model, ActionId::DiscardLastInsert, "DropIns");
    let unstage_cell = action_label(model, ActionId::UnstageCell, "UnstgCell");
    let unstage_row = action_label(model, ActionId::UnstageRow, "UnstgRow");
    let show_staged = action_label(model, ActionId::ShowStaged, "Staged");
    let copy_staged = action_label(model, ActionId::CopyStaged, "CopyStg");
    let show_notices = action_label(model, ActionId::ShowNotices, "Notices");
    let clear_notices = action_label(model, ActionId::ClearNotices, "ClrNtc");
    let copy_notices = action_label(model, ActionId::CopyNotices, "CopyNtc");
    let hex_more = action_label(model, ActionId::HexMore, "Hex+");
    let hex_less = action_label(model, ActionId::HexLess, "Hex-");
    let expand_tree = action_label(model, ActionId::ExpandTree, "Tree+");
    let collapse_tree = action_label(model, ActionId::CollapseTree, "Tree-");
    let apply_mut = action_label(model, ActionId::ApplyMutations, "Apply");
    let follow_fk = action_label(model, ActionId::FollowForeignKey, "FollowFK");
    let structure = action_label(model, ActionId::ShowStructure, "Structure");
    let copy_ddl = action_label(model, ActionId::CopyStructureDdl, "CopyDdl");
    let truncate = action_label(model, ActionId::TruncateTable, "Truncate");
    let drop_t = action_label(model, ActionId::DropTable, "Drop");
    let vacuum_t = action_label(model, ActionId::VacuumTable, "Vacuum");
    let analyze_t = action_label(model, ActionId::AnalyzeTable, "Analyze");
    let optimize_t = action_label(model, ActionId::OptimizeTable, "Optimize");
    let rename_t = action_label(model, ActionId::RenameTable, "Rename");
    let ddl_add = action_label(model, ActionId::DdlAddColumn, "AddCol");
    let ddl_idx = action_label(model, ActionId::DdlCreateIndex, "AddIdx");
    let ddl_drop_col = action_label(model, ActionId::DdlDropColumn, "DropCol");
    let ddl_drop_idx = action_label(model, ActionId::DdlDropIndex, "DropIdx");
    let ddl_add_c = action_label(model, ActionId::DdlAddConstraint, "AddCon");
    let ddl_drop_c = action_label(model, ActionId::DdlDropConstraint, "DropCon");
    let activity = action_label(model, ActionId::ShowActivity, "Activity");
    let roles = action_label(model, ActionId::ShowRoles, "Roles");
    let cancel_be = action_label(model, ActionId::CancelBackend, "CancelBE");
    let term_be = action_label(model, ActionId::TerminateBackend, "TermBE");
    let kill_mut = action_label(model, ActionId::KillMutation, "KillMut");
    let scan_redis = action_label(model, ActionId::ScanRedisKeys, "ScanKeys");
    let redis_info = action_label(model, ActionId::RedisInfo, "RedisInfo");
    let redis_add = action_label(model, ActionId::StageRedisAdd, "RAdd");
    let redis_rm = action_label(model, ActionId::StageRedisRemove, "RRem");
    let redis_more = action_label(model, ActionId::RedisCollectionMore, "RMore");
    let redis_sub = action_label(model, ActionId::RedisSubscribe, "Sub");
    let redis_psub = action_label(model, ActionId::RedisPSubscribe, "PSub");
    let export_csv = action_label(model, ActionId::ExportCsv, "ExpCsv");
    let export_json = action_label(model, ActionId::ExportJson, "ExpJson");
    let export_tsv = action_label(model, ActionId::ExportTsv, "ExpTsv");
    let export_stream = action_label(model, ActionId::ExportStreamCsv, "ExpStream");
    let import_csv = action_label(model, ActionId::ImportCsv, "ImpCsv");
    let pg_dump = action_label(model, ActionId::PgDump, "PgDump");
    let pg_restore = action_label(model, ActionId::PgRestore, "PgRestore");
    let cancel_q = action_label(model, ActionId::CancelQuery, "Cancel");
    let inspect = action_label(model, ActionId::Inspect, "Inspect");
    let close_insp = action_label(model, ActionId::CloseInspector, "CloseInsp");
    let close_tab = action_label(model, ActionId::CloseTab, "Close Tab");
    let close_others = action_label(model, ActionId::CloseOtherTabs, "CloseOthers");
    let rename_tab = action_label(model, ActionId::RenameTab, "RenTab");
    let tab_left = action_label(model, ActionId::MoveTabLeft, "TabL");
    let tab_right = action_label(model, ActionId::MoveTabRight, "TabR");
    let dup_tab = action_label(model, ActionId::DuplicateTab, "DupTab");
    let go_tab = action_label(model, ActionId::GoToTab, "GoTab");
    let list_tabs = action_label(model, ActionId::ListTabs, "ListTabs");
    let copy_tabs = action_label(model, ActionId::CopyTabs, "CopyTabs");
    let submit = action_label(model, ActionId::Submit, "Submit");
    let cancel = action_label(model, ActionId::Cancel, "Cancel");
    let quit = action_label(model, ActionId::Quit, "Quit");
    let remove = action_label(model, ActionId::Remove, "Remove");
    let rename_group = action_label(model, ActionId::RenameGroup, "RenGroup");
    let reconnect = action_label(model, ActionId::Reconnect, "Reconn");
    let session_health = action_label(model, ActionId::SessionHealth, "Health");
    let actions: Vec<Action<'_, ActionId>> =
        if model.password_prompt().is_some() || model.confirm().is_some() {
            vec![
                Action {
                    id: ActionId::Submit,
                    label: submit.as_str(),
                    enabled: true,
                    style: None,
                },
                Action {
                    id: ActionId::Cancel,
                    label: cancel.as_str(),
                    enabled: true,
                    style: None,
                },
                Action {
                    id: ActionId::Quit,
                    label: quit.as_str(),
                    enabled: true,
                    style: None,
                },
            ]
        } else {
            match model.screen() {
                crate::Screen::Editor => vec![
                    Action {
                        id: ActionId::Save,
                        label: save.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Test,
                        label: test.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Connect,
                        label: connect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ImportUrl,
                        label: import_url.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::OpenExternalUrl,
                        label: open_ext.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Cancel,
                        label: cancel.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Quit,
                        label: quit.as_str(),
                        enabled: true,
                        style: None,
                    },
                ],
                crate::Screen::Workbench => vec![
                    Action {
                        id: ActionId::NextDatabase,
                        label: next_db.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PrevTab,
                        label: prev_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::NextTab,
                        label: next_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::QuickSwitch,
                        label: quick.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PinTab,
                        label: pin_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::NewSql,
                        label: new_sql.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RunSql,
                        label: run_sql.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RunScript,
                        label: run_script.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Explain,
                        label: explain.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FindReplace,
                        label: find_rep.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FormatSql,
                        label: format_sql.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Complete,
                        label: complete.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::History,
                        label: history.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RestoreHistory,
                        label: restore_hist.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SavedQueries,
                        label: saved_q.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SaveQuery,
                        label: save_q.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::LoadQuery,
                        label: load_q.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SaveFile,
                        label: save_file.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SaveIntent,
                        label: save_intent.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SaveFilter,
                        label: save_filter.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ApplyFilter,
                        label: apply_filter.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterIsNull,
                        label: filt_null.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterIsNotNull,
                        label: filt_nn.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterEmpty,
                        label: filt_empty.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterNotEmpty,
                        label: filt_not_empty.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterByLocator,
                        label: filt_loc.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RemoveLastFilter,
                        label: filt_pop.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RemoveColumnFilters,
                        label: filt_col.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterLike,
                        label: filt_like.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterILike,
                        label: filt_ilike.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterNe,
                        label: filt_ne.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterLt,
                        label: filt_lt.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterLe,
                        label: filt_le.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterGt,
                        label: filt_gt.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FilterGe,
                        label: filt_ge.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::EditRawWhere,
                        label: raw_where.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ClearRawWhere,
                        label: clear_raw.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyFilterBar,
                        label: copy_filt_bar.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CycleSort,
                        label: cycle_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PushSort,
                        label: push_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PopSort,
                        label: pop_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::InvertPrimarySort,
                        label: inv_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RotateSort,
                        label: rot_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::KeepPrimarySort,
                        label: keep_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ClearSort,
                        label: clear_sort.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::EditQuickFilter,
                        label: quick_filt.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ClearQuickFilter,
                        label: clear_quick.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::GoToRow,
                        label: go_row.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::GoToFirstRow,
                        label: go_first.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::GoToLastRow,
                        label: go_last.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::GoToColumn,
                        label: go_col.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::HomeCursor,
                        label: home_cur.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::EndCursor,
                        label: end_cur.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RefreshTable,
                        label: refresh.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::MoveColumnLeft,
                        label: col_left.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::MoveColumnRight,
                        label: col_right.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::NarrowColumn,
                        label: col_narrow.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::WidenColumn,
                        label: col_widen.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FitColumn,
                        label: col_fit.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FitAllColumns,
                        label: col_fit_all.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ToggleColumn,
                        label: col_toggle.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ResetColumns,
                        label: col_reset.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SoloColumn,
                        label: col_solo.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ShowAllColumns,
                        label: col_all.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::InvertColumns,
                        label: col_inv.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SaveColumns,
                        label: col_save.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::UndoStaged,
                        label: undo_staged.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DiscardStaged,
                        label: discard_staged.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ReviewMutations,
                        label: review_mut.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::EditCell,
                        label: edit_cell.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyCell,
                        label: copy_cell.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyCellHex,
                        label: copy_cell_hex.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyRow,
                        label: copy_row.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyRowCsv,
                        label: copy_row_csv.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyRowJson,
                        label: copy_row_json.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyRowMarkdown,
                        label: copy_row_md.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyRowSqlInsert,
                        label: copy_row_ins.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyRowSqlUpdate,
                        label: copy_row_upd.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyPick,
                        label: copy_pick.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyColumnNames,
                        label: copy_cols.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyColumn,
                        label: copy_col.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyStatus,
                        label: copy_status.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyTableName,
                        label: copy_table.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyPkNames,
                        label: copy_pk.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyLocator,
                        label: copy_loc.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyWhere,
                        label: copy_where.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ToggleBool,
                        label: toggle_bool.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SetNull,
                        label: set_null.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SetToday,
                        label: set_today.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SetNow,
                        label: set_now.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::IncDay,
                        label: inc_day.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DecDay,
                        label: dec_day.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::IncMonth,
                        label: inc_month.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DecMonth,
                        label: dec_month.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PickDate,
                        label: pick_date.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::IncNumber,
                        label: inc_num.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DecNumber,
                        label: dec_num.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FormatJson,
                        label: fmt_json.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CompactJson,
                        label: compact_json.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DeleteRow,
                        label: delete_row.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::InsertRow,
                        label: insert_row.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DuplicateRow,
                        label: dup_row.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::EditInsert,
                        label: edit_insert.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DiscardLastInsert,
                        label: discard_last_ins.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::UnstageCell,
                        label: unstage_cell.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::UnstageRow,
                        label: unstage_row.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ShowStaged,
                        label: show_staged.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyStaged,
                        label: copy_staged.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ShowNotices,
                        label: show_notices.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ClearNotices,
                        label: clear_notices.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyNotices,
                        label: copy_notices.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::HexMore,
                        label: hex_more.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::HexLess,
                        label: hex_less.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ExpandTree,
                        label: expand_tree.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CollapseTree,
                        label: collapse_tree.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ApplyMutations,
                        label: apply_mut.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::FollowForeignKey,
                        label: follow_fk.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ShowStructure,
                        label: structure.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyStructureDdl,
                        label: copy_ddl.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::TruncateTable,
                        label: truncate.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DropTable,
                        label: drop_t.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RenameTable,
                        label: rename_t.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::VacuumTable,
                        label: vacuum_t.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::AnalyzeTable,
                        label: analyze_t.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::OptimizeTable,
                        label: optimize_t.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DdlAddColumn,
                        label: ddl_add.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DdlCreateIndex,
                        label: ddl_idx.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DdlDropColumn,
                        label: ddl_drop_col.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DdlDropIndex,
                        label: ddl_drop_idx.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DdlAddConstraint,
                        label: ddl_add_c.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DdlDropConstraint,
                        label: ddl_drop_c.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ShowActivity,
                        label: activity.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ShowRoles,
                        label: roles.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CancelBackend,
                        label: cancel_be.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::TerminateBackend,
                        label: term_be.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::KillMutation,
                        label: kill_mut.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ScanRedisKeys,
                        label: scan_redis.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RedisInfo,
                        label: redis_info.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::StageRedisAdd,
                        label: redis_add.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::StageRedisRemove,
                        label: redis_rm.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RedisCollectionMore,
                        label: redis_more.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RedisSubscribe,
                        label: redis_sub.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RedisPSubscribe,
                        label: redis_psub.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ExportCsv,
                        label: export_csv.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ExportJson,
                        label: export_json.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ExportTsv,
                        label: export_tsv.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ExportStreamCsv,
                        label: export_stream.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ImportCsv,
                        label: import_csv.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PgDump,
                        label: pg_dump.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::PgRestore,
                        label: pg_restore.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CancelQuery,
                        label: cancel_q.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Inspect,
                        label: inspect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CloseInspector,
                        label: close_insp.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CloseTab,
                        label: close_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CloseOtherTabs,
                        label: close_others.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RenameTab,
                        label: rename_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::MoveTabLeft,
                        label: tab_left.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::MoveTabRight,
                        label: tab_right.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::DuplicateTab,
                        label: dup_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::GoToTab,
                        label: go_tab.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ListTabs,
                        label: list_tabs.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::CopyTabs,
                        label: copy_tabs.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Disconnect,
                        label: disconnect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::SessionHealth,
                        label: session_health.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Reconnect,
                        label: reconnect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Quit,
                        label: quit.as_str(),
                        enabled: true,
                        style: None,
                    },
                ],
                _ => vec![
                    Action {
                        id: ActionId::Open,
                        label: open.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::New,
                        label: new.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::ImportUrl,
                        label: import_url.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::OpenExternalUrl,
                        label: open_ext.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::QuickSwitch,
                        label: quick_conn.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Remove,
                        label: remove.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::RenameGroup,
                        label: rename_group.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Reconnect,
                        label: reconnect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Quit,
                        label: quit.as_str(),
                        enabled: true,
                        style: None,
                    },
                ],
            }
        };
    let mut state = ActionBarState {
        focused: (model.focus() == Some(FocusRegion::Actions)).then_some(model.selected_action()),
        regions: Vec::new(),
    };
    frame.render_stateful_widget(
        ActionBar::new(&actions, &model.theme).gap(" "),
        area,
        &mut state,
    );
    for region in state.regions {
        geometry.push(ShellTarget::Action(region.id), region.area);
    }
}

fn action_label(model: &Model, id: ActionId, base: &str) -> String {
    if model.focus() == Some(FocusRegion::Actions) && model.selected_action() == id {
        format!("> {base}")
    } else if model.hovered() == Some(ShellTarget::Action(id)) {
        format!("~ {base}")
    } else {
        base.to_owned()
    }
}

fn render_hints(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let mut keymap = model.keymap().clone();
    if model.focus() == Some(FocusRegion::Actions) {
        let _ = keymap.disable(ShellKeyAction::FocusPrevious);
        let _ = keymap.disable(ShellKeyAction::Quit);
    } else {
        let _ = keymap.disable(ShellKeyAction::Activate);
        let _ = keymap.disable(ShellKeyAction::ActionPrevious);
        let _ = keymap.disable(ShellKeyAction::ActionNext);
        let _ = keymap.disable(ShellKeyAction::Quit);
    }
    render_hint_bar(frame, area, &keymap.hint_spans(), &model.theme);
}

fn render_status(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let focus = if model.focus() == Some(FocusRegion::Footer) {
        "[FOCUSED] Footer"
    } else {
        "Footer"
    };
    let left = [StatusSlot {
        id: StatusId::State,
        content: if model.engine_resync_required() {
            "Resync required"
        } else {
            "Ready"
        },
        priority: 0,
        min_width: 5,
        enabled: true,
        style: Style::new(),
        hover_style: None,
    }];
    let right = [StatusSlot {
        id: StatusId::Focus,
        content: focus,
        priority: 0,
        min_width: 6,
        enabled: true,
        style: Style::new(),
        hover_style: None,
    }];
    let mut state = StatusBarState::default();
    frame.render_stateful_widget(
        StatusBar::new(&left, &right, &model.theme).alpha(1.0),
        area,
        &mut state,
    );
    for region in state.regions {
        geometry.push(ShellTarget::Focus(FocusRegion::Footer), region.area);
    }
}

fn render_panel(model: &Model, frame: &mut Frame<'_>, area: Rect, title: &str, focused: bool) {
    let focused_title = focused.then(|| format!("> {title}"));
    let body = if title == "Workspace" || title.ends_with("Workspace") {
        if model.screen() == crate::Screen::Workbench {
            Some(model.workbench().context.line())
        } else {
            Some(model.profiles().status_line())
        }
    } else if title == "Catalog" && model.screen() == crate::Screen::Workbench {
        Some(model.workbench().catalog_status_line())
    } else {
        None
    };
    let panel = Panel::new(&model.theme)
        .title(focused_title.as_deref().unwrap_or(title))
        .emphasis(if focused {
            PanelEmphasis::Focused
        } else {
            PanelEmphasis::Normal
        });
    frame.render_widget(&panel, area);
    if area.height <= 2 || area.width <= 2 {
        return;
    }
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let is_workspace = title == "Workspace" || title.ends_with("Workspace");
    let is_catalog = title == "Catalog" || title.ends_with("Catalog");
    if is_catalog && model.screen() == Screen::Workbench {
        render_workbench_catalog(model, frame, inner, body.as_deref());
        return;
    }
    if !is_workspace {
        if let Some(status) = body.as_ref() {
            use ratatui_core::widgets::Widget;
            Line::from(status.as_str()).render(inner, frame.buffer_mut());
        }
        return;
    }
    match model.screen() {
        Screen::Connections | Screen::ConnectionPicker => {
            render_connection_tree(model, frame, inner, body.as_deref());
        }
        Screen::Editor => render_connection_form(model, frame, inner),
        Screen::Workbench => render_workbench_facts(model, frame, inner, body.as_deref()),
    }
}

fn render_workbench_catalog(
    model: &Model,
    frame: &mut Frame<'_>,
    area: Rect,
    status: Option<&str>,
) {
    use ratatui_core::widgets::Widget;
    let mut content = area;
    if let Some(status) = status {
        Line::from(status).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let filter = match &model.workbench().catalog {
        crate::model::catalog::CatalogModel::Loaded { filter, .. } => filter.clone(),
        _ => String::new(),
    };
    if !filter.is_empty() && content.height > 0 {
        Line::from(format!("filter: {filter}")).render(
            Rect {
                x: content.x,
                y: content.y,
                width: content.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let visible = model.workbench().catalog.visible_nodes();
    if visible.is_empty() {
        return;
    }
    let labels: Vec<String> = visible.iter().map(|n| n.tree_label()).collect();
    let ids: Vec<String> = visible.iter().map(|n| n.id.clone()).collect();
    let depths: Vec<u16> = visible.iter().map(|n| n.depth).collect();
    let expanded: Vec<bool> = visible.iter().map(|n| n.expanded).collect();
    let branches: Vec<bool> = visible.iter().map(|n| n.branch).collect();
    let selected_key = match &model.workbench().catalog {
        crate::model::catalog::CatalogModel::Loaded { selected_id, .. } => selected_id.clone(),
        _ => None,
    };
    let tree_nodes: Vec<TreeNode<'_, String>> = ids
        .iter()
        .zip(labels.iter())
        .zip(depths.iter())
        .zip(expanded.iter())
        .zip(branches.iter())
        .map(|((((id, label), depth), exp), branch)| TreeNode {
            id: id.clone(),
            label: Line::from(label.as_str()),
            trailing: None,
            depth: *depth,
            branch: *branch,
            expanded: *exp,
            enabled: true,
            status: TreeNodeStatus::Ready,
        })
        .collect();
    let mut state = TreeState::new(selected_key);
    state.set_focused(model.focus() == Some(FocusRegion::Catalog));
    frame.render_stateful_widget(&Tree::new(&tree_nodes, &model.theme), content, &mut state);
}

fn render_connection_tree(model: &Model, frame: &mut Frame<'_>, area: Rect, status: Option<&str>) {
    use ratatui_core::widgets::Widget;
    let mut content = area;
    if let Some(status) = status {
        Line::from(status).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let search = match model.profiles() {
        crate::model::profiles::ProfileListState::Loaded { search, .. } => search.clone(),
        _ => String::new(),
    };
    if !search.is_empty() && content.height > 0 {
        Line::from(format!("filter: {search}")).render(
            Rect {
                x: content.x,
                y: content.y,
                width: content.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let (nodes, labels, depths, selected_key) = build_connection_tree_nodes(model);
    let tree_nodes: Vec<TreeNode<'_, String>> = nodes
        .iter()
        .zip(labels.iter())
        .zip(depths.iter())
        .map(|((id, label), depth)| {
            let is_group = id.starts_with("g:");
            TreeNode {
                id: id.clone(),
                label: Line::from(label.as_str()),
                trailing: None,
                depth: *depth,
                branch: is_group,
                expanded: is_group
                    && !model
                        .profiles()
                        .is_group_collapsed(id.strip_prefix("g:").unwrap_or("")),
                enabled: true,
                status: TreeNodeStatus::Ready,
            }
        })
        .collect();
    let mut state = TreeState::new(selected_key);
    state.set_focused(model.focus() == Some(FocusRegion::Content));
    frame.render_stateful_widget(&Tree::new(&tree_nodes, &model.theme), content, &mut state);
}

/// Build TermRock Tree projection: group branches + profile leaves.
fn build_connection_tree_nodes(
    model: &Model,
) -> (Vec<String>, Vec<String>, Vec<u16>, Option<String>) {
    let rows = model.profiles().visible_rows();
    let mut groups: Vec<Option<String>> = Vec::new();
    for row in &rows {
        let key = row.group.clone();
        if !groups.iter().any(|existing| existing == &key) {
            groups.push(key);
        }
    }
    groups.sort_by(|left, right| match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    });
    let mut ids = Vec::new();
    let mut labels = Vec::new();
    let mut depths = Vec::new();
    for group in groups {
        match &group {
            Some(name) => {
                ids.push(format!("g:{name}"));
                labels.push(name.clone());
                depths.push(0);
                if model.profiles().is_group_collapsed(name) {
                    continue;
                }
                for row in &rows {
                    if row.group.as_deref() == Some(name.as_str()) {
                        ids.push(format!("p:{}", row.id_hex));
                        labels.push(row.list_line());
                        depths.push(1);
                    }
                }
            }
            None => {
                for row in &rows {
                    if row.group.is_none() {
                        ids.push(format!("p:{}", row.id_hex));
                        labels.push(row.list_line());
                        depths.push(0);
                    }
                }
            }
        }
    }
    let selected_key = model
        .profiles()
        .selected_row()
        .map(|row| format!("p:{}", row.id_hex));
    (ids, labels, depths, selected_key)
}

fn render_connection_form(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    use ratatui_core::widgets::Widget;
    let editor = model.editor();
    let mut form_area = area;
    if let Some(error) = editor.validation_error.as_ref() {
        Line::from(format!("! {error}")).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        form_area.y = form_area.y.saturating_add(1);
        form_area.height = form_area.height.saturating_sub(1);
    } else if let Some(status) = editor.test_status.as_ref() {
        Line::from(format!("test: {status}")).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        form_area.y = form_area.y.saturating_add(1);
        form_area.height = form_area.height.saturating_sub(1);
    }
    // Owned display values live for this render frame.
    let engine = editor.field_value(EditorField::Engine);
    let name = editor.field_value(EditorField::Name);
    let group = editor.field_value(EditorField::Group);
    let environment = editor.field_value(EditorField::Environment);
    let host = editor.field_value(EditorField::Host);
    let port = editor.field_value(EditorField::Port);
    let database = editor.field_value(EditorField::Database);
    let username = editor.field_value(EditorField::Username);
    let password = editor.field_value(EditorField::Password);
    let password_source = editor.field_value(EditorField::PasswordSource);
    let tls_mode = editor.field_value(EditorField::TlsMode);
    let ssh_host = editor.field_value(EditorField::SshHost);
    let ssh_port = editor.field_value(EditorField::SshPort);
    let ssh_username = editor.field_value(EditorField::SshUsername);
    let ssh_password = editor.field_value(EditorField::SshPassword);
    let ssh_private_key = editor.field_value(EditorField::SshPrivateKey);
    let ssh_known_hosts = editor.field_value(EditorField::SshKnownHostsPath);
    let ssh_use_agent = editor.field_value(EditorField::SshUseAgent);
    let startup_sql = editor.field_value(EditorField::StartupSql);

    let general = [
        FormField::new(
            EditorField::Engine,
            Line::from("Engine"),
            Line::from(engine.as_str()),
        ),
        FormField::new(
            EditorField::Name,
            Line::from("Name"),
            Line::from(name.as_str()),
        )
        .required(true),
        FormField::new(
            EditorField::Group,
            Line::from("Group"),
            Line::from(group.as_str()),
        ),
        FormField::new(
            EditorField::Environment,
            Line::from("Environment"),
            Line::from(environment.as_str()),
        ),
    ];
    let connection = [
        FormField::new(
            EditorField::Host,
            Line::from("Host"),
            Line::from(host.as_str()),
        )
        .required(true),
        FormField::new(
            EditorField::Port,
            Line::from("Port"),
            Line::from(port.as_str()),
        )
        .required(true),
        FormField::new(
            EditorField::Database,
            Line::from("Database"),
            Line::from(database.as_str()),
        ),
    ];
    let credentials = [
        FormField::new(
            EditorField::Username,
            Line::from("Username"),
            Line::from(username.as_str()),
        ),
        FormField::new(
            EditorField::Password,
            Line::from("Password"),
            Line::from(password.as_str()),
        ),
        FormField::new(
            EditorField::PasswordSource,
            Line::from("Password source"),
            Line::from(password_source.as_str()),
        ),
    ];
    let tls = [FormField::new(
        EditorField::TlsMode,
        Line::from("TLS mode"),
        Line::from(tls_mode.as_str()),
    )];
    let ssh = [
        FormField::new(
            EditorField::SshHost,
            Line::from("SSH bastion"),
            Line::from(ssh_host.as_str()),
        ),
        FormField::new(
            EditorField::SshPort,
            Line::from("SSH port"),
            Line::from(ssh_port.as_str()),
        ),
        FormField::new(
            EditorField::SshUsername,
            Line::from("SSH user"),
            Line::from(ssh_username.as_str()),
        ),
        FormField::new(
            EditorField::SshPassword,
            Line::from("SSH password/passphrase"),
            Line::from(ssh_password.as_str()),
        ),
        FormField::new(
            EditorField::SshPrivateKey,
            Line::from("SSH private key"),
            Line::from(ssh_private_key.as_str()),
        ),
        FormField::new(
            EditorField::SshKnownHostsPath,
            Line::from("known_hosts path"),
            Line::from(ssh_known_hosts.as_str()),
        ),
        FormField::new(
            EditorField::SshUseAgent,
            Line::from("SSH auth mode"),
            Line::from(ssh_use_agent.as_str()),
        ),
    ];
    let startup = [FormField::new(
        EditorField::StartupSql,
        Line::from("Startup SQL (!write/!danger prefixes)"),
        Line::from(startup_sql.as_str()),
    )];
    let sections = [
        FormSection {
            title: Line::from("General"),
            fields: &general,
        },
        FormSection {
            title: Line::from("Connection"),
            fields: &connection,
        },
        FormSection {
            title: Line::from("Credentials"),
            fields: &credentials,
        },
        FormSection {
            title: Line::from("TLS"),
            fields: &tls,
        },
        FormSection {
            title: Line::from("SSH tunnel"),
            fields: &ssh,
        },
        FormSection {
            title: Line::from("Startup actions"),
            fields: &startup,
        },
    ];
    let mut state = FormState::new(Some(editor.focused));
    state.set_active(model.focus() == Some(FocusRegion::Content));
    frame.render_stateful_widget(&Form::new(&sections, &model.theme), form_area, &mut state);
}

fn render_workbench_facts(model: &Model, frame: &mut Frame<'_>, area: Rect, _status: Option<&str>) {
    use ratatui_core::widgets::Widget;
    let wb = model.workbench();
    let mut y = area.y;
    let max_y = area.y.saturating_add(area.height);
    let tab_line = wb
        .tabs
        .get(wb.selected_tab)
        .map(|tab| {
            format!(
                "tab: {}{}{}",
                tab.title,
                if tab.dirty { " *" } else { "" },
                if tab.running { " …" } else { "" }
            )
        })
        .unwrap_or_else(|| "tab: —".into());
    let editor_status = wb
        .active_editor()
        .map(|ed| ed.status_line())
        .unwrap_or_default();
    let history_status = wb.history.status_line();
    let header = [
        wb.context.line(),
        tab_line,
        editor_status,
        history_status,
        wb.active_grid()
            .map(|g| g.status_line())
            .unwrap_or_else(|| wb.status.summary()),
        wb.mutation_review
            .as_ref()
            .map(|r| {
                let n = r.lines.len();
                format!(
                    "review {}.{} · {n} stmt(s) · first: {}",
                    r.schema,
                    r.table,
                    r.lines
                        .first()
                        .map(|l| l.sql.as_str())
                        .unwrap_or("—")
                )
            })
            .unwrap_or_default(),
        wb.active_grid()
            .and_then(|g| g.cell_edit.as_ref())
            .map(|e| {
                format!(
                    "editing {}.{} = [{}] (paste value, Activate to stage)",
                    e.abs_row, e.column, e.buffer
                )
            })
            .unwrap_or_default(),
    ];
    for line in header {
        if y >= max_y || line.is_empty() {
            continue;
        }
        let clipped: String = line.chars().take(area.width as usize).collect();
        Line::from(clipped).render(
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        y = y.saturating_add(1);
    }
    let body = Rect {
        x: area.x,
        y,
        width: area.width,
        height: max_y.saturating_sub(y),
    };
    if body.height == 0 {
        return;
    }

    // SQL tab: editor above results with remembered split percent.
    if let Some(editor) = wb.active_editor() {
        let editor_h = ((u32::from(body.height) * u32::from(editor.split_editor_percent())) / 100)
            .clamp(2, u32::from(body.height.saturating_sub(2)).max(2))
            as u16;
        let editor_area = Rect {
            x: body.x,
            y: body.y,
            width: body.width,
            height: editor_h.min(body.height),
        };
        let results_area = Rect {
            x: body.x,
            y: body.y.saturating_add(editor_area.height),
            width: body.width,
            height: body.height.saturating_sub(editor_area.height),
        };
        let mut ta = TextAreaState::new(editor.text());
        ta.set_focused(editor.focused() && model.focus() == Some(FocusRegion::Content));
        // Approximate cursor: place at end when offset matches text length.
        if editor.cursor() < editor.text().len() {
            // Leave default end-of-document placement when we cannot map cheaply.
            let _ = editor.cursor();
        }
        frame.render_stateful_widget(
            &TextArea::new(&model.theme).title("SQL"),
            editor_area,
            &mut ta,
        );
        if let Some(session) = wb.completion.as_ref() {
            let owned: Vec<(String, String, String)> = session
                .candidates
                .iter()
                .map(|c| (c.id.clone(), c.label.clone(), c.kind.clone()))
                .collect();
            let candidates: Vec<CompletionCandidate<'_, String>> = owned
                .iter()
                .map(|(id, label, kind)| {
                    CompletionCandidate::new(id.clone(), label.as_str()).kind(kind.as_str())
                })
                .collect();
            let anchor = Rect {
                x: editor_area.x.saturating_add(2),
                y: editor_area.y.saturating_add(1),
                width: 1,
                height: 1,
            };
            let mut menu_state =
                CompletionMenuState::new(session.selected_id.clone());
            frame.render_stateful_widget(
                &CompletionMenu::new(&candidates, &model.theme, editor_area, anchor)
                    .preferred_size(CompletionMenuSize {
                        width: 36,
                        height: 8,
                    }),
                editor_area,
                &mut menu_state,
            );
        }
        if results_area.height > 0 {
            if let Some(grid) = wb.active_grid() {
                render_data_grid(model, frame, results_area, grid);
            }
        }
        return;
    }

    let grid_area = body;
    if grid_area.height > 1 {
        let insp_lines = wb.inspector.lines();
        let insp_h = if insp_lines.is_empty() {
            0
        } else {
            (insp_lines.len() as u16).min(grid_area.height / 3).max(1)
        };
        let sort_bar = wb.active_grid().and_then(|g| g.sort_chip_bar());
        let filter_bar = wb
            .active_grid()
            .and_then(|g| g.filter_chip_bar());
        let sort_h = u16::from(sort_bar.is_some());
        let filter_h = u16::from(filter_bar.is_some());
        let control_h = sort_h.saturating_add(filter_h);
        let grid_h = grid_area
            .height
            .saturating_sub(insp_h)
            .saturating_sub(control_h);
        let mut bar_y = grid_area.y;
        if let (Some(bar), true) = (sort_bar.as_deref(), sort_h > 0) {
            let clipped: String = bar.chars().take(grid_area.width as usize).collect();
            Line::from(format!("⇅ {clipped}")).render(
                Rect {
                    x: grid_area.x,
                    y: bar_y,
                    width: grid_area.width,
                    height: 1,
                },
                frame.buffer_mut(),
            );
            bar_y = bar_y.saturating_add(1);
        }
        if let (Some(bar), true) = (filter_bar.as_deref(), filter_h > 0) {
            let clipped: String = bar.chars().take(grid_area.width as usize).collect();
            Line::from(format!("▣ {clipped}")).render(
                Rect {
                    x: grid_area.x,
                    y: bar_y,
                    width: grid_area.width,
                    height: 1,
                },
                frame.buffer_mut(),
            );
        }
        if grid_h > 0 {
            if let Some(grid) = wb.active_grid() {
                render_data_grid(
                    model,
                    frame,
                    Rect {
                        x: grid_area.x,
                        y: grid_area.y.saturating_add(control_h),
                        width: grid_area.width,
                        height: grid_h,
                    },
                    grid,
                );
            }
        }
        if insp_h > 0 {
            let mut iy = grid_area.y.saturating_add(grid_h);
            for line in insp_lines {
                if iy >= grid_area.y.saturating_add(grid_area.height) {
                    break;
                }
                let clipped: String = line.chars().take(grid_area.width as usize).collect();
                Line::from(clipped).render(
                    Rect {
                        x: grid_area.x,
                        y: iy,
                        width: grid_area.width,
                        height: 1,
                    },
                    frame.buffer_mut(),
                );
                iy = iy.saturating_add(1);
            }
        }
    }
}

fn render_data_grid(
    model: &Model,
    frame: &mut Frame<'_>,
    area: Rect,
    grid: &crate::model::grid::DataGridModel,
) {
    use ratatui_core::widgets::Widget;
    if grid.columns.is_empty() {
        Line::from("(no result)").render(area, frame.buffer_mut());
        return;
    }
    // Display order + visibility from column_layout (physical matrix order unchanged).
    let visible = grid.visible_columns();
    if visible.is_empty() {
        Line::from("(no visible columns)").render(area, frame.buffer_mut());
        return;
    }
    let columns: Vec<GridColumn<'_, usize>> = visible
        .iter()
        .enumerate()
        .map(|(i, name)| GridColumn::fixed(i, name.as_str(), grid.column_width(name)))
        .collect();
    let body_rows = u64::from(area.height.saturating_sub(1).max(1));
    let first = grid.viewport_row.max(grid.start_row);
    let mut owned_rows: Vec<Vec<String>> = Vec::new();
    let mut row_abs: Vec<u64> = Vec::new();
    // Reserve space for staged insert drafts at the foot of the viewport.
    let insert_slots = grid.drafts.inserts.len().min(body_rows as usize) as u64;
    let resident_slots = body_rows.saturating_sub(insert_slots);
    for slot in 0..resident_slots {
        let abs = first.saturating_add(slot);
        let mut texts = Vec::with_capacity(visible.len());
        for name in &visible {
            let col = grid.physical_column_index(name).unwrap_or(0);
            // Staged overlays + draft markers (never color alone).
            texts.push(grid.cell_display_at(abs, col));
        }
        owned_rows.push(texts);
        row_abs.push(abs);
    }
    // Paint staged inserts as synthetic + rows (presentation only; not resident).
    // Use high abs keys so they never collide with real page rows.
    for (i, insert) in grid.drafts.inserts.iter().enumerate().take(insert_slots as usize) {
        if let Some(texts) = grid.insert_row_display(insert.draft_id, &visible) {
            owned_rows.push(texts);
            // Synthetic absolute keys: top of u64 range − draft_id.
            row_abs.push(u64::MAX.saturating_sub(insert.draft_id).saturating_sub(i as u64));
        }
    }
    let mut cell_bufs: Vec<Vec<GridCell<'_>>> = Vec::new();
    for texts in &owned_rows {
        cell_bufs.push(
            texts
                .iter()
                .map(|t| {
                    if t == "…" || t.starts_with('…') {
                        GridCell::pending()
                    } else {
                        GridCell::text(t.as_str())
                    }
                })
                .collect(),
        );
    }
    let rows: Vec<GridRow<'_, u64>> = cell_bufs
        .iter()
        .enumerate()
        .map(|(i, cells)| {
            let abs = row_abs.get(i).copied().unwrap_or(first.saturating_add(i as u64));
            GridRow::new(abs, abs, cells.as_slice())
        })
        .collect();
    let total = match grid.totals {
        crate::model::grid::GridRowTotal::Exact(n)
        | crate::model::grid::GridRowTotal::Estimated(n) => {
            n.max(grid.rows_loaded)
                .saturating_add(grid.drafts.inserts.len() as u64)
        }
        crate::model::grid::GridRowTotal::Unknown => grid
            .start_row
            .saturating_add(u64::from(grid.row_count))
            .saturating_add(body_rows)
            .saturating_add(grid.drafts.inserts.len() as u64),
    };
    let mut state = VirtualGridState::new();
    state.set_focused(model.focus() == Some(FocusRegion::Content));
    let widget = VirtualGrid::new(&columns, &rows, &model.theme)
        .total_rows(total.max(1))
        .gutter(true)
        .header(true);
    frame.render_stateful_widget(&widget, area, &mut state);
}
