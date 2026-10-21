use serde::Serialize;
use windows::ApplicationModel::Contacts::{Contact, ContactAddressKind, ContactEmailKind, ContactManager, ContactPhoneKind, ContactStoreAccessType};

#[derive(Debug, Serialize)]
pub struct Labeled {
    label: &'static str,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    id: String,
    name: String,
    given_name: String,
    family_name: String,
    organization: String,
    phone_numbers: Vec<Labeled>,
    email_addresses: Vec<Labeled>,
    postal_addresses: Vec<Labeled>,
}

fn phone_label(kind: ContactPhoneKind) -> &'static str {
    match kind {
        ContactPhoneKind::Home => "Home",
        ContactPhoneKind::Mobile => "Mobile",
        ContactPhoneKind::Work => "Work",
        ContactPhoneKind::Pager => "Pager",
        ContactPhoneKind::BusinessFax => "Work Fax",
        ContactPhoneKind::HomeFax => "Home Fax",
        ContactPhoneKind::Company => "Company",
        ContactPhoneKind::Assistant => "Assistant",
        ContactPhoneKind::Radio => "Radio",
        _ => "Phone",
    }
}

fn email_label(kind: ContactEmailKind) -> &'static str {
    match kind {
        ContactEmailKind::Personal => "Personal",
        ContactEmailKind::Work => "Work",
        _ => "Email",
    }
}

fn address_label(kind: ContactAddressKind) -> &'static str {
    match kind {
        ContactAddressKind::Home => "Home",
        ContactAddressKind::Work => "Work",
        _ => "Address",
    }
}

fn join_address(parts: [String; 5]) -> String {
    parts.into_iter().map(|p| p.trim().to_owned()).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(", ")
}

fn person(c: &Contact) -> windows::core::Result<Person> {
    let phones = c.Phones()?.into_iter().filter_map(|p| Some(Labeled { label: phone_label(p.Kind().ok()?), value: p.Number().ok()?.to_string() })).filter(|l| !l.value.is_empty());
    let emails = c.Emails()?.into_iter().filter_map(|e| Some(Labeled { label: email_label(e.Kind().ok()?), value: e.Address().ok()?.to_string() })).filter(|l| !l.value.is_empty());
    let addresses = c.Addresses()?.into_iter().filter_map(|a| {
        let text = join_address([a.StreetAddress().ok()?.to_string(), a.Locality().ok()?.to_string(), a.Region().ok()?.to_string(), a.PostalCode().ok()?.to_string(), a.Country().ok()?.to_string()]);
        Some(Labeled { label: address_label(a.Kind().ok()?), value: text })
    });
    Ok(Person {
        id: c.Id()?.to_string(),
        name: c.DisplayName()?.to_string(),
        given_name: c.FirstName()?.to_string(),
        family_name: c.LastName()?.to_string(),
        organization: c.JobInfo()?.into_iter().find_map(|j| j.CompanyName().ok().map(|s| s.to_string()).filter(|s| !s.is_empty())).unwrap_or_default(),
        phone_numbers: phones.collect(),
        email_addresses: emails.collect(),
        postal_addresses: addresses.filter(|l| !l.value.is_empty()).collect(),
    })
}

fn all() -> windows::core::Result<Vec<Person>> {
    let store = ContactManager::RequestStoreAsyncWithAccessType(ContactStoreAccessType::AllContactsReadOnly)?.get()?;
    Ok(store.FindContactsAsync()?.get()?.into_iter().filter_map(|c| person(&c).ok()).collect())
}

#[tauri::command]
pub async fn contacts_list() -> Result<Vec<Person>, String> {
    tauri::async_runtime::spawn_blocking(|| all().map_err(|e| e.message())).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_only_the_filled_address_parts() {
        let parts = ["Rua A, 10".into(), "São Paulo".into(), " ".into(), "01000-000".into(), "Brasil".into()];
        assert_eq!(join_address(parts), "Rua A, 10, São Paulo, 01000-000, Brasil");
    }

    #[test]
    fn labels_unknown_kinds_generically() {
        assert_eq!(phone_label(ContactPhoneKind::Other), "Phone");
        assert_eq!(email_label(ContactEmailKind::Other), "Email");
        assert_eq!(address_label(ContactAddressKind::Other), "Address");
    }
}
