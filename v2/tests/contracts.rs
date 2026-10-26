use modify_hash_v2::{
    core::{member, Draft},
    media::{classify, Media},
};
use serde_json::json;
#[test]
fn preferences_and_exact_marker_contract() {
    let d = Draft::defaults(&json!({}));
    assert!(d.showMD5Log && d.enableVideo);
    assert!(!d.enableAudio && !d.enableImage && !d.enableZipPassword);
    assert_eq!(d.zipPassword, "1024");
    assert_eq!(modify_hash_v2::core::MARKER, b"#$1024$#");
}
#[test]
fn media_is_content_based_not_extension_based() {
    for (bytes, expected) in [
        (b"GIF89a".as_slice(), Media::Image),
        (b"fLaC", Media::Audio),
        (b"FLV\x01", Media::Video),
    ] {
        assert_eq!(classify(bytes, bytes.len() as u64).unwrap(), Some(expected));
    }
    assert_eq!(classify(b"plain text.mp4", 14).unwrap(), None);
}
#[test]
fn bmff_brands_do_not_all_become_video() {
    for (brand, expected) in [
        (b"isom", Media::Video),
        (b"M4A ", Media::Audio),
        (b"heic", Media::Image),
        (b"avif", Media::Image),
        (b"crx ", Media::Image),
    ] {
        let mut bytes = b"\0\0\0\x18ftyp".to_vec();
        bytes.extend(brand);
        assert_eq!(
            classify(&bytes, bytes.len() as u64).unwrap(),
            Some(expected)
        );
    }
}
#[test]
fn malformed_containers_are_explicit_errors() {
    assert!(classify(b"ID3", 3).is_err());
    assert!(classify(b"OggS", 4).is_err());
    assert!(classify(b"\x1a\x45\xdf\xa3", 4).is_err());
}
#[test]
fn archive_components_are_portable_and_closed() {
    for name in [
        "../escape",
        "/absolute",
        "a/../b",
        "a//b",
        "a\\b",
        "C:/x",
        "a\0b",
        "CON",
        "x/NUL.txt",
        "x/com9",
        "a. ",
        "a/./b",
    ] {
        assert!(member(name).is_err(), "{name}");
    }
    for name in [
        "normal.txt",
        "雪 café/file (2).mp4",
        "dir/",
        "file..txt",
        "auxiliary",
    ] {
        assert!(member(name).is_ok(), "{name}");
    }
}
#[test]
fn password_fails_before_any_effect() {
    let mut d = Draft::defaults(&json!({}));
    d.paths = vec![if cfg!(windows) {
        "C:\\fixture".into()
    } else {
        "/fixture".into()
    }];
    d.enableZipPassword = true;
    assert!(d.validate("zipCompress").unwrap_err().contains("Encrypted"));
    d.zipPassword = " ".into();
    assert!(d.validate("zipCompress").is_ok());
}
#[test]
fn ordinary_inputs_are_not_restricted_to_extensions() {
    let mut d = Draft::defaults(&json!({}));
    d.paths = vec![if cfg!(windows) {
        "C:\\fixture\\雪 without extension".into()
    } else {
        "/fixture/雪 without extension".into()
    }];
    assert!(d.validate("modifyHash").is_ok());
}
