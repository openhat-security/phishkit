use phishkit_core::engagement::{
    default_dryrun_domain, landing_sub, normalize_target_host, phishlet_name_for_target, slugify,
    upstream_domain,
};
use phishkit_core::mail::mask_secret;

#[test]
fn normalize_strips_scheme_path_and_port() {
    assert_eq!(
        normalize_target_host("https://App.Example.com:8443/login"),
        "app.example.com"
    );
}

#[test]
fn slugify_and_phishlet_naming() {
    assert_eq!(slugify("My App!!"), "my-app");
    assert_eq!(phishlet_name_for_target("app.acme.com"), "app-acme-com");
}

#[test]
fn dryrun_and_landing_sub() {
    assert_eq!(upstream_domain("app.acme.com"), "acme.com");
    assert_eq!(default_dryrun_domain("app.acme.com"), "acme.phishkit");
    assert_eq!(landing_sub("app.acme.com"), "app");
    assert_eq!(landing_sub("acme.com"), "");
}

#[test]
fn mail_secret_redaction() {
    assert_eq!(mask_secret(""), "");
    assert_eq!(mask_secret("hunter2"), phishkit_core::mail::SECRET_MASK);
}
