use core::cmp::Ordering;
use std::sync::Arc;

use puffin::{NanoSecond, Reader, ScopeCollection, ScopeId, ThreadInfo, UnpackedFrameData};

pub struct GroupedStats<'a> {
    pub key: Key<'a>,
    pub scope_stats: ScopeStats,
    pub children: Vec<GroupedStats<'a>>,
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key<'a> {
    pub id: ScopeId,
    pub thread_name: &'a str,
}

#[derive(Copy, Clone, Default)]
pub struct ScopeStats {
    pub count: usize,
    pub bytes: usize,
    /// Time covered by all scopes, minus those covered by child scopes.
    /// A lot of time == useful scope.
    pub total_self_ns: NanoSecond,
    pub total_ns: NanoSecond,
    /// Time covered by the slowest scope, minus those covered by child scopes.
    /// A lot of time == useful scope.
    pub max_ns: NanoSecond,
}

pub struct StatsTotals {
    pub bytes: usize,
    pub ns: i64,
    pub scopes: usize,
    pub num_threads: usize,
}

pub fn process_scopes<'a>(
    scope_infos: &ScopeCollection,
    frames: &'a [Arc<UnpackedFrameData>],
    options: &super::Options,
) -> (Vec<GroupedStats<'a>>, StatsTotals) {
    let mut threads = std::collections::HashSet::<&ThreadInfo>::new();
    let mut scopes = vec![];

    for frame in frames {
        threads.extend(frame.thread_streams.keys());
        for (thread_info, stream) in &frame.thread_streams {
            collect_stream(
                &mut scopes,
                &thread_info.name,
                &stream.stream,
                options.tree_view,
            )
            .ok();
        }
    }

    let mut totals = stats_totals(&scopes);
    totals.num_threads = threads.len();

    sort_grouped_stats(&mut scopes, &|a, b| {
        let ord = options.sort_by.grouped_stats_ordering(scope_infos, a, b);
        if options.sort_asc {
            ord
        } else {
            ord.reverse()
        }
    });

    (scopes, totals)
}
fn collect_stream<'s>(
    stats: &mut Vec<GroupedStats<'s>>,
    thread_name: &'s str,
    stream: &'s puffin::Stream,
    tree_view: bool,
) -> puffin::Result<()> {
    for scope in puffin::Reader::from_start(stream) {
        collect_scope(stats, thread_name, stream, &scope?, tree_view)?;
    }
    Ok(())
}

fn stats_totals(stats: &Vec<GroupedStats>) -> StatsTotals {
    let mut result = StatsTotals {
        bytes: 0,
        ns: 0,
        scopes: 0,
        num_threads: 0,
    };
    for stat in stats {
        result.bytes += stat.scope_stats.bytes;
        result.ns += stat.scope_stats.total_self_ns;
        result.scopes += 1;
        let children_result = stats_totals(&stat.children);
        result.bytes += children_result.bytes;
        result.ns += children_result.ns;
        result.scopes += children_result.scopes;
    }
    result
}
fn sort_grouped_stats(
    stats: &mut Vec<GroupedStats>,
    compare: &impl Fn(&GroupedStats, &GroupedStats) -> Ordering,
) {
    stats.sort_by(compare);
    for stat in stats {
        sort_grouped_stats(&mut stat.children, compare);
    }
}

fn scope_byte_size(scope: &puffin::Scope<'_>) -> usize {
    1 + // `(` sentinel
    8 + // start time
    8 + // scope id
    1 + scope.record.data.len() + // dynamic data len
    8 + // scope size
    1 + // `)` sentinel
    8 // stop time
}
fn collect_scope<'s>(
    stats: &mut Vec<GroupedStats<'s>>,
    thread_name: &'s str,
    stream: &'s puffin::Stream,
    scope: &puffin::Scope<'s>,
    tree_view: bool,
) -> puffin::Result<()> {
    let mut ns_used_by_children = 0;

    let key = Key {
        id: scope.id,
        thread_name,
    };
    let entry_index = if let Some(e) = stats.iter().position(|sstat| sstat.key == key) {
        e
    } else {
        let index = stats.len();
        stats.push(GroupedStats {
            key,
            scope_stats: ScopeStats::default(),
            children: vec![],
        });
        index
    };
    for child_scope in Reader::with_offset(stream, scope.child_begin_position)? {
        let child_scope = &child_scope?;
        collect_scope(
            if tree_view {
                &mut stats[entry_index].children
            } else {
                stats
            },
            thread_name,
            stream,
            child_scope,
            tree_view,
        )?;
        ns_used_by_children += child_scope.record.duration_ns;
    }

    let self_time = scope.record.duration_ns.saturating_sub(ns_used_by_children);
    let entry = &mut stats[entry_index];
    entry.scope_stats.count += 1;
    entry.scope_stats.bytes += scope_byte_size(scope);
    entry.scope_stats.total_self_ns += self_time;
    entry.scope_stats.total_ns += scope.record.duration_ns;
    entry.scope_stats.max_ns = entry.scope_stats.max_ns.max(self_time);

    Ok(())
}
