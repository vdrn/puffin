// TODO: first column disabled is bugged
use core::cmp::Ordering;
use std::collections::HashMap;

use egui::{util::hash, Checkbox, RichText, Ui, Widget, WidgetText};

use puffin::{ScopeCollection, ScopeDetails, UnpackedFrameData};

use crate::filter::Filter;

mod process_scopes;
use process_scopes::{GroupedStats, Key, ScopeStats};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StatsColumnId {
    Thread,
    Location,
    ScopeName,
    ID,
    Count,
    Size,
    #[default]
    TotalSelfTime,
    MeanSelfTime,
    MaxSelfTime,
    TotalTime,
    MeanTime,
}

impl StatsColumnId {
    pub fn title(self) -> &'static str {
        match self {
            StatsColumnId::Thread => "Thread",
            StatsColumnId::Location => "Location",
            StatsColumnId::ScopeName => "Scope name",
            StatsColumnId::ID => "ID",
            StatsColumnId::Count => "Count",
            StatsColumnId::Size => "Size",
            StatsColumnId::TotalSelfTime => "Total self time",
            StatsColumnId::MeanSelfTime => "Mean self time",
            StatsColumnId::MaxSelfTime => "Max self time",
            StatsColumnId::TotalTime => "Total time",
            StatsColumnId::MeanTime => "Mean time",
        }
    }
    pub fn grouped_stats_ordering(
        self,
        scope_infos: &ScopeCollection,
        a: &GroupedStats,
        b: &GroupedStats,
    ) -> Ordering {
        match self {
            StatsColumnId::Thread => a.key.thread_name.cmp(&b.key.thread_name),
            StatsColumnId::Location => {
                if let (Some(ai), Some(bi)) = (
                    scope_infos.fetch_by_id(&a.key.id),
                    scope_infos.fetch_by_id(&b.key.id),
                ) {
                    ai.location().cmp(&bi.location())
                } else {
                    Ordering::Equal
                }
            }
            StatsColumnId::ScopeName => {
                if let (Some(ai), Some(bi)) = (
                    scope_infos.fetch_by_id(&a.key.id),
                    scope_infos.fetch_by_id(&b.key.id),
                ) {
                    ai.name().cmp(bi.name())
                } else {
                    Ordering::Equal
                }
            }
            StatsColumnId::ID => a.key.id.0.cmp(&b.key.id.0),
            StatsColumnId::Count => a.scope_stats.count.cmp(&b.scope_stats.count),
            StatsColumnId::Size => a.scope_stats.bytes.cmp(&b.scope_stats.bytes),
            StatsColumnId::TotalSelfTime => a
                .scope_stats
                .total_self_ns
                .cmp(&b.scope_stats.total_self_ns),
            StatsColumnId::MeanSelfTime => (a.scope_stats.total_self_ns as f32
                / a.scope_stats.count as f32)
                .partial_cmp(&(b.scope_stats.total_self_ns as f32 / b.scope_stats.count as f32))
                .unwrap_or(Ordering::Equal),
            StatsColumnId::MaxSelfTime => a.scope_stats.max_ns.cmp(&b.scope_stats.max_ns),
            StatsColumnId::TotalTime => a.scope_stats.total_ns.cmp(&b.scope_stats.total_ns),
            StatsColumnId::MeanTime => (a.scope_stats.total_ns as f32 / a.scope_stats.count as f32)
                .partial_cmp(&(b.scope_stats.total_ns as f32 / b.scope_stats.count as f32))
                .unwrap_or(Ordering::Equal),
        }
    }
}
pub fn draw_column_header(ui: &mut Ui, options: &mut Options, column: StatsColumnId) {
    let is_sort_col = column == options.sort_by;
    let column_label = format!(
        "{} {}",
        column.title(),
        if !is_sort_col {
            ""
        } else if options.sort_asc {
            "▲"
        } else {
            "▼"
        }
    );
    if ui
        .button(WidgetText::RichText(column_label.into()).monospace())
        .clicked()
    {
        if is_sort_col {
            options.sort_asc = !options.sort_asc;
        } else {
            options.sort_by = column;
            options.sort_asc = false;
        }
    }
}
pub fn draw_column_data(
    ui: &mut Ui,
    column_id: StatsColumnId,
    key: &Key,
    scope_details: &ScopeDetails,
    stats: &ScopeStats,
) {
    match column_id {
        StatsColumnId::Thread => ui.label(key.thread_name.to_owned()),
        StatsColumnId::Location => ui.label(scope_details.location()),
        StatsColumnId::ScopeName => ui.label(scope_details.name().to_string()),
        StatsColumnId::ID => ui.label(key.id.0.to_string()),
        StatsColumnId::Count => ui.monospace(format!("{:>5}", stats.count)),
        StatsColumnId::Size => ui.monospace(format!("{:>6.1} kB", stats.bytes as f32 * 1e-3)),
        StatsColumnId::TotalSelfTime => {
            ui.monospace(format!("{:>8.1} µs", stats.total_self_ns as f32 * 1e-3))
        }
        StatsColumnId::MeanSelfTime => ui.monospace(format!(
            "{:>8.1} µs",
            stats.total_self_ns as f32 * 1e-3 / (stats.count as f32)
        )),
        StatsColumnId::MaxSelfTime => {
            ui.monospace(format!("{:>8.1} µs", stats.max_ns as f32 * 1e-3))
        }
        StatsColumnId::TotalTime => {
            ui.monospace(format!("{:>8.1} µs", stats.total_ns as f32 * 1e-3))
        }
        StatsColumnId::MeanTime => ui.monospace(format!(
            "{:>8.1} µs",
            stats.total_ns as f32 * 1e-3 / (stats.count as f32)
        )),
    };
}

// #[derive(Clone, Debug, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// pub enum StatsTableType {
//     CollapsableHeader,
//     Grid,
// }

#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct StatsColumn {
    id: StatsColumnId,
    enabled: bool,
}
impl StatsColumn {
    pub fn new(id: StatsColumnId, enabled: bool) -> Self {
        Self { id, enabled }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Options {
    filter: Filter,
    sort_by: StatsColumnId,
    sort_asc: bool,

    columns: Vec<StatsColumn>,

    tree_view: bool,
    tree_view_state: HashMap<u64, bool>,

    // table_type: StatsTableType,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            tree_view_state: HashMap::default(),
            tree_view: false,
            filter: Default::default(),
            sort_by: Default::default(),
            sort_asc: Default::default(),
            // table_type: StatsTableType::Grid,
            columns: vec![
                StatsColumn::new(StatsColumnId::ID, true),
                StatsColumn::new(StatsColumnId::Thread, false),
                StatsColumn::new(StatsColumnId::Location, true),
                StatsColumn::new(StatsColumnId::ScopeName, true),
                StatsColumn::new(StatsColumnId::Count, true),
                StatsColumn::new(StatsColumnId::TotalSelfTime, true),
                StatsColumn::new(StatsColumnId::MeanSelfTime, true),
                StatsColumn::new(StatsColumnId::MaxSelfTime, true),
                StatsColumn::new(StatsColumnId::TotalTime, true),
                StatsColumn::new(StatsColumnId::MeanTime, true),
                StatsColumn::new(StatsColumnId::Size, true),
            ],
        }
    }
}

pub fn ui(
    ui: &mut egui::Ui,
    options: &mut Options,
    scope_infos: &ScopeCollection,
    frames: &[std::sync::Arc<UnpackedFrameData>],
) {
    crate::profile_function!();

    let (scopes, totals) = process_scopes::process_scopes(scope_infos, frames, options);

    ui.label("This view can be used to find functions that are called a lot.\n\
              The overhead of a profile scope is around ~50ns, so remove profile scopes from fast functions that are called often.");
    ui.label(format!(
        "Current frame: {} unique scopes, using a total of {:.1} kB, covering {:.1} ms over {} thread(s)",
        totals.scopes,
        totals.bytes as f32 * 1e-3,
        totals.ns as f32 * 1e-6,
        totals.num_threads
    ));

    ui.separator();

    ui.horizontal(|ui| {
        options.filter.ui(ui);

        ui.separator();

        ui.menu_button("Set columns", |ui| {
            for i in 0..options.columns.len() {
                ui.horizontal(|ui| {
                    let checkbox_lbl = options.columns[i].id.title();
                    ui.checkbox(&mut options.columns[i].enabled, checkbox_lbl);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        // NOTE: these swaps will cause one menu option to be drawn twice, and other
                        // zero times.
                        // But just for one frame so it's fine?
                        if i != 0 {
                            if ui.button(RichText::new("▲").monospace()).clicked() {
                                options.columns.swap(i, i - 1);
                            }
                        }
                        if i != options.columns.len() - 1 {
                            if ui.button(RichText::new("▼").monospace()).clicked() {
                                options.columns.swap(i, i + 1);
                            }
                        }
                    });
                });
            }
        });

        ui.separator();

        ui.label("Tree-view: ");
        Checkbox::new(&mut options.tree_view, "").ui(ui);

        // if options.tree_view {
        //     ui.separator();
        //     ui.label("Table type: ");
        //     egui::ComboBox::new(hash("table type selectbox"), "")
        //         .selected_text(format!("{:?}", options.table_type))
        //         .show_ui(ui, |ui| {
        //             ui.selectable_value(&mut options.table_type, StatsTableType::Grid, "Grid");
        //             ui.selectable_value(
        //                 &mut options.table_type,
        //                 StatsTableType::CollapsableHeader,
        //                 "Collapsable headers",
        //             );
        //         });
        // }
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let columns = options
            .columns
            .iter()
            .filter_map(|col| if col.enabled { Some(col.id) } else { None })
            .collect::<Vec<_>>();

        // if !options.tree_view || options.table_type == StatsTableType::Grid {
            egui::Grid::new("table")
                .striped(true)
                .spacing([32.0, ui.spacing().item_spacing.y])
                .show(ui, |ui| {
                    for &column in &columns {
                        draw_column_header(ui, options, column);
                    }
                    ui.end_row();

                    draw_grid_rows(
                        scope_infos,
                        0,
                        String::new(),
                        &scopes,
                        options,
                        &columns,
                        ui,
                    );
                });
        // } else {
        //     ui.columns(columns.len(), |ui| {
        //         for (i, column) in columns.iter().enumerate() {
        //             draw_column_header(&mut ui[i], options, *column);
        //         }
        //     });

        //     draw_collapsable_rows(scope_infos, &columns, &scopes, options, ui);
        // }
    });
}

// fn get_scope_details(
//     scope_infos: &ScopeCollection,
//     filter: &Filter,
//     stat: GroupedStats,
// ) -> Option<&ScopeDetails> {
//     let Some(scope_details) = scope_infos.fetch_by_id(&stat.key.id) else {
//         return None;
//     };
//     if filter.include(stat.key.thread_name) {
//         return Some(scope_details);
//     }
// }
fn draw_grid_rows(
    scope_infos: &ScopeCollection,
    level: usize,
    tree_string: String,
    scopes: &[GroupedStats],
    options: &mut Options,
    columns: &[StatsColumnId],
    ui: &mut Ui,
) {
    for (i, stat) in scopes.iter().enumerate() {
        let is_last = i == scopes.len() - 1;

        let Some(scope_details) = scope_infos.fetch_by_id(&stat.key.id) else {
            continue;
        };

        let mut draw_children = false;
        if options.filter.include(&stat.key.thread_name)
            || options.filter.include(&scope_details.location())
            || options.filter.include(&scope_details.name())
        {
            ui.horizontal(|ui| {
                let tree_glyph = if level == 0 {
                    ""
                } else if is_last {
                    " └╴"
                } else {
                    " ├╴"
                };

                ui.label(RichText::new(&format!("{}{}", tree_string, tree_glyph)).monospace());
                if !stat.children.is_empty() {
                    let expanded = options
                        .tree_view_state
                        .entry(hash(&stat.key))
                        .or_insert(true);

                    if ui.button(if *expanded { "-" } else { "+" }).clicked() {
                        *expanded = !*expanded;
                    }
                    draw_children = *expanded;
                }

                if let Some(first) = options.columns.first() {
                    draw_column_data(ui, first.id, &stat.key, scope_details, &stat.scope_stats);
                }
            });
            for col in options.columns.iter().skip(1).filter(|col| col.enabled) {
                draw_column_data(ui, col.id, &stat.key, &scope_details, &stat.scope_stats);
            }
            ui.end_row();
        } else {
            draw_children = true;
        }

        if draw_children {
            let tree_glyph = if level == 0 {
                ""
            } else if is_last {
                "   "
            } else {
                " | "
            };

            draw_grid_rows(
                scope_infos,
                level + 1,
                format!("{}{}", tree_string, tree_glyph),
                &stat.children,
                options,
                columns,
                ui,
            );
        }
    }
}
// fn draw_collapsable_rows(
//     scope_infos: &ScopeCollection,

//     columns: &[StatsColumnId],
//     scopes: &[GroupedStats],
//     options: &mut Options,
//     ui: &mut Ui, // ui:&mut Ui,
// ) {
//     for stat in scopes.iter() {
//         if !options.filter.include(&stat.key.id.0.to_string()) {
//             continue;
//         }
//         let Some(scope_details) = scope_infos.fetch_by_id(&stat.key.id) else {
//             continue;
//         };

//         let id = ui.make_persistent_id(&stat.key);

//         if !stat.children.is_empty() {
//             egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
//                 .show_header(ui, |ui| {
//                     data_row(ui, &scope_details, columns, stat);
//                 })
//                 .body(|ui| {
//                     draw_collapsable_rows(scope_infos, columns, &stat.children, options, ui);
//                 });
//         } else {
//             data_row(ui, &scope_details, columns, stat);
//         }
//     }
// }
// fn data_row(
//     ui: &mut Ui,
//     scope_details: &ScopeDetails,
//     columns: &[StatsColumnId],
//     stat: &GroupedStats,
// ) {
//     ui.columns(columns.len(), |ui| {
//         for (i, column_id) in columns.iter().enumerate() {
//             draw_column_data(
//                 &mut ui[i],
//                 *column_id,
//                 &stat.key,
//                 scope_details,
//                 &stat.scope_stats,
//             );
//         }
//     });
// }
