//! Category-only translation of the media branches in file-type 21.3.4.
//! No filename/extension inference: V1 uses content, including ISO BMFF brands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Media {
    Video,
    Audio,
    Image,
}
fn at(b: &[u8], n: usize, s: &[u8]) -> bool {
    b.get(n..n.saturating_add(s.len())) == Some(s)
}
pub fn classify(mut b: &[u8], size: u64) -> Result<Option<Media>, String> {
    use Media::*;
    for _ in 0..256 {
        if b.starts_with(&[0xef, 0xbb, 0xbf]) {
            b = &b[3..];
            continue;
        }
        if !b.starts_with(b"ID3") {
            break;
        }
        let Some(h) = b.get(..10) else {
            return Err("Truncated ID3 header".into());
        };
        let n = h[6..10]
            .iter()
            .fold(0usize, |a, x| (a << 7) | usize::from(x & 127));
        if n > 16 * 1024 * 1024 {
            return Err("ID3 detection exceeds 16 MiB".into());
        }
        if (n + 10) as u64 > size {
            return Ok(Some(Audio));
        }
        b = b
            .get(n + 10..)
            .ok_or("ID3 detection exceeds buffered probe")?;
    }
    if b.len() < 2 {
        return Ok(None);
    }
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut p = 8usize;
        let mut seen = false;
        for _ in 0..512 {
            let Some(h) = b.get(p..p + 8) else {
                return Ok(None);
            };
            let len = i32::from_be_bytes(h[..4].try_into().unwrap());
            if len < 0 {
                return Ok(None);
            }
            let kind = &h[4..8];
            if kind == b"IHDR" {
                if len != 13 {
                    return Ok(None);
                }
                seen = true;
            }
            if kind == b"IDAT" || kind == b"acTL" {
                return Ok(Some(Image));
            }
            if !seen && kind != b"CgBI" {
                return Ok(None);
            }
            p = p
                .checked_add(len as usize + 12)
                .ok_or("PNG size overflow")?;
            if p as u64 + 8 >= size {
                return Ok(Some(Image));
            }
            if p > b.len() {
                return Err("PNG probe exceeds 16 MiB".into());
            }
        }
        return Ok(Some(Image));
    }
    if b.starts_with(b"II*\0") || b.starts_with(b"MM\0*") {
        if b.len() < 8 {
            return Ok(None);
        }
        let be = b[0] == b'M';
        let read32 = |s: &[u8]| {
            if be {
                u32::from_be_bytes(s.try_into().unwrap())
            } else {
                u32::from_le_bytes(s.try_into().unwrap())
            }
        };
        let offset = read32(&b[4..8]) as usize;
        if offset >= 6 && at(b, 8, b"CR") {
            return Ok(Some(Image));
        }
        if offset >= 8 && b.len() >= 12 {
            let pair = if be {
                [
                    u16::from_be_bytes(b[8..10].try_into().unwrap()),
                    u16::from_be_bytes(b[10..12].try_into().unwrap()),
                ]
            } else {
                [
                    u16::from_le_bytes(b[8..10].try_into().unwrap()),
                    u16::from_le_bytes(b[10..12].try_into().unwrap()),
                ]
            };
            if pair == [28, 254] || pair == [31, 11] {
                return Ok(Some(Image));
            }
        }
        let Some(h) = b.get(offset..offset.saturating_add(2)) else {
            return if offset as u64 >= size {
                Ok(None)
            } else {
                Err("TIFF probe exceeds 16 MiB".into())
            };
        };
        let count = if be {
            u16::from_be_bytes(h.try_into().unwrap())
        } else {
            u16::from_le_bytes(h.try_into().unwrap())
        } as usize;
        if count > 512 {
            return Ok(Some(Image));
        }
        let end = offset.saturating_add(2 + count * 12);
        if end as u64 > size {
            return Ok(None);
        }
        if end > b.len() {
            return Err("TIFF probe exceeds 16 MiB".into());
        }
        return Ok(Some(Image));
    }
    let images: &[&[u8]] = &[
        b"BM",
        b"GIF",
        b"II\xbc",
        b"\xff\xd8\xff",
        b"FLIF",
        b"8BPS",
        b"icns",
        b"BPG\xfb",
        b"\x89PNG\r\n\x1a\n",
        b"II*\0",
        b"MM\0*",
        b"II+\0",
        b"MM\0+",
        b"gimp xcf ",
        b"IIRO\x08\0\0\0\x18",
        b"IIU\0\x18\0\0\0\x88\xe7\x74\xd8",
        b"\xabKTX 11\xbb\r\n\x1a\n",
        b"\xffO\xffQ",
        b"\xff\x0a",
        b"\0\0\0\x0cJXL \r\n\x87\x0a",
        b"FUJIFILMCCD-RAW",
        b"\0\0\x01\0",
        b"\0\0\x02\0",
    ];
    if images.iter().any(|s| b.starts_with(s)) {
        return Ok(Some(Image));
    }
    if b.starts_with(b"AC")
        && b.get(2..6)
            .and_then(|n| std::str::from_utf8(n).ok())
            .is_some_and(|n| ("1000"..="1050").contains(&n))
    {
        return Ok(Some(Image));
    }
    let audio: &[&[u8]] = &[
        b"\x0bw",
        b"MP+",
        b"MPCK",
        b"FORM",
        b"MThd",
        b"DSD ",
        b"fLaC",
        b"wvpk",
        b"MAC ",
        b"#!AMR",
        b"IMPM",
        b"Extended Module:",
        b"Creative Voice File",
    ];
    if audio.iter().any(|s| b.starts_with(s)) || at(b, 44, b"SCRM") {
        return Ok(Some(Audio));
    }
    if b.starts_with(b"OggS") {
        let p = b.get(28..36).ok_or("Truncated Ogg header")?;
        if p.starts_with(b"\x80theora") || p.starts_with(b"\x01video\0") {
            return Ok(Some(Video));
        }
        if [
            b"OpusHead".as_slice(),
            b"\x7fFLAC",
            b"Speex  ",
            b"\x01vorbis",
        ]
        .iter()
        .any(|s| p.starts_with(s))
        {
            return Ok(Some(Audio));
        }
        return Ok(None); // application/ogg in the source
    }
    if at(b, 4, b"ftyp") && b.get(8).is_some_and(|v| v & 0x60 != 0) {
        let brand = b.get(8..12).ok_or("Truncated ISO BMFF brand")?;
        let s = String::from_utf8_lossy(brand).replace('\0', " ");
        return Ok(Some(match s.trim() {
            "avif" | "avis" | "mif1" | "msf1" | "heic" | "heix" | "hevc" | "hevx" | "crx" => Image,
            "M4A" | "M4B" | "F4A" | "F4B" => Audio,
            _ => Video,
        }));
    }
    if b.starts_with(b"RIFF") {
        if at(b, 8, b"WEBP") {
            return Ok(Some(Image));
        }
        if at(b, 8, b"AVI") {
            return Ok(Some(Video));
        }
        if at(b, 8, b"WAVE") || at(b, 8, b"QLCM") {
            return Ok(Some(Audio));
        }
    }
    if b.starts_with(b"\0\0\0\x0cjP  \r\n\x87\n") {
        return Ok(
            if [b"jp2 ", b"jpx ", b"jpm ", b"mjp2"]
                .iter()
                .any(|s| at(b, 20, *s))
            {
                Some(Image)
            } else {
                None
            },
        );
    }
    if b.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        // Parse bounded EBML header children; do not search arbitrary payload bytes.
        fn vint(b: &[u8], p: &mut usize, keep: bool) -> Option<u64> {
            let x = *b.get(*p)?;
            let n = x.leading_zeros() as usize + 1;
            if n > 8 {
                return None;
            }
            let mut v = if keep {
                x as u64
            } else {
                (x & ((1u16 << (8 - n)) - 1) as u8) as u64
            };
            *p += 1;
            for _ in 1..n {
                v = (v << 8) | u64::from(*b.get(*p)?);
                *p += 1;
            }
            Some(v)
        }
        let mut p = 4;
        let n = vint(b, &mut p, false).ok_or("Invalid EBML length")? as usize;
        let end = p.checked_add(n).ok_or("EBML length overflow")?;
        for _ in 0..256 {
            if p >= end {
                return Ok(None);
            }
            let id = vint(b, &mut p, true).ok_or("Truncated EBML ID")?;
            let n = vint(b, &mut p, false).ok_or("Truncated EBML child")? as usize;
            if n > 1024 * 1024 {
                return Err("EBML probe limit".into());
            }
            let v = b
                .get(p..p.checked_add(n).ok_or("EBML overflow")?)
                .ok_or("EBML probe exceeds 16 MiB")?;
            if id == 0x4282 {
                return Ok(if v == b"webm" || v == b"matroska" {
                    Some(Video)
                } else {
                    None
                });
            }
            p += n;
        }
        return Err("EBML element limit".into());
    }
    if b.starts_with(&[0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9]) {
        let mut p = 30usize;
        for _ in 0..512 {
            let Some(h) = b.get(p..p + 24) else {
                return Ok(None);
            };
            let n = u64::from_le_bytes(h[16..24].try_into().unwrap());
            if !(24..=1024 * 1024).contains(&n) {
                return Err("ASF object bound".into());
            }
            if h.starts_with(&[
                0x91, 0x07, 0xdc, 0xb7, 0xb7, 0xa9, 0xcf, 0x11, 0x8e, 0xe6, 0, 0xc0, 0x0c, 0x20,
                0x53, 0x65,
            ]) {
                if at(
                    b,
                    p + 24,
                    &[
                        0x40, 0x9e, 0x69, 0xf8, 0x4d, 0x5b, 0xcf, 0x11, 0xa8, 0xfd, 0, 0x80, 0x5f,
                        0x5c, 0x44, 0x2b,
                    ],
                ) {
                    return Ok(Some(Audio));
                }
                if at(
                    b,
                    p + 24,
                    &[
                        0xc0, 0xef, 0x19, 0xbc, 0x4d, 0x5b, 0xcf, 0x11, 0xa8, 0xfd, 0, 0x80, 0x5f,
                        0x5c, 0x44, 0x2b,
                    ],
                ) {
                    return Ok(Some(Video));
                }
            }
            p = p.checked_add(n as usize).ok_or("ASF overflow")?;
        }
        return Err("ASF object limit".into());
    }
    if b.starts_with(b"FLV\x01")
        || [b"free", b"mdat", b"moov", b"wide"]
            .iter()
            .any(|s| at(b, 4, *s))
        || b.starts_with(b"\0\0\x01\xba")
        || b.starts_with(b"\0\0\x01\xb3")
        || (at(b, 0, b"G") && at(b, 188, b"G"))
        || (at(b, 4, b"G") && at(b, 196, b"G"))
    {
        return Ok(Some(Video));
    }
    // V1's imprecise detector scans for MPEG sync only after confident signatures.
    if b.starts_with(&[0xff, 0xfe])
        && (at(b, 2, b"<\0?\0x\0m\0l\0 \0")
            || at(b, 2, b"W\0i\0n\0d\0o\0w\0s\0 \0R\0e\0g\0i\0s\0t\0r\0y\0"))
    {
        return Ok(None);
    }
    for p in 0..b.len().saturating_sub(1).min(1) {
        if b[p] == 255 && b[p + 1] & 224 == 224 && (b[p + 1] & 0x16 == 0x10 || b[p + 1] & 6 != 0) {
            return Ok(Some(Audio));
        }
    }
    Ok(None)
}
