pub const BLOATWARE_CANDIDATES: [(&str, &str); 29] = [
    ("Clipchamp.Clipchamp", "Clipchamp"),
    ("Microsoft.BingNews", "Microsoft News"),
    ("Microsoft.BingWeather", "Microsoft Weather"),
    ("Microsoft.GetHelp", "Get Help"),
    ("Microsoft.Getstarted", "Get Started"),
    ("Microsoft.GamingApp", "Xbox"),
    ("Microsoft.Microsoft3DViewer", "3D Viewer"),
    ("Microsoft.MicrosoftOfficeHub", "Microsoft 365 (Office Hub)"),
    (
        "Microsoft.MicrosoftSolitaireCollection",
        "Microsoft Solitaire Collection",
    ),
    ("Microsoft.MixedReality.Portal", "Mixed Reality Portal"),
    ("Microsoft.OutlookForWindows", "Outlook for Windows"),
    ("Microsoft.People", "People"),
    ("Microsoft.PowerAutomateDesktop", "Power Automate"),
    ("Microsoft.SkypeApp", "Skype"),
    ("Microsoft.Todos", "Microsoft To Do"),
    ("Microsoft.WindowsAlarms", "Clock"),
    ("microsoft.windowscommunicationsapps", "Mail and Calendar"),
    ("Microsoft.WindowsFeedbackHub", "Feedback Hub"),
    ("Microsoft.WindowsMaps", "Maps"),
    ("Microsoft.Xbox.TCUI", "Xbox TCUI"),
    ("Microsoft.XboxGameOverlay", "Xbox Game Bar Plugin"),
    ("Microsoft.XboxGamingOverlay", "Xbox Game Bar"),
    ("Microsoft.XboxIdentityProvider", "Xbox Identity Provider"),
    ("Microsoft.XboxSpeechToTextOverlay", "Xbox Speech To Text"),
    ("Microsoft.YourPhone", "Phone Link"),
    ("Microsoft.ZuneMusic", "Media Player (Legacy Music)"),
    ("Microsoft.ZuneVideo", "Movies & TV"),
    ("MicrosoftTeams", "Microsoft Teams"),
    ("MicrosoftCorporationII.MicrosoftFamily", "Microsoft Family"),
];

pub fn canonical_bloatware_package(package_name: &str) -> Option<&'static str> {
    let lower = package_name.trim().to_lowercase();
    BLOATWARE_CANDIDATES
        .iter()
        .find(|(candidate, _)| candidate.to_lowercase() == lower)
        .map(|(candidate, _)| *candidate)
}
