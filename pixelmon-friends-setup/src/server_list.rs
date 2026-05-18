use crate::{launcher, manifest::Manifest, paths::InstallPaths};
use anyhow::{anyhow, Context, Result};
use std::{fs, path::Path};

const TAG_END: u8 = 0;
const TAG_BYTE: u8 = 1;
const TAG_SHORT: u8 = 2;
const TAG_INT: u8 = 3;
const TAG_LONG: u8 = 4;
const TAG_FLOAT: u8 = 5;
const TAG_DOUBLE: u8 = 6;
const TAG_BYTE_ARRAY: u8 = 7;
const TAG_STRING: u8 = 8;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;
const TAG_INT_ARRAY: u8 = 11;
const TAG_LONG_ARRAY: u8 = 12;

#[derive(Clone, Debug, PartialEq)]
enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List { element_type: u8, values: Vec<Tag> },
    Compound(Vec<(String, Tag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

pub fn upsert_server(manifest: &Manifest, paths: &InstallPaths) -> Result<()> {
    let servers_path = paths.game_dir.join("servers.dat");
    upsert_server_file(
        &servers_path,
        &manifest.server_name,
        &manifest.server_address,
    )?;
    launcher::print_installed_message(&manifest.server_name);
    Ok(())
}

fn upsert_server_file(path: &Path, server_name: &str, server_address: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("서버 목록 폴더를 만들 수 없습니다: {}", parent.display()))?;
    }

    let mut root = if path.is_file() {
        let bytes = fs::read(path)
            .with_context(|| format!("서버 목록을 읽을 수 없습니다: {}", path.display()))?;
        parse_root_compound(&bytes).unwrap_or_else(|_| Tag::Compound(Vec::new()))
    } else {
        Tag::Compound(Vec::new())
    };

    upsert_server_value(&mut root, server_name, server_address)?;
    let bytes = write_root_compound(&root)?;
    fs::write(path, bytes)
        .with_context(|| format!("서버 목록을 저장할 수 없습니다: {}", path.display()))?;
    Ok(())
}

fn upsert_server_value(root: &mut Tag, server_name: &str, server_address: &str) -> Result<()> {
    let Tag::Compound(root_entries) = root else {
        return Err(anyhow!("서버 목록 루트 형식이 올바르지 않습니다"));
    };

    let Some(servers_entry_index) = root_entries.iter().position(|(name, _)| name == "servers") else {
        root_entries.push((
            "servers".to_string(),
            Tag::List {
                element_type: TAG_COMPOUND,
                values: vec![server_entry(server_name, server_address)],
            },
        ));
        return Ok(());
    };

    let (_, servers_tag) = &mut root_entries[servers_entry_index];
    match servers_tag {
        Tag::List {
            element_type,
            values,
        } if *element_type == TAG_COMPOUND || values.is_empty() => {
            *element_type = TAG_COMPOUND;
            if let Some(existing) = values.iter_mut().find(|value| {
                compound_string(value, "ip").is_some_and(|ip| ip == server_address)
                    || compound_string(value, "name").is_some_and(|name| name == server_name)
            }) {
                set_compound_string(existing, "name", server_name)?;
                set_compound_string(existing, "ip", server_address)?;
            } else {
                values.push(server_entry(server_name, server_address));
            }
            Ok(())
        }
        _ => {
            *servers_tag = Tag::List {
                element_type: TAG_COMPOUND,
                values: vec![server_entry(server_name, server_address)],
            };
            Ok(())
        }
    }
}

fn server_entry(server_name: &str, server_address: &str) -> Tag {
    Tag::Compound(vec![
        ("name".to_string(), Tag::String(server_name.to_string())),
        ("ip".to_string(), Tag::String(server_address.to_string())),
        ("hideAddress".to_string(), Tag::Byte(0)),
        ("acceptTextures".to_string(), Tag::Byte(0)),
    ])
}

fn compound_string<'a>(tag: &'a Tag, key: &str) -> Option<&'a str> {
    let Tag::Compound(entries) = tag else {
        return None;
    };
    entries.iter().find_map(|(name, value)| {
        if name == key {
            if let Tag::String(text) = value {
                Some(text.as_str())
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn set_compound_string(tag: &mut Tag, key: &str, text: &str) -> Result<()> {
    let Tag::Compound(entries) = tag else {
        return Err(anyhow!("서버 항목 형식이 올바르지 않습니다"));
    };

    if let Some((_, value)) = entries.iter_mut().find(|(name, _)| name == key) {
        *value = Tag::String(text.to_string());
    } else {
        entries.push((key.to_string(), Tag::String(text.to_string())));
    }
    Ok(())
}

fn parse_root_compound(bytes: &[u8]) -> Result<Tag> {
    let mut reader = NbtReader::new(bytes);
    let tag_type = reader.read_u8()?;
    if tag_type != TAG_COMPOUND {
        return Err(anyhow!("NBT 루트가 compound가 아닙니다"));
    }
    let _root_name = reader.read_string()?;
    read_payload(&mut reader, TAG_COMPOUND)
}

fn read_payload(reader: &mut NbtReader<'_>, tag_type: u8) -> Result<Tag> {
    match tag_type {
        TAG_BYTE => Ok(Tag::Byte(reader.read_i8()?)),
        TAG_SHORT => Ok(Tag::Short(reader.read_i16()?)),
        TAG_INT => Ok(Tag::Int(reader.read_i32()?)),
        TAG_LONG => Ok(Tag::Long(reader.read_i64()?)),
        TAG_FLOAT => Ok(Tag::Float(f32::from_bits(reader.read_u32()?))),
        TAG_DOUBLE => Ok(Tag::Double(f64::from_bits(reader.read_u64()?))),
        TAG_BYTE_ARRAY => {
            let len = reader.read_len()?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(reader.read_i8()?);
            }
            Ok(Tag::ByteArray(values))
        }
        TAG_STRING => Ok(Tag::String(reader.read_string()?)),
        TAG_LIST => {
            let element_type = reader.read_u8()?;
            let len = reader.read_len()?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(read_payload(reader, element_type)?);
            }
            Ok(Tag::List {
                element_type,
                values,
            })
        }
        TAG_COMPOUND => {
            let mut entries = Vec::new();
            loop {
                let child_type = reader.read_u8()?;
                if child_type == TAG_END {
                    break;
                }
                let name = reader.read_string()?;
                let value = read_payload(reader, child_type)?;
                entries.push((name, value));
            }
            Ok(Tag::Compound(entries))
        }
        TAG_INT_ARRAY => {
            let len = reader.read_len()?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(reader.read_i32()?);
            }
            Ok(Tag::IntArray(values))
        }
        TAG_LONG_ARRAY => {
            let len = reader.read_len()?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(reader.read_i64()?);
            }
            Ok(Tag::LongArray(values))
        }
        other => Err(anyhow!("지원하지 않는 NBT 태그입니다: {other}")),
    }
}

fn write_root_compound(tag: &Tag) -> Result<Vec<u8>> {
    let mut writer = Vec::new();
    writer.push(TAG_COMPOUND);
    write_string(&mut writer, "")?;
    write_payload(&mut writer, tag)?;
    Ok(writer)
}

fn write_named_payload(writer: &mut Vec<u8>, name: &str, tag: &Tag) -> Result<()> {
    writer.push(tag_type(tag));
    write_string(writer, name)?;
    write_payload(writer, tag)
}

fn write_payload(writer: &mut Vec<u8>, tag: &Tag) -> Result<()> {
    match tag {
        Tag::Byte(value) => writer.push(*value as u8),
        Tag::Short(value) => writer.extend(value.to_be_bytes()),
        Tag::Int(value) => writer.extend(value.to_be_bytes()),
        Tag::Long(value) => writer.extend(value.to_be_bytes()),
        Tag::Float(value) => writer.extend(value.to_bits().to_be_bytes()),
        Tag::Double(value) => writer.extend(value.to_bits().to_be_bytes()),
        Tag::ByteArray(values) => {
            write_len(writer, values.len())?;
            for value in values {
                writer.push(*value as u8);
            }
        }
        Tag::String(value) => write_string(writer, value)?,
        Tag::List {
            element_type,
            values,
        } => {
            writer.push(*element_type);
            write_len(writer, values.len())?;
            for value in values {
                write_payload(writer, value)?;
            }
        }
        Tag::Compound(entries) => {
            for (name, value) in entries {
                write_named_payload(writer, name, value)?;
            }
            writer.push(TAG_END);
        }
        Tag::IntArray(values) => {
            write_len(writer, values.len())?;
            for value in values {
                writer.extend(value.to_be_bytes());
            }
        }
        Tag::LongArray(values) => {
            write_len(writer, values.len())?;
            for value in values {
                writer.extend(value.to_be_bytes());
            }
        }
    }
    Ok(())
}

fn tag_type(tag: &Tag) -> u8 {
    match tag {
        Tag::Byte(_) => TAG_BYTE,
        Tag::Short(_) => TAG_SHORT,
        Tag::Int(_) => TAG_INT,
        Tag::Long(_) => TAG_LONG,
        Tag::Float(_) => TAG_FLOAT,
        Tag::Double(_) => TAG_DOUBLE,
        Tag::ByteArray(_) => TAG_BYTE_ARRAY,
        Tag::String(_) => TAG_STRING,
        Tag::List { .. } => TAG_LIST,
        Tag::Compound(_) => TAG_COMPOUND,
        Tag::IntArray(_) => TAG_INT_ARRAY,
        Tag::LongArray(_) => TAG_LONG_ARRAY,
    }
}

fn write_string(writer: &mut Vec<u8>, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let len = u16::try_from(bytes.len()).context("NBT 문자열이 너무 깁니다")?;
    writer.extend(len.to_be_bytes());
    writer.extend(bytes);
    Ok(())
}

fn write_len(writer: &mut Vec<u8>, len: usize) -> Result<()> {
    let len = i32::try_from(len).context("NBT 목록 길이가 너무 깁니다")?;
    writer.extend(len.to_be_bytes());
    Ok(())
}

struct NbtReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> NbtReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.position + N;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| anyhow!("NBT 데이터를 끝까지 읽을 수 없습니다"))?;
        self.position = end;
        slice
            .try_into()
            .map_err(|_| anyhow!("NBT 데이터를 변환할 수 없습니다"))
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_exact::<1>()?[0])
    }

    fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    fn read_i16(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(self.read_exact()?))
    }

    fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read_exact()?))
    }

    fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.read_exact()?))
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.read_exact()?))
    }

    fn read_i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.read_exact()?))
    }

    fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.read_exact()?))
    }

    fn read_len(&mut self) -> Result<usize> {
        let len = self.read_i32()?;
        if len < 0 {
            return Err(anyhow!("NBT 길이가 음수입니다"));
        }
        usize::try_from(len).context("NBT 길이를 변환할 수 없습니다")
    }

    fn read_string(&mut self) -> Result<String> {
        let len = usize::from(self.read_u16()?);
        let end = self.position + len;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| anyhow!("NBT 문자열을 끝까지 읽을 수 없습니다"))?;
        self.position = end;
        String::from_utf8(slice.to_vec()).context("NBT 문자열이 UTF-8이 아닙니다")
    }
}

#[cfg(test)]
mod tests {
    use super::{compound_string, parse_root_compound, upsert_server_value, write_root_compound, Tag};

    #[test]
    fn creates_server_list() {
        let mut root = Tag::Compound(Vec::new());

        upsert_server_value(&mut root, "뭐해 포켓몬 모드 서버", "34.64.32.34:25565").unwrap();

        let Tag::Compound(entries) = &root else {
            panic!("root should be compound");
        };
        let servers = entries.iter().find(|(name, _)| name == "servers").unwrap();
        let Tag::List { values, .. } = &servers.1 else {
            panic!("servers should be list");
        };
        assert_eq!(values.len(), 1);
        assert_eq!(compound_string(&values[0], "name"), Some("뭐해 포켓몬 모드 서버"));
        assert_eq!(compound_string(&values[0], "ip"), Some("34.64.32.34:25565"));
    }

    #[test]
    fn round_trips_server_list() {
        let mut root = Tag::Compound(Vec::new());
        upsert_server_value(&mut root, "뭐해 포켓몬 모드 서버", "34.64.32.34:25565").unwrap();

        let bytes = write_root_compound(&root).unwrap();
        let parsed = parse_root_compound(&bytes).unwrap();

        assert_eq!(parsed, root);
    }
}
