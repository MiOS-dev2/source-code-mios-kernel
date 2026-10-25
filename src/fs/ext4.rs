#![allow(dead_code)]
#![allow(non_snake_case)]
 
use crate::ata;
 
 
const EXT4_SUPER_MAGIC: u16 = 0xEF53;
const EXT4_EXTENT_MAGIC: u16 = 0xF30A;
const EXT4_EXTENTS_FL: u32 = 0x00080000;
const EXT4_S_IFMT: u16 = 0xF000;
const EXT4_S_IFDIR: u16 = 0x4000;
const EXT4_S_IFREG: u16 = 0x8000;
const EXT4_FEATURE_INCOMPAT_64BIT: u32 = 0x80;
 
const GPT_SIGNATURE: u64 = 0x5452415020494645;
const LINUX_FS_GUID: [u8; 16] = [
    0x0F, 0xC6, 0x3D, 0xAF, 0x84, 0x83, 0x47, 0x72,
    0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
];
 
 
pub struct Ext4FS {
    pub disk_index: usize,
    pub partition_offset: u64,
    pub block_size: u32,
    pub inodes_per_group: u32,
    pub inode_size: u16,
    pub blocks_per_group: u32,
    pub first_data_block: u32,
    pub group_desc_size: u16,
    pub groups_count: u32,
    pub inodes_count: u32,
    pub blocks_count: u64,
}
 
 
fn read_sector(disk_index: usize, sector: u32, buf: &mut [u8; 512]) -> bool {
    unsafe { ata::read_sector(disk_index, sector, buf) }
}
 
fn parse_gpt(disk_index: usize, offset_arr: &mut [u64; 16], count_arr: &mut [u64; 16]) -> usize {
    let mut mbr = [0u8; 512];
    if !read_sector(disk_index, 0, &mut mbr) {
        return 0;
    }
 
    let mut gpt_hdr = [0u8; 512];
    if !read_sector(disk_index, 1, &mut gpt_hdr) {
        return 0;
    }
 
    let mut sig: u64 = 0;
    for i in 0..8 {
        sig |= (gpt_hdr[i] as u64) << (i * 8);
    }
    if sig != GPT_SIGNATURE {
        return 0;
    }
 
    let mut part_lba: u64 = 0;
    for i in 0..8 {
        part_lba |= (gpt_hdr[0x48 + i] as u64) << (i * 8);
    }
 
    let num_entries = u32::from_le_bytes([
        gpt_hdr[0x50], gpt_hdr[0x51], gpt_hdr[0x52], gpt_hdr[0x53],
    ]) as usize;
 
    let entry_size = u32::from_le_bytes([
        gpt_hdr[0x54], gpt_hdr[0x55], gpt_hdr[0x56], gpt_hdr[0x57],
    ]) as usize;
 
    if num_entries == 0 || entry_size < 128 || entry_size > 512 {
        return 0;
    }
 
    let max_entries = num_entries.min(128);
    let entries_per_sector = 512 / entry_size;
 
    let mut count = 0usize;
 
    for i in 0..max_entries {
        let sector_idx = (i / entries_per_sector) as u32;
        let entry_off = (i % entries_per_sector) * entry_size;
 
        let mut buf = [0u8; 512];
        if !read_sector(disk_index, (part_lba as u32) + sector_idx, &mut buf) {
            break;
        }
 
        let mut guid = [0u8; 16];
        guid.copy_from_slice(&buf[entry_off..entry_off + 16]);
 
        let mut all_zero = true;
        for b in guid.iter() {
            if *b != 0 { all_zero = false; break; }
        }
        if all_zero { continue; }
 
        let is_linux = guid == LINUX_FS_GUID;
 
        let first_lba = u64::from_le_bytes([
            buf[entry_off + 0x20], buf[entry_off + 0x21],
            buf[entry_off + 0x22], buf[entry_off + 0x23],
            buf[entry_off + 0x24], buf[entry_off + 0x25],
            buf[entry_off + 0x26], buf[entry_off + 0x27],
        ]);
 
        let last_lba = u64::from_le_bytes([
            buf[entry_off + 0x28], buf[entry_off + 0x29],
            buf[entry_off + 0x2A], buf[entry_off + 0x2B],
            buf[entry_off + 0x2C], buf[entry_off + 0x2D],
            buf[entry_off + 0x2E], buf[entry_off + 0x2F],
        ]);
 
        if is_linux && count < 16 {
            offset_arr[count] = first_lba;
            count_arr[count] = last_lba - first_lba + 1;
            count += 1;
        }
    }
 
    count
}
 
fn parse_mbr(disk_index: usize, offset_arr: &mut [u64; 16], count_arr: &mut [u64; 16]) -> usize {
    let mut mbr = [0u8; 512];
    if !read_sector(disk_index, 0, &mut mbr) {
        return 0;
    }
    if mbr[510] != 0x55 || mbr[511] != 0xAA {
        return 0;
    }
 
    let mut count = 0usize;
    for i in 0..4 {
        let base = 0x1BE + i * 16;
        let ptype = mbr[base + 4];
        if ptype == 0x00 { continue; }
        if ptype == 0xEE { continue; }
 
        let lba = u32::from_le_bytes([
            mbr[base + 8], mbr[base + 9], mbr[base + 10], mbr[base + 11],
        ]) as u64;
        let sectors = u32::from_le_bytes([
            mbr[base + 12], mbr[base + 13], mbr[base + 14], mbr[base + 15],
        ]) as u64;
 
        if count < 16 {
            offset_arr[count] = lba;
            count_arr[count] = sectors;
            count += 1;
        }
    }
    count
}
 
fn find_ext4_partition(disk_index: usize) -> Option<u64> {
    let mut gpt_offsets = [0u64; 16];
    let mut gpt_counts = [0u64; 16];
 
    let mut offsets = [0u64; 16];
    let mut counts = [0u64; 16];
    let mut n = parse_gpt(disk_index, &mut gpt_offsets, &mut gpt_counts);
 
    if n > 0 {
        for i in 0..n {
            offsets[i] = gpt_offsets[i];
            counts[i] = gpt_counts[i];
        }
    } else {
        n = parse_mbr(disk_index, &mut offsets, &mut counts);
    }
 
    for i in 0..n {
        let start_sector = offsets[i] as u32;
        let sb_sector = start_sector + 2;
 
        let mut sec0 = [0u8; 512];
        let mut sec1 = [0u8; 512];
 
        if !read_sector(disk_index, sb_sector, &mut sec0) { continue; }
        if !read_sector(disk_index, sb_sector + 1, &mut sec1) { continue; }
 
        let magic = u16::from_le_bytes([sec0[0x38], sec0[0x39]]);
        if magic == EXT4_SUPER_MAGIC {
            return Some(offsets[i] * 512);
        }
 
        let _ = sec1;
    }
 
    None
}
 
 
impl Ext4FS {
    pub fn new(disk_index: usize) -> Option<Self> {
        let partition_offset = find_ext4_partition(disk_index)?;
 
        let sb_offset = partition_offset + 1024;
        let sb_sector = (sb_offset / 512) as u32;
        let sb_sector_off = (sb_offset % 512) as usize;
 
        let mut sec0 = [0u8; 512];
        let mut sec1 = [0u8; 512];
        if !read_sector(disk_index, sb_sector, &mut sec0) {
            return None;
        }
        if !read_sector(disk_index, sb_sector + 1, &mut sec1) {
            return None;
        }
 
        let mut sb_buf = [0u8; 1024];
        sb_buf[0..512 - sb_sector_off].copy_from_slice(&sec0[sb_sector_off..]);
        sb_buf[512 - sb_sector_off..1024 - sb_sector_off].copy_from_slice(&sec1[..512 - sb_sector_off]);
 
        let magic = u16::from_le_bytes([sb_buf[0x38], sb_buf[0x39]]);
        if magic != EXT4_SUPER_MAGIC {
            return None;
        }
 
        let s_inodes_count = u32::from_le_bytes([sb_buf[0x00], sb_buf[0x01], sb_buf[0x02], sb_buf[0x03]]);
        let s_blocks_count_lo = u32::from_le_bytes([sb_buf[0x04], sb_buf[0x05], sb_buf[0x06], sb_buf[0x07]]);
        let s_log_block_size = u32::from_le_bytes([sb_buf[0x18], sb_buf[0x19], sb_buf[0x1A], sb_buf[0x1B]]);
        let s_blocks_per_group = u32::from_le_bytes([sb_buf[0x20], sb_buf[0x21], sb_buf[0x22], sb_buf[0x23]]);
        let s_inodes_per_group = u32::from_le_bytes([sb_buf[0x28], sb_buf[0x29], sb_buf[0x2A], sb_buf[0x2B]]);
        let s_first_data_block = u32::from_le_bytes([sb_buf[0x14], sb_buf[0x15], sb_buf[0x16], sb_buf[0x17]]);
        let s_inode_size = u16::from_le_bytes([sb_buf[0x58], sb_buf[0x59]]);
        let s_feature_incompat = u32::from_le_bytes([sb_buf[0x60], sb_buf[0x61], sb_buf[0x62], sb_buf[0x63]]);
 
        if s_inodes_per_group == 0 || s_log_block_size > 6 || s_inode_size == 0 {
            return None;
        }

        let has_64bit = (s_feature_incompat & EXT4_FEATURE_INCOMPAT_64BIT) != 0;
        let group_desc_size = if has_64bit { 64u16 } else { 32u16 };
 
        let block_size = 1024u32 << s_log_block_size;
 
        let groups_count = (s_inodes_count + s_inodes_per_group - 1) / s_inodes_per_group;
 
        let s_blocks_count_hi = if has_64bit {
            u32::from_le_bytes([sb_buf[0x150], sb_buf[0x151], sb_buf[0x152], sb_buf[0x153]])
        } else {
            0
        };
        let blocks_count = ((s_blocks_count_hi as u64) << 32) | (s_blocks_count_lo as u64);
 
        Some(Ext4FS {
            disk_index,
            partition_offset,
            block_size,
            inodes_per_group: s_inodes_per_group,
            inode_size: s_inode_size,
            blocks_per_group: s_blocks_per_group,
            first_data_block: s_first_data_block,
            group_desc_size,
            groups_count,
            inodes_count: s_inodes_count,
            blocks_count,
        })
    }
 
 
    fn get_inode_table_block(&self, group: u32) -> Option<u64> {
        let super_block = if self.block_size == 1024 { 1u64 } else { 0u64 };
        let gd_start_block = super_block + 1;
 
        let gd_offset_bytes = gd_start_block * (self.block_size as u64)
            + (group as u64) * (self.group_desc_size as u64);
 
        let mut gd_buf = [0u8; 64];
        if !self.read_bytes_raw(gd_offset_bytes, &mut gd_buf) {
            return None;
        }
 
        let inode_table_lo = u32::from_le_bytes([
            gd_buf[0x08], gd_buf[0x09], gd_buf[0x0A], gd_buf[0x0B],
        ]) as u64;
 
        let inode_table_hi = if self.group_desc_size >= 64 {
            u32::from_le_bytes([
                gd_buf[0x28], gd_buf[0x29], gd_buf[0x2A], gd_buf[0x2B],
            ]) as u64
        } else {
            0
        };
 
        Some(inode_table_lo | (inode_table_hi << 32))
    }
 
 
    pub fn read_inode(&self, inode_num: u32) -> Option<[u8; 160]> {
        if inode_num == 0 || inode_num > self.inodes_count {
            return None;
        }
 
        let inode_index = (inode_num - 1) as u64;
        let group = (inode_index / self.inodes_per_group as u64) as u32;
        let index_in_group = (inode_index % self.inodes_per_group as u64) as u64;
 
        let inode_table_block = self.get_inode_table_block(group)?;
 
        let inode_offset = inode_table_block * (self.block_size as u64)
            + index_in_group * (self.inode_size as u64);
 
        let mut inode_buf = [0u8; 160];
        let copy_len = (self.inode_size as usize).min(160);
        if !self.read_bytes_raw(inode_offset, &mut inode_buf[..copy_len]) {
            return None;
        }
 
        Some(inode_buf)
    }
 
 
    fn read_bytes_raw(&self, offset: u64, out: &mut [u8]) -> bool {
        let offset = offset + self.partition_offset;
 
        let sector_size = 512u64;
        let mut done = 0usize;
        let mut cur_offset = offset;
        let total = out.len();
 
        while done < total {
            let sector = (cur_offset / sector_size) as u32;
            let sector_off = (cur_offset % sector_size) as usize;
            let mut sec_buf = [0u8; 512];
 
            if !read_sector(self.disk_index, sector, &mut sec_buf) {
                return false;
            }
 
            let to_copy = (512 - sector_off).min(total - done);
            out[done..done + to_copy].copy_from_slice(&sec_buf[sector_off..sector_off + to_copy]);
 
            done += to_copy;
            cur_offset += to_copy as u64;
        }
 
        true
    }
 
 
    fn inode_mode(inode: &[u8; 160]) -> u16 {
        u16::from_le_bytes([inode[0], inode[1]])
    }
    fn inode_size_lo(inode: &[u8; 160]) -> u32 {
        u32::from_le_bytes([inode[4], inode[5], inode[6], inode[7]])
    }
    fn inode_flags(inode: &[u8; 160]) -> u32 {
        u32::from_le_bytes([inode[32], inode[33], inode[34], inode[35]])
    }
    fn inode_i_block(inode: &[u8; 160]) -> [u8; 60] {
        let mut b = [0u8; 60];
        b.copy_from_slice(&inode[40..100]);
        b
    }
    fn inode_is_dir(inode: &[u8; 160]) -> bool {
        (Self::inode_mode(inode) & EXT4_S_IFMT) == EXT4_S_IFDIR
    }
    fn inode_is_file(inode: &[u8; 160]) -> bool {
        (Self::inode_mode(inode) & EXT4_S_IFMT) == EXT4_S_IFREG
    }
 
 
    pub fn read_file_data(&self, inode: &[u8; 160], out: &mut [u8]) -> Option<usize> {
        let flags = Self::inode_flags(inode);
        if (flags & EXT4_EXTENTS_FL) == 0 {
            return None;
        }
 
        let file_size = Self::inode_size_lo(inode) as usize;
        let max_out = out.len().min(file_size);
 
        let i_block = Self::inode_i_block(inode);
 
        let eh_magic = u16::from_le_bytes([i_block[0], i_block[1]]);
        let eh_entries = u16::from_le_bytes([i_block[2], i_block[3]]);
        let eh_depth = u16::from_le_bytes([i_block[6], i_block[7]]);
 
        if eh_magic != EXT4_EXTENT_MAGIC {
            return None;
        }
 
        let mut written = 0usize;
 
        if eh_depth == 0 {
            written = self.read_extent_leaf(&i_block[12..], eh_entries, file_size, max_out, out)?;
        } else if eh_depth == 1 {
            for idx in 0..eh_entries as usize {
                let base = 12 + idx * 12;
                if base + 12 > 60 { break; }
 
                let ei_leaf_lo = u32::from_le_bytes([
                    i_block[base+4], i_block[base+5], i_block[base+6], i_block[base+7],
                ]);
                let ei_leaf_hi = u16::from_le_bytes([i_block[base+8], i_block[base+9]]);
                let leaf_block = ((ei_leaf_hi as u64) << 32) | (ei_leaf_lo as u64);
 
                let mut leaf_buf = [0u8; 4096];
                let leaf_bytes = (self.block_size as usize).min(4096);
                let leaf_offset = leaf_block * (self.block_size as u64);
                if !self.read_bytes_raw(leaf_offset, &mut leaf_buf[..leaf_bytes]) {
                    return None;
                }
 
                let leaf_magic = u16::from_le_bytes([leaf_buf[0], leaf_buf[1]]);
                let leaf_entries = u16::from_le_bytes([leaf_buf[2], leaf_buf[3]]);
                if leaf_magic != EXT4_EXTENT_MAGIC { continue; }
 
                let got = self.read_extent_leaf(
                    &leaf_buf[12..], leaf_entries, file_size, max_out - written, &mut out[written..]
                )?;
                written += got;
                if written >= max_out { break; }
            }
        } else {
            return None;
        }
 
        Some(written)
    }
 
    fn read_extent_leaf(
        &self,
        leaf: &[u8],
        entries: u16,
        file_size: usize,
        max_out: usize,
        out: &mut [u8],
    ) -> Option<usize> {
        let mut written = 0usize;
 
        for i in 0..entries as usize {
            let base = i * 12;
            if base + 12 > leaf.len() { break; }
 
            let ee_len = u16::from_le_bytes([leaf[base+4], leaf[base+5]]);
            let ee_start_hi = u16::from_le_bytes([leaf[base+6], leaf[base+7]]);
            let ee_start_lo = u32::from_le_bytes([
                leaf[base+8], leaf[base+9], leaf[base+10], leaf[base+11],
            ]);
 
            let phys_block = ((ee_start_hi as u64) << 32) | (ee_start_lo as u64);
            let len = if ee_len > 32768 { 0usize } else { ee_len as usize };
 
            for b in 0..len {
                if written >= max_out || written >= file_size { break; }
 
                let block_to_read = phys_block + b as u64;
                let block_offset = block_to_read * (self.block_size as u64);
 
                let remain = max_out.min(file_size) - written;
                let chunk = remain.min(self.block_size as usize);
 
                if !self.read_bytes_raw(block_offset, &mut out[written..written + chunk]) {
                    return None;
                }
                written += chunk;
            }
 
            if written >= max_out || written >= file_size { break; }
        }
 
        Some(written)
    }
 
 
    pub fn read_dir(&self, dir_inode: &[u8; 160], out: &mut [u8]) -> usize {
        let mut data = [0u8; 65536];
        let size = match self.read_file_data(dir_inode, &mut data) {
            Some(s) => s,
            None => return 0,
        };
 
        let mut pos = 0usize;
        let mut out_pos = 0usize;
        let mut count = 0usize;
 
        while pos + 8 <= size {
            let inode_num = u32::from_le_bytes([
                data[pos], data[pos+1], data[pos+2], data[pos+3],
            ]);
            let rec_len = u16::from_le_bytes([data[pos+4], data[pos+5]]) as usize;
            let name_len = data[pos+6] as usize;
            let file_type = data[pos+7];
 
            if rec_len == 0 { break; }
            if pos + rec_len > size { break; }
 
            if inode_num != 0 && name_len > 0 && name_len <= 255 {
                let name_start = pos + 8;
                let name_end = name_start + name_len;
 
                if name_end <= size {
                    let name = &data[name_start..name_end];
                    let is_dot = name == b"." || name == b"..";
 
                    if !is_dot {
                        if out_pos + 6 + name_len > out.len() { break; }
 
                        out[out_pos..out_pos+4].copy_from_slice(&inode_num.to_le_bytes());
                        out[out_pos+4] = file_type;
                        out[out_pos+5] = name_len as u8;
                        out[out_pos+6..out_pos+6+name_len].copy_from_slice(name);
                        out_pos += 6 + name_len;
 
                        count += 1;
                    }
                }
            }
 
            pos += rec_len;
        }
 
        count
    }
 
    pub fn find_in_dir(&self, dir_inode: &[u8; 160], name: &str) -> Option<u32> {
        let mut data = [0u8; 65536];
        let size = self.read_file_data(dir_inode, &mut data)?;
 
        let target = name.as_bytes();
        let mut pos = 0usize;
 
        while pos + 8 <= size {
            let inode_num = u32::from_le_bytes([
                data[pos], data[pos+1], data[pos+2], data[pos+3],
            ]);
            let rec_len = u16::from_le_bytes([data[pos+4], data[pos+5]]) as usize;
            let name_len = data[pos+6] as usize;
 
            if rec_len == 0 { break; }
            if pos + rec_len > size { break; }
 
            if inode_num != 0 && name_len == target.len() {
                let name_start = pos + 8;
                let name_end = name_start + name_len;
                if name_end <= size && &data[name_start..name_end] == target {
                    return Some(inode_num);
                }
            }
 
            pos += rec_len;
        }
 
        None
    }
}
 
 
static mut FS: Option<Ext4FS> = None;
static mut ROOT_INODE: Option<[u8; 160]> = None;
 
pub unsafe fn mount(disk_index: usize) -> bool {
    if let Some(fs) = Ext4FS::new(disk_index) {
        if let Some(root) = fs.read_inode(2) {
            FS = Some(fs);
            ROOT_INODE = Some(root);
            return true;
        }
    }
    false
}
 
pub unsafe fn is_mounted() -> bool {
    FS.is_some()
}
 
pub unsafe fn list_files() -> &'static str {
    static mut RESULT: [u8; 8192] = [0; 8192];
    let mut pos = 0usize;
 
    let fs = match &FS {
        Some(f) => f,
        None => {
            let msg = b"EXT4 not mounted!";
            RESULT[..msg.len()].copy_from_slice(msg);
            RESULT[msg.len()] = 0;
            return core::str::from_utf8(&RESULT[..msg.len()]).unwrap_or("");
        }
    };
    let root = match &ROOT_INODE {
        Some(r) => r,
        None => return "No root inode!",
    };
 
    let mut dir_buf = [0u8; 8192];
    let count = fs.read_dir(root, &mut dir_buf);
 
    let header = b"=== EXT4 root directory ===\n";
    for b in header { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
 
    let mut dp = 0usize;
    for _ in 0..count {
        if dp + 6 > dir_buf.len() { break; }
 
        let inode_num = u32::from_le_bytes([
            dir_buf[dp], dir_buf[dp+1], dir_buf[dp+2], dir_buf[dp+3],
        ]);
        let file_type = dir_buf[dp+4];
        let name_len = dir_buf[dp+5] as usize;
        dp += 6;
 
        if dp + name_len > dir_buf.len() { break; }
        let name = &dir_buf[dp..dp+name_len];
        dp += name_len;
 
        let is_dir = file_type == 2;
        let prefix: &[u8] = if is_dir { b"[DIR]  " } else { b"[FILE] " };
        for b in prefix { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
 
        for b in name { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
 
        let n = num_to_str(inode_num as usize);
        if pos < 8191 { RESULT[pos] = b' '; pos += 1; }
        if pos < 8191 { RESULT[pos] = b'('; pos += 1; }
        for b in n.as_bytes() { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
        if pos < 8191 { RESULT[pos] = b')'; pos += 1; }
 
        if pos < 8191 { RESULT[pos] = b'\n'; pos += 1; }
    }
 
    RESULT[pos] = 0;
    core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
}
 
pub unsafe fn read_file(filename: &str) -> &'static str {
    static mut RESULT: [u8; 16384] = [0; 16384];
    let mut pos = 0usize;
 
    let fs = match &FS {
        Some(f) => f,
        None => {
            let msg = b"EXT4 not mounted!";
            RESULT[..msg.len()].copy_from_slice(msg);
            RESULT[msg.len()] = 0;
            return core::str::from_utf8(&RESULT[..msg.len()]).unwrap_or("");
        }
    };
    let root = match &ROOT_INODE {
        Some(r) => r,
        None => return "No root inode!",
    };
 
    let inode_num = match fs.find_in_dir(root, filename) {
        Some(n) => n,
        None => {
            let msg = b"File not found in root!\n";
            RESULT[..msg.len()].copy_from_slice(msg);
            RESULT[msg.len()] = 0;
            return core::str::from_utf8(&RESULT[..msg.len()]).unwrap_or("");
        }
    };
 
    let file_inode = match fs.read_inode(inode_num) {
        Some(i) => i,
        None => return "Cannot read inode",
    };
 
    if Ext4FS::inode_is_dir(&file_inode) {
        let msg = b"This is a directory, use ext4ls (only root)";
        RESULT[..msg.len()].copy_from_slice(msg);
        RESULT[msg.len()] = 0;
        return core::str::from_utf8(&RESULT[..msg.len()]).unwrap_or("");
    }
 
    let mut data = [0u8; 16000];
    let size = match fs.read_file_data(&file_inode, &mut data) {
        Some(s) => s,
        None => return "Cannot read file data (unsupported layout)",
    };
 
    let header = b"=== EXT4 file content ===\n\n";
    for b in header { if pos < 16383 { RESULT[pos] = *b; pos += 1; } }
 
    for i in 0..size {
        if pos >= 16383 { break; }
        let c = data[i];
        if (c >= 0x20 && c < 0x7F) || c == b'\n' || c == b'\r' || c == b'\t' {
            RESULT[pos] = c;
        } else {
            RESULT[pos] = b'.';
        }
        pos += 1;
    }
 
    if pos < 16383 { RESULT[pos] = b'\n'; pos += 1; }
    RESULT[pos] = 0;
    core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
}
 
pub unsafe fn info() -> &'static str {
    static mut RESULT: [u8; 512] = [0; 512];
    let mut pos = 0usize;
 
    let fs = match &FS {
        Some(f) => f,
        None => {
            let msg = b"EXT4 not mounted!";
            RESULT[..msg.len()].copy_from_slice(msg);
            RESULT[msg.len()] = 0;
            return core::str::from_utf8(&RESULT[..msg.len()]).unwrap_or("");
        }
    };
 
    let put = |r: &mut [u8; 512], p: &mut usize, s: &str| {
        for b in s.as_bytes() {
            if *p < 511 { r[*p] = *b; *p += 1; }
        }
    };
 
    put(&mut RESULT, &mut pos, "EXT4 Filesystem\nPartition offset (bytes): ");
    let po = num_to_str(fs.partition_offset as usize);
    put(&mut RESULT, &mut pos, po);
    put(&mut RESULT, &mut pos, "\nBlock size: ");
    let bs = num_to_str(fs.block_size as usize);
    put(&mut RESULT, &mut pos, bs);
    put(&mut RESULT, &mut pos, "\nInode size: ");
    let isz = num_to_str(fs.inode_size as usize);
    put(&mut RESULT, &mut pos, isz);
    put(&mut RESULT, &mut pos, "\nInodes: ");
    let ic = num_to_str(fs.inodes_count as usize);
    put(&mut RESULT, &mut pos, ic);
    put(&mut RESULT, &mut pos, "\nGroups: ");
    let gc = num_to_str(fs.groups_count as usize);
    put(&mut RESULT, &mut pos, gc);
    put(&mut RESULT, &mut pos, "\n");
 
    RESULT[pos] = 0;
    core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
}
 
fn num_to_str(num: usize) -> &'static str {
    static mut BUF: [u8; 20] = [0; 20];
    unsafe {
        let mut temp = num;
        let mut digits = [0u8; 20];
        let mut i = 0;
        if temp == 0 {
            BUF[0] = b'0';
            BUF[1] = 0;
            return "0";
        }
        while temp > 0 {
            digits[i] = b'0' + (temp % 10) as u8;
            temp /= 10;
            i += 1;
        }
        let mut j = 0;
        while i > 0 {
            i -= 1;
            BUF[j] = digits[i];
            j += 1;
        }
        BUF[j] = 0;
        core::str::from_utf8(&BUF[..j]).unwrap_or("0")
    }
}