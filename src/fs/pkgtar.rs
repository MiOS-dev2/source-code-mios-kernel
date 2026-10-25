
use alloc::string::String;

fn octal(field: &[u8]) -> Option<usize> {
    let mut v: usize = 0;
    let mut seen = false;
    for &b in field {
        if b == 0 || b == b' ' {
            if seen { break; }
            continue;
        }
        if b < b'0' || b > b'7' {
            return None;
        }
        v = v.checked_mul(8)?.checked_add((b - b'0') as usize)?;
        seen = true;
    }
    Some(v)
}

fn checksum_ok(h: &[u8]) -> bool {
    let stored = match octal(&h[148..156]) {
        Some(v) => v,
        None => return false,
    };
    let mut sum: usize = 0;
    for i in 0..512 {
        sum += if i >= 148 && i < 156 { 32 } else { h[i] as usize };
    }
    sum == stored
}

fn cstr(bytes: &[u8]) -> String {
    let mut n = 0;
    while n < bytes.len() && bytes[n] != 0 {
        n += 1;
    }
    String::from_utf8_lossy(&bytes[..n]).into_owned()
}

pub fn walk<F>(tar: &[u8], mut f: F) -> Result<usize, &'static str>
where
    F: FnMut(&str, bool, &[u8]) -> Result<(), &'static str>,
{
    let mut off: usize = 0;
    let mut count: usize = 0;
    let mut long_name: Option<String> = None;

    while off + 512 <= tar.len() {
        let h = &tar[off..off + 512];
        if h.iter().all(|&b| b == 0) {
            break;
        }
        if !checksum_ok(h) {
            return Err("not a valid tar archive");
        }
        let size = match octal(&h[124..136]) {
            Some(s) => s,
            None => return Err("bad size in tar"),
        };
        let data_start = off + 512;
        let data_end = match data_start.checked_add(size) {
            Some(e) if e <= tar.len() => e,
            _ => return Err("tar archive is truncated"),
        };
        let typeflag = h[156];

        match typeflag {
            b'L' => {
                long_name = Some(cstr(&tar[data_start..data_end]));
            }
            b'0' | 0 | b'5' => {
                let path: String = match long_name.take() {
                    Some(n) => n,
                    None => {
                        let name = cstr(&h[0..100]);
                        if &h[257..262] == b"ustar" && h[345] != 0 {
                            let prefix = cstr(&h[345..500]);
                            let mut full = prefix;
                            full.push('/');
                            full.push_str(&name);
                            full
                        } else {
                            name
                        }
                    }
                };
                let is_dir = typeflag == b'5' || path.ends_with('/');
                let data: &[u8] = if is_dir { &[] } else { &tar[data_start..data_end] };
                f(&path, is_dir, data)?;
                count += 1;
            }
            _ => {
            }
        }

        off = data_start + ((size + 511) & !511);
    }
    Ok(count)
}
