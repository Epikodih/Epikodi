//! Conversions entités du domaine → messages du protocole.

use epikodi_domain::device::Device;
use epikodi_domain::extension::{Extension, Permission};
use epikodi_domain::media::Media;
use epikodi_domain::playback::{resume_position, WatchProgress};
use epikodi_domain::playlist::Playlist;
use epikodi_domain::profile::{ActiveProfile, Profile};
use epikodi_domain::protocol::{
    DeviceSummary, DeviceView, ExtensionView, GuestInfo, MediaItem, PermissionView, PlaylistView,
    ProfileDetail, ProfileKind, ProfileListItem, ProfileView, ProgressView, SessionDevice,
    SourceView,
};
use epikodi_domain::source::Source;

pub const GUEST_NAME: &str = "Invité";

pub fn media_item(m: &Media) -> MediaItem {
    MediaItem {
        id: m.id,
        parent_id: m.parent_id,
        kind: m.kind,
        title: m.title.clone(),
        year: m.year,
        poster_url: m
            .poster_path
            .as_ref()
            .map(|_| format!("/api/media/{}/poster", m.id)),
        duration_s: m.duration_s,
        sort_number: m.sort_number,
        age_rating: m.age_rating,
        match_status: m.match_status,
        status: m.status,
    }
}

pub fn media_items(list: &[Media]) -> Vec<MediaItem> {
    list.iter().map(media_item).collect()
}

pub fn account_view(p: &Profile) -> ProfileView {
    ProfileView {
        kind: ProfileKind::Account,
        id: Some(p.id),
        name: p.name.clone(),
        icon: p.icon.clone(),
        language: p.language.clone(),
        age: p.age,
        is_restricted: p.is_restricted,
    }
}

pub fn guest_view() -> ProfileView {
    ProfileView {
        kind: ProfileKind::Guest,
        id: None,
        name: GUEST_NAME.into(),
        icon: None,
        language: "fr".into(),
        age: 0,
        is_restricted: true,
    }
}

pub fn profile_view(p: &ActiveProfile) -> ProfileView {
    match p {
        ActiveProfile::Guest => guest_view(),
        ActiveProfile::Account(a) => account_view(a),
    }
}

pub fn profile_detail(p: &Profile) -> ProfileDetail {
    ProfileDetail {
        profile: account_view(p),
        requires_pin: p.requires_pin(),
    }
}

pub fn profile_list_item(p: &Profile) -> ProfileListItem {
    ProfileListItem {
        id: p.id,
        name: p.name.clone(),
        icon: p.icon.clone(),
        language: p.language.clone(),
        age: p.age,
        is_restricted: p.is_restricted,
        requires_pin: p.requires_pin(),
    }
}

pub fn guest_info() -> GuestInfo {
    GuestInfo {
        available: true,
        is_restricted: true,
    }
}

pub fn device_summary(d: &Device) -> DeviceSummary {
    DeviceSummary {
        id: d.id,
        name: d.name.clone(),
    }
}

pub fn session_device(d: &Device) -> SessionDevice {
    SessionDevice {
        id: d.id,
        name: d.name.clone(),
        paired_at: d.paired_at.clone(),
    }
}

pub fn device_view(d: &Device, current: Option<i64>) -> DeviceView {
    DeviceView {
        id: d.id,
        name: d.name.clone(),
        platform: d.platform.clone(),
        paired_at: d.paired_at.clone(),
        expires_at: d.expires_at.clone(),
        last_seen_at: d.last_seen_at.clone(),
        current: current == Some(d.id),
    }
}

pub fn progress_view(p: &WatchProgress, duration_s: Option<i64>) -> ProgressView {
    ProgressView {
        position_s: p.position_s,
        is_watched: p.is_watched,
        last_played_at: p.last_played_at.clone(),
        resume_at_s: resume_position(Some(p), duration_s),
    }
}

pub fn playlist_view(p: &Playlist) -> PlaylistView {
    PlaylistView {
        id: p.id,
        name: p.name.clone(),
        item_count: p.item_count,
    }
}

pub fn source_view(s: &Source) -> SourceView {
    SourceView {
        id: s.id,
        name: s.name.clone(),
        protocol: s.protocol,
        uri: s.uri.clone(),
        has_credentials: s.has_credentials,
        status: s.status,
        last_scan_at: s.last_scan_at.clone(),
    }
}

pub fn permission_views(
    requested: &[Permission],
    granted: impl Fn(Permission) -> bool,
) -> Vec<PermissionView> {
    requested
        .iter()
        .map(|p| PermissionView {
            id: *p,
            description: p.description().into(),
            granted: granted(*p),
        })
        .collect()
}

pub fn extension_view(e: &Extension) -> ExtensionView {
    let requested: Vec<Permission> = e.permissions.0.keys().copied().collect();
    ExtensionView {
        id: e.id.clone(),
        name: e.name.clone(),
        version: e.version.clone(),
        kind: e.kind,
        enabled: e.enabled,
        status: e.status,
        last_error: e.last_error.clone(),
        permissions: permission_views(&requested, |p| e.permissions.allows(p)),
        settings: e.settings.clone(),
    }
}
