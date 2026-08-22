use kueri_lib::db::{
    connect::{ConnectionConfigV2, SCHEMA_VERSION},
    DbKind,
};
use kueri_lib::{
    safety::SafetyLevel,
    secrets::PasswordSource,
    ssh::profile::{SshAuth, SshProfile, SshRef},
    tls::{TlsConfig, TlsMode},
};
use uuid::Uuid;

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: T,
) {
    let json = serde_json::to_string(&value).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(value, back);
}

#[test]
fn tls_config_roundtrip() {
    roundtrip(TlsConfig {
        mode: TlsMode::VerifyFull,
        ca_path: Some("/tmp/ca.pem".into()),
        cert_path: None,
        key_path: None,
    });
}

#[test]
fn safety_level_default_is_confirm_destructive() {
    assert_eq!(SafetyLevel::default(), SafetyLevel::ConfirmDestructive);
}

#[test]
fn safety_level_roundtrip() {
    roundtrip(SafetyLevel::ReadOnly);
    roundtrip(SafetyLevel::Off);
}

#[test]
fn password_source_variants_roundtrip() {
    roundtrip(PasswordSource::Plain);
    roundtrip(PasswordSource::Keychain);
    roundtrip(PasswordSource::Env {
        name: "PGPASSWORD".into(),
    });
    roundtrip(PasswordSource::OnePassword {
        item: "prod-db".into(),
        field: "password".into(),
    });
    roundtrip(PasswordSource::Vault {
        path: "secret/db".into(),
        field: "password".into(),
    });
    roundtrip(PasswordSource::AwsSm {
        arn: "arn:aws:secretsmanager:us-east-1:0:secret:x".into(),
        region: "us-east-1".into(),
    });
}

#[test]
fn ssh_ref_variants_roundtrip() {
    let profile = SshProfile {
        id: Uuid::new_v4(),
        name: "bastion".into(),
        host: "jump.example.com".into(),
        port: 22,
        user: "ec2-user".into(),
        auth: SshAuth::Agent,
        jump: None,
    };
    roundtrip(SshRef::Profile(profile.id));
    roundtrip(SshRef::Inline(profile));
}

#[test]
fn connection_config_v2_roundtrip() {
    let cfg = ConnectionConfigV2 {
        id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        name: "prod-analytics".into(),
        kind: DbKind::Postgres,
        host: "db.internal".into(),
        port: 5432,
        database: "analytics".into(),
        user: "readonly".into(),
        password: PasswordSource::Keychain,
        tls: Some(TlsConfig {
            mode: TlsMode::Require,
            ca_path: None,
            cert_path: None,
            key_path: None,
        }),
        ssh: None,
        safety: SafetyLevel::ConfirmDestructive,
        color: Some("prod".into()),
        tags: vec!["prod".into(), "analytics".into()],
        file_path: None,
    };
    roundtrip(cfg);
}
