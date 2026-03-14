fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: SuperRouteRepairBroker <app-instance-id> <connection-id> <nonce>"
        );
        std::process::exit(2);
    }

    eprintln!(
        "Super Route Pro repair broker skeleton received unlock request for app '{}' connection '{}'.",
        args[1], args[2]
    );
}
