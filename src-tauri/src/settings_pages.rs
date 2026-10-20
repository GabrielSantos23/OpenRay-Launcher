use crate::apps::InstalledApp;
use serde_json::Value;
use std::collections::BTreeMap;

const PAGES: &[(&str, &str)] = &[
    ("SettingsPageA9", "a9"),
    ("SettingsPageAbout", "about"),
    ("SettingsPageAccountsEmailApp", "emailandaccounts"),
    ("SettingsPageAccountsPicture", "yourinfo"),
    ("SettingsPageAccountsSync", "backup"),
    ("SettingsPageAccountsUsers", "otherusers"),
    ("SettingsPageActivate", "activation"),
    ("SettingsPageAdvancedApps", "advanced-apps"),
    ("SettingsPageAIComponents", "aicomponents"),
    ("SettingsPageAppActions", "appactions"),
    ("SettingsPageAppsDefaults", "defaultapps"),
    ("SettingsPageAppsForWebsites", "appsforwebsites"),
    ("SettingsPageAppsNotifications", "notifications"),
    ("SettingsPageAudio", "sound"),
    ("SettingsPageBackground", "personalization-background"),
    ("SettingsPageC2D", "clicktodo"),
    ("SettingsPageClipboard", "clipboard"),
    ("SettingsPageColors", "colors"),
    ("SettingsPageContinuum", "project"),
    ("SettingsPageDeviceEncryption", "deviceencryption"),
    ("SettingsPageDevices", "bluetooth"),
    ("SettingsPageDevicesCameras", "camera"),
    ("SettingsPageDevicesDL", "personalization-lighting"),
    ("SettingsPageDevicesMobileDevices", "mobile-devices"),
    ("SettingsPageDevicesPen", "pen"),
    ("SettingsPageDevicesPrinters", "printers"),
    ("SettingsPageDevicesTouch", "devices-touch"),
    ("SettingsPageDevicesTouchpad", "devices-touchpad"),
    ("SettingsPageEaseOfAccessAudio", "easeofaccess-audio"),
    ("SettingsPageEaseOfAccessClosedCaptioning", "easeofaccess-closedcaptioning"),
    ("SettingsPageEaseOfAccessColorFilter", "easeofaccess-colorfilter"),
    ("SettingsPageEaseOfAccessEyeGaze", "easeofaccess-eyecontrol"),
    ("SettingsPageEaseOfAccessHearingAids", "easeofaccess-hearingaids"),
    ("SettingsPageEaseOfAccessHighContrast", "easeofaccess-highcontrast"),
    ("SettingsPageEaseOfAccessKeyboard", "easeofaccess-keyboard"),
    ("SettingsPageEaseOfAccessMagnifier", "easeofaccess-magnifier"),
    ("SettingsPageEaseOfAccessMouse", "easeofaccess-mouse"),
    ("SettingsPageEaseOfAccessMousePointer", "easeofaccess-mousepointer"),
    ("SettingsPageEaseOfAccessNarrator", "easeofaccess-narrator"),
    ("SettingsPageEaseOfAccessSpeechRecognition", "easeofaccess-speechrecognition"),
    ("SettingsPageEaseOfAccessTextCursor", "easeofaccess-cursor"),
    ("SettingsPageEaseOfAccessTextSize", "easeofaccess-display"),
    ("SettingsPageEaseOfAccessVisualEffects", "easeofaccess-visualeffects"),
    ("SettingsPageFindMyDevice", "findmydevice"),
    ("SettingsPageFlights", "windowsinsider"),
    ("SettingsPageFonts", "fonts"),
    ("SettingsPageGameBar", "gaming-gamebar"),
    ("SettingsPageGameDVR", "gaming-gamedvr"),
    ("SettingsPageGameMode", "gaming-gamemode"),
    ("SettingsPageInstalledApps", "appsfeatures"),
    ("SettingsPageLockScreen", "lockscreen"),
    ("SettingsPageMaps", "maps"),
    ("SettingsPageMultiTasking", "multitasking"),
    ("SettingsPageNetworkAirplaneMode", "network-airplanemode"),
    ("SettingsPageNetworkDialup", "network-dialup"),
    ("SettingsPageNetworkEthernet", "network-ethernet"),
    ("SettingsPageNetworkManageAdapterOptions", "network-advancedsettings"),
    ("SettingsPageNetworkMobileBroadband", "network-cellular"),
    ("SettingsPageNetworkMobileHotspot", "network-mobilehotspot"),
    ("SettingsPageNetworkProxy", "network-proxy"),
    ("SettingsPageNetworkVPN", "network-vpn"),
    ("SettingsPageNetworkWiFi", "network-wifi"),
    ("SettingsPageNetworkWorkplace", "workplace"),
    ("SettingsPageOptionalFeatures", "optionalfeatures"),
    ("SettingsPageOtherUsers", "otherusers"),
    ("SettingsPagePasskeysManagement", "savedpasskeys"),
    ("SettingsPagePCSystemAutoPlay", "autoplay"),
    ("SettingsPagePCSystemBluetooth", "bluetooth"),
    ("SettingsPagePCSystemDevices", "connecteddevices"),
    ("SettingsPagePCSystemDeviceSettings", "mousetouchpad"),
    ("SettingsPagePCSystemDisplay", "display"),
    ("SettingsPagePersonalizationTextInput", "personalization-textinput"),
    ("SettingsPagePowerAndBattery", "powersleep"),
    ("SettingsPagePrivacyAccountInfo", "privacy-accountinfo"),
    ("SettingsPagePrivacyActivityHistory", "privacy-activityhistory"),
    ("SettingsPagePrivacyAppDiagnostics", "privacy-appdiagnostics"),
    ("SettingsPagePrivacyAppRequestedDownloads", "privacy-automaticfiledownloads"),
    ("SettingsPagePrivacyBackgroundApps", "privacy-backgroundapps"),
    ("SettingsPagePrivacyBroadFileSystemAccess", "privacy-broadfilesystemaccess"),
    ("SettingsPagePrivacyCalendar", "privacy-calendar"),
    ("SettingsPagePrivacyCallHistory", "privacy-callhistory"),
    ("SettingsPagePrivacyContacts", "privacy-contacts"),
    ("SettingsPagePrivacyCustomPeripherals", "privacy-customdevices"),
    ("SettingsPagePrivacyDocuments", "privacy-documents"),
    ("SettingsPagePrivacyDownloadsFolder", "privacy-downloadsfolder"),
    ("SettingsPagePrivacyEmail", "privacy-email"),
    ("SettingsPagePrivacyGeneral", "privacy-general"),
    ("SettingsPagePrivacyLocation", "privacy-location"),
    ("SettingsPagePrivacyMicrophone", "privacy-microphone"),
    ("SettingsPagePrivacyNotifications", "privacy-notifications"),
    ("SettingsPagePrivacyPictures", "privacy-pictures"),
    ("SettingsPagePrivacyVideos", "privacy-videos"),
    ("SettingsPagePrivacyWebcam", "privacy-webcam"),
    ("SettingsPageRemoteDesktop", "remotedesktop"),
    ("SettingsPageRestoreMusUpdate", "windowsupdate"),
    ("SettingsPageRestoreRestore", "recovery"),
    ("SettingsPageSearchPermissionsAndHistory", "search-permissions"),
    ("SettingsPageSearchIndex", "cortana-windowssearch"),
    ("SettingsPageSharedExperiences", "crossdevice"),
    ("SettingsPageSignInOptions", "signinoptions"),
    ("SettingsPageSpeech", "speech"),
    ("SettingsPageStart", "personalization-start"),
    ("SettingsPageStartup", "startupapps"),
    ("SettingsPageStorageSenseStorageOverview", "storagesense"),
    ("SettingsPageTaskbar", "taskbar"),
    ("SettingsPageThemes", "themes"),
    ("SettingsPageTimeRegionDateTime", "dateandtime"),
    ("SettingsPageTimeRegionLanguage", "regionlanguage"),
    ("SettingsPageTimeRegionSpelling", "typing"),
    ("SettingsPageTroubleshoot", "troubleshoot"),
    ("SettingsPageUsb", "usb"),
    ("SettingsPageVideo", "videoplayback"),
    ("SettingsPageWindowsDefender", "windowsdefender"),
    ("SettingsPageYourFamilyGroup", "family-group"),
];

fn uri_for(page: &str) -> Option<&'static str> {
    PAGES.iter().find(|(id, _)| *id == page).map(|(_, uri)| *uri)
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.get("Value")?.as_str().map(str::to_string)
}

pub fn parse(cache: &str) -> Vec<(InstalledApp, String)> {
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(cache.trim_start_matches('\u{feff}')) else { return Vec::new() };
    let mut pages: BTreeMap<String, (InstalledApp, String)> = BTreeMap::new();
    for entry in &entries {
        let Some(parsing) = string(entry, "System.ParsingName") else { continue };
        if !parsing.starts_with("AAA_SettingsPage") {
            continue;
        }
        let Some(path) = entry.get("SettingPaths").and_then(Value::as_array).and_then(|p| p.first()) else { continue };
        if path.get("System.Setting.GroupID").is_some() {
            continue;
        }
        let Some(page) = string(path, "System.Setting.PageID") else { continue };
        let (Some(uri), Some(title)) = (uri_for(&page), string(entry, "System.Comment")) else { continue };
        let keywords = string(entry, "System.HighKeywords").unwrap_or_default();
        let key = title.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        pages.entry(key).or_insert((InstalledApp { name: title, path: format!("ms-settings:{uri}") }, keywords));
    }
    pages.into_values().collect()
}

#[derive(serde::Serialize)]
pub struct SystemPage {
    name: String,
    path: String,
    keywords: String,
    kind: &'static str,
}

pub fn system_pages() -> Vec<SystemPage> {
    let settings = settings_pages().into_iter().map(|(app, keywords)| SystemPage { name: app.name, path: app.path, keywords, kind: "System Settings" });
    let panel = crate::apps::control_panel_items().into_iter().map(|app| SystemPage { name: app.name, path: app.path, keywords: String::new(), kind: "Control Panel" });
    settings.chain(panel).collect()
}

pub fn settings_pages() -> Vec<(InstalledApp, String)> {
    let Ok(local) = std::env::var("LOCALAPPDATA") else { return Vec::new() };
    let file = std::path::Path::new(&local).join(r"Packages\windows.immersivecontrolpanel_cw5n1h2txyewy\LocalState\DeviceSearchCache\SettingsCache.txt");
    std::fs::read_to_string(file).map(|text| parse(&text)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_mapped_page_entries_only() {
        let cache = r#"[
          {"System.ParsingName":{"Type":12,"Value":"AAA_SettingsPageAudio-2.settingcontent-ms"},"SettingPaths":[{"System.Setting.PageID":{"Type":12,"Value":"SettingsPageAudio"}}],"System.Comment":{"Type":12,"Value":"Configurações de som"},"System.HighKeywords":{"Type":12,"Value":"áudio;alto-falantes"}},
          {"System.ParsingName":{"Type":12,"Value":"AAA_SettingsGroupMicrophoneOptions.settingcontent-ms"},"SettingPaths":[{"System.Setting.PageID":{"Type":12,"Value":"SettingsPageAudio"},"System.Setting.GroupID":{"Type":12,"Value":"Mic"}}],"System.Comment":{"Type":12,"Value":"Mic"}},
          {"System.ParsingName":{"Type":12,"Value":"AAA_SettingsPageUnknown.settingcontent-ms"},"SettingPaths":[{"System.Setting.PageID":{"Type":12,"Value":"SettingsPageUnknown"}}],"System.Comment":{"Type":12,"Value":"Unknown"}}
        ]"#;
        let pages = parse(cache);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].0, InstalledApp { name: "Configurações de som".into(), path: "ms-settings:sound".into() });
        assert_eq!(pages[0].1, "áudio;alto-falantes");
        assert!(parse("not json").is_empty());
    }

    #[test]
    fn every_mapped_uri_exists_in_windows() {
        let Ok(known) = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ms-settings.txt")) else { return };
        let known: Vec<&str> = known.split_whitespace().collect();
        for (_, uri) in PAGES.iter() {
            assert!(known.contains(&format!("ms-settings:{uri}").as_str()), "ms-settings:{uri} is not a Windows URI");
        }
    }

    #[test]
    #[ignore = "reads the real Settings cache and Control Panel"]
    fn prints_repeated_names() {
        let pages = system_pages();
        let mut names: Vec<String> = pages.iter().map(|p| format!("{} | {} | {}", p.name, p.path, p.kind)).collect();
        names.sort();
        println!("total {}", names.len());
        for pair in names.windows(2) {
            if pair[0].split(" | ").next() == pair[1].split(" | ").next() {
                println!("DUP {} || {}", pair[0], pair[1]);
            }
        }
    }
}
