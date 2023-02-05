use std::{path::Path};
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ},
    RegKey,
};
use std::process::Command;
fn get_chrome_windows() -> Option<String> {
    let hkey_localmachine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkey_currentuser = RegKey::predef(HKEY_CURRENT_USER);
    let hkeys = [hkey_currentuser, hkey_localmachine];
    let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe";
    for hkey in hkeys.iter() {
        let info = hkey.open_subkey_with_flags(path, KEY_READ);
        if let Err(_) = info {
            continue;
        }
        let info = info.unwrap();
        for value in info.enum_values() {
            if let Err(_) = value {
                continue;
            }
            let key = value.unwrap().1;
            let path = key.to_string();
            let path = path.replace("\"", "");
            if Path::exists(&Path::new(&path)) {
                return Some(path);
            }
        }
    }

    return None;
}

fn get_edge_windows() -> Option<String> {
    return Some("start msedge".to_string());
}

fn get_firefox_windows() -> Option<String> {
    let hkey_localmachine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkey_currentuser = RegKey::predef(HKEY_CURRENT_USER);
    let hkeys = [hkey_currentuser, hkey_localmachine];
    let path = r"SOFTWARE\Mozilla\Mozilla Firefox";
    for hkey in hkeys.iter() {
        let info = hkey.open_subkey_with_flags(path, KEY_READ);
        let info = match info {
            Ok(v) => v,
            Err(_) => continue,
        };
        for value in info.enum_keys() {
            let value = match value {
                Ok(v) => v,
                Err(_) => continue,
            };

            let p = Path::new(path).join(value).join("Main");
            let p = p.to_str().unwrap();
            let subkey = hkey.open_subkey_with_flags(p, KEY_READ);
            let subkey = match subkey {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = subkey.get_value("PathToExe");
            let path: String = match path {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = path.replace("\"", "");
            if Path::exists(&Path::new(&path)) {
                return Some(path);
            }
        }
    }
    return None
}



pub struct Browser {
    browser: BrowserEnum,
}

impl Browser {
    pub fn chrome() -> Browser {
        Browser {
            browser: BrowserEnum::Chrome
        }
    }
    pub fn edge() -> Browser {
        Browser {
            browser: BrowserEnum::Edge
        }
    }
    pub fn firefox() -> Browser {
        Browser {
            browser: BrowserEnum::Firefox
        }
    }
}

enum BrowserEnum {
    Chrome,
    Edge,
    Firefox,
}


pub fn open_browser(browser: Browser, url: String) {
    let path = match browser.browser {
        BrowserEnum::Chrome => {
            get_chrome_windows().expect("Couldn't find browser on this computer")
        }
        BrowserEnum::Edge => {
            get_edge_windows().expect("Couldn't find browser on this computer")
        }
        BrowserEnum::Firefox => {
            get_firefox_windows().expect("Couldn't find browser on this computer")
        }
    };

    let path = format!("{} --app={} --new-window", path, url);
    let split: Vec<&str> = path.split(" ").collect();
    let _command = Command::new(split[0]).args(&split[1..]).output().ok();
}