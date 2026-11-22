//! Gives MediaPipe's lite hand landmark model the metadata the Tasks HandLandmarker needs (#48).
//!
//! The lite model (`hand_landmark_lite.tflite` from mediapipe-assets) has the same input and
//! outputs as the full one in `hand_landmarker.task`, but no TFLite metadata, and HandLandmarker
//! refuses it ("requires specifying NormalizationOptions metadata"). This copies the full model's
//! `TFLITE_METADATA` buffer and its packed label file onto the lite model.
//!
//! The lite FlatBuffer is kept byte for byte behind a new prefix holding a new root `Model` table
//! (offsets only point forward, so the new tables that refer to old data go in front of it); the
//! new metadata buffer goes after it, then the full model's zip of associated files.
//!
//! cargo run --example lite_hand_model -- <full.tflite> <lite.tflite> <out.tflite>

use std::fs;

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// A FlatBuffer table at `pos`: the absolute position of field `id`, if it is set.
fn field(b: &[u8], table: usize, id: usize) -> Option<usize> {
    let vtable = (table as i64 - i32::from_le_bytes(b[table..table + 4].try_into().unwrap()) as i64) as usize;
    let vt_len = u16::from_le_bytes(b[vtable..vtable + 2].try_into().unwrap()) as usize;
    let at = 4 + 2 * id;
    if at + 2 > vt_len {
        return None;
    }
    let off = u16::from_le_bytes(b[vtable + at..vtable + at + 2].try_into().unwrap()) as usize;
    (off != 0).then_some(table + off)
}

/// Follows the offset stored at `at`.
fn deref(b: &[u8], at: usize) -> usize {
    at + u32_at(b, at) as usize
}

/// A vector's element positions (each holding an offset, for vectors of tables).
fn elements(b: &[u8], vector: usize) -> Vec<usize> {
    (0..u32_at(b, vector) as usize).map(|i| vector + 4 + 4 * i).collect()
}

fn string(b: &[u8], s: usize) -> &[u8] {
    &b[s + 4..s + 4 + u32_at(b, s) as usize]
}

// Model table fields (tensorflow/lite/schema/schema.fbs).
const VERSION: usize = 0;
const BUFFERS: usize = 4;
const METADATA: usize = 6;
const MODEL_FIELDS: usize = 8;

/// The full model's metadata bytes and its trailing zip of associated files.
fn metadata_of(full: &[u8]) -> (Vec<u8>, Vec<u8>, usize) {
    let model = deref(full, 0);
    let buffers = deref(full, field(full, model, BUFFERS).unwrap());
    let entries = elements(full, deref(full, field(full, model, METADATA).unwrap()));
    let entry = entries
        .iter()
        .map(|&e| deref(full, e))
        .find(|&m| string(full, deref(full, field(full, m, 0).unwrap())) == b"TFLITE_METADATA")
        .expect("full model has TFLITE_METADATA");
    let index = u32_at(full, field(full, entry, 1).unwrap()) as usize;
    let buffer = deref(full, elements(full, buffers)[index]);
    let data = deref(full, field(full, buffer, 0).unwrap());
    let meta = full[data + 4..data + 4 + u32_at(full, data) as usize].to_vec();
    let start = zip_start(full);
    (meta, full[start..].to_vec(), start)
}

/// Where the trailing zip starts. Its offsets count from the
/// start of the whole file, as the metadata writer leaves them.
fn zip_start(b: &[u8]) -> usize {
    let eocd = (0..b.len() - 21).rev().find(|&i| b[i..i + 4] == *b"PK\x05\x06").expect("zip");
    let mut at = u32_at(b, eocd + 16) as usize;
    let mut entries = vec![];
    for _ in 0..u16::from_le_bytes(b[eocd + 10..eocd + 12].try_into().unwrap()) {
        assert_eq!(&b[at..at + 4], b"PK\x01\x02");
        entries.push(at);
        let lens: usize = [28, 30, 32].iter().map(|&o| u16::from_le_bytes(b[at + o..at + o + 2].try_into().unwrap()) as usize).sum();
        at += 46 + lens;
    }
    let start = entries.iter().map(|&e| u32_at(b, e + 42) as usize).min().unwrap();
    assert_eq!(&b[start..start + 4], b"PK\x03\x04");
    start
}

/// Moves the zip at the end of `b`, which began at `from`, to where it is now.
fn rebase_zip(b: &mut [u8], from: usize, now: usize) {
    let shift = |b: &mut [u8], at: usize| {
        let v = u32_at(b, at) as usize - from + now;
        b[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
    };
    let eocd = (0..b.len() - 21).rev().find(|&i| b[i..i + 4] == *b"PK\x05\x06").unwrap();
    shift(b, eocd + 16);
    let mut at = u32_at(b, eocd + 16) as usize;
    for _ in 0..u16::from_le_bytes(b[eocd + 10..eocd + 12].try_into().unwrap()) {
        shift(b, at + 42);
        let lens: usize = [28, 30, 32].iter().map(|&o| u16::from_le_bytes(b[at + o..at + o + 2].try_into().unwrap()) as usize).sum();
        at += 46 + lens;
    }
}

fn align(v: &mut Vec<u8>, to: usize) {
    while v.len() % to != 0 {
        v.push(0);
    }
}

fn with_metadata(lite: &[u8], meta: &[u8], zip: &[u8], zip_from: usize) -> Vec<u8> {
    let model = deref(lite, 0);
    let old_buffers = elements(lite, deref(lite, field(lite, model, BUFFERS).unwrap()));
    let old_meta = field(lite, model, METADATA).map(|f| elements(lite, deref(lite, f))).unwrap_or_default();

    // Prefix layout: root offset, "TFL3", vtable, Model table, buffers vector, metadata vector.
    let vt_at = 8;
    let vt_len = 4 + 2 * MODEL_FIELDS;
    let table_at = (vt_at + vt_len + 3) / 4 * 4;
    let table_len = 4 + 4 * MODEL_FIELDS; // soffset + one 4-byte slot per field
    let buffers_at = table_at + table_len;
    let meta_vec_at = buffers_at + 4 + 4 * (old_buffers.len() + 1);
    let prefix = (meta_vec_at + 4 + 4 * (old_meta.len() + 1) + 15) / 16 * 16;
    let old = |pos: usize| prefix + pos; // where an old position lands

    // After the old FlatBuffer: new Buffer table, its data, the Metadata table, its name.
    let mut tail_at = old(lite.len());
    tail_at = (tail_at + 15) / 16 * 16;
    let buffer_vt = tail_at; // vtable [6, 8, 4] for Buffer{data}
    let buffer_table = buffer_vt + 8;
    let data_at = (buffer_table + 8 + 4 + 15) / 16 * 16 - 4; // data bytes 16-aligned
    let meta_vt = (data_at + 4 + meta.len() + 3) / 4 * 4; // vtable [8, 12, 4, 8]
    let meta_table = meta_vt + 8;
    let name_at = meta_table + 12;
    let name = b"TFLITE_METADATA";
    let end = name_at + 4 + name.len() + 1;

    let mut out = vec![0u8; prefix];
    let put = |out: &mut Vec<u8>, at: usize, v: u32| out[at..at + 4].copy_from_slice(&v.to_le_bytes());
    let put16 = |out: &mut Vec<u8>, at: usize, v: u16| out[at..at + 2].copy_from_slice(&v.to_le_bytes());
    put(&mut out, 0, table_at as u32);
    out[4..8].copy_from_slice(b"TFL3");
    put16(&mut out, vt_at, vt_len as u16);
    put16(&mut out, vt_at + 2, table_len as u16);
    put(&mut out, table_at, (table_at - vt_at) as u32); // soffset: table − vtable
    for id in 0..MODEL_FIELDS {
        let slot = table_at + 4 + 4 * id;
        let value = match id {
            BUFFERS => Some((buffers_at - slot) as u32),
            METADATA => Some((meta_vec_at - slot) as u32),
            _ => field(lite, model, id).map(|f| {
                if id == VERSION { u32_at(lite, f) } else { (old(deref(lite, f)) - slot) as u32 }
            }),
        };
        if let Some(v) = value {
            put16(&mut out, vt_at + 4 + 2 * id, (slot - table_at) as u16);
            put(&mut out, slot, v);
        }
    }
    put(&mut out, buffers_at, old_buffers.len() as u32 + 1);
    for (i, &e) in old_buffers.iter().enumerate() {
        let slot = buffers_at + 4 + 4 * i;
        put(&mut out, slot, (old(deref(lite, e)) - slot) as u32);
    }
    let slot = buffers_at + 4 + 4 * old_buffers.len();
    put(&mut out, slot, (buffer_table - slot) as u32);
    put(&mut out, meta_vec_at, old_meta.len() as u32 + 1);
    for (i, &e) in old_meta.iter().enumerate() {
        let slot = meta_vec_at + 4 + 4 * i;
        put(&mut out, slot, (old(deref(lite, e)) - slot) as u32);
    }
    let slot = meta_vec_at + 4 + 4 * old_meta.len();
    put(&mut out, slot, (meta_table - slot) as u32);

    out.extend_from_slice(lite);
    out.resize(end, 0);
    put16(&mut out, buffer_vt, 6);
    put16(&mut out, buffer_vt + 2, 8);
    put16(&mut out, buffer_vt + 4, 4);
    put(&mut out, buffer_table, (buffer_table - buffer_vt) as u32);
    put(&mut out, buffer_table + 4, (data_at - (buffer_table + 4)) as u32);
    put(&mut out, data_at, meta.len() as u32);
    out[data_at + 4..data_at + 4 + meta.len()].copy_from_slice(meta);
    put(&mut out, name_at, name.len() as u32);
    out[name_at + 4..name_at + 4 + name.len()].copy_from_slice(name);
    put16(&mut out, meta_vt, 8);
    put16(&mut out, meta_vt + 2, 12);
    put16(&mut out, meta_vt + 4, 4);
    put16(&mut out, meta_vt + 6, 8);
    put(&mut out, meta_table, (meta_table - meta_vt) as u32);
    put(&mut out, meta_table + 4, (name_at - (meta_table + 4)) as u32);
    put(&mut out, meta_table + 8, old_buffers.len() as u32);
    align(&mut out, 16);
    let zip_now = out.len();
    out.extend_from_slice(zip);
    rebase_zip(&mut out, zip_from, zip_now);
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [full, lite, out] = &args[..] else {
        panic!("usage: lite_hand_model <full.tflite> <lite.tflite> <out.tflite>");
    };
    let (meta, zip, zip_from) = metadata_of(&fs::read(full).unwrap());
    let lite = fs::read(lite).unwrap();
    let made = with_metadata(&lite, &meta, &zip, zip_from);
    // Read back: the new model's metadata must be the full model's.
    let (back, back_zip, _) = metadata_of(&made);
    assert!(back == meta && back_zip.len() == zip.len());
    fs::write(out, &made).unwrap();
    println!("metadata {} bytes, zip {} bytes, model {} → {} bytes", meta.len(), zip.len(), lite.len(), made.len());
}
