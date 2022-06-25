use crate::{
    core::{
        config::Config,
        hooks::HookName,
        manager::{event::EventAction, state::WmState, util::pad_region},
        xconnection::{XClientConfig, XClientHandler, Xid},
    },
    Result,
};

#[tracing::instrument(level = "trace", err, skip(conn))]
pub(super) fn layout_visible<X>(state: &mut WmState, conn: &X) -> Result<Vec<EventAction>>
where
    X: XClientHandler + XClientConfig,
{
    state
        .screens
        .visible_workspaces()
        .into_iter()
        .flat_map(|wix| apply_layout(state, conn, wix).transpose())
        .collect()
}

#[tracing::instrument(level = "debug", err, skip(conn))]
pub(super) fn apply_layout<X>(
    state: &mut WmState,
    conn: &X,
    wix: usize,
) -> Result<Option<EventAction>>
where
    X: XClientHandler + XClientConfig,
{
    let (i, s) = match state.screens.indexed_screen_for_workspace(wix) {
        Some((i, s)) => (i, s),
        None => return Ok(None),
    };

    let Config {
        show_bar,
        mut border_px,
        mut gap_px,
        ..
    } = state.config;
    let float_border_px = border_px;

    let (lc, aa) = state.workspaces.get_arrange_actions(
        wix,
        s.region(show_bar),
        &state
            .clients
            .clients_for_ids(&state.workspaces[wix].client_ids()),
    )?;

    if state.workspaces.get_workspace(wix)?.len() <= aa.floating.len() + 1 {
        if lc.smart_borders {
            border_px = 0;
        }
        if lc.smart_gaps {
            gap_px = 0;
        }
    }

    for (id, region) in aa.actions {
        trace!(id, ?region, "positioning client");
        if let Some(region) = region {
            let mut border_px = border_px;
            if aa.floating.contains(&id) {
                border_px = float_border_px;
            };
            let reg = pad_region(&region, lc.gapless, gap_px, border_px);
            conn.position_client(id, reg, border_px, false)?;
            state.clients.map_if_needed(id, conn)?;
        } else {
            state.clients.unmap_if_needed(id, conn)?;
        }
    }

    for id in aa.floating {
        debug!(id, "mapping floating client above tiled");
        conn.raise_client(id)?;
    }

    Ok(Some(EventAction::RunHook(HookName::LayoutApplied(wix, i))))
}
