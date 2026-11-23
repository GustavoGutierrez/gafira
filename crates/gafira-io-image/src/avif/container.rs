//! The container of an AVIF image (ISO/IEC 23008-12 and the AV1 Image File
//! Format, both public): which items there are, where their bytes are and how
//! they are put together. Nothing is trusted: every place is checked.

/// What is wrong with a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Bad(pub &'static str);

type Read<T> = Result<T, Bad>;

/// The most items read, boxes in a box and pieces of a mosaic.
const MOST: usize = 4096;
const MOST_TILES: usize = 1024;

/// Bytes read from the first on.
#[derive(Clone, Copy)]
struct Bytes<'a>(&'a [u8]);

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Read<&'a [u8]> {
        let (head, rest) = self.0.split_at_checked(n).ok_or(Bad("cut short"))?;
        self.0 = rest;
        Ok(head)
    }

    /// A whole number of `n` bytes, 8 at most; 0 for none.
    fn number(&mut self, n: usize) -> Read<u64> {
        let bytes = self.take(n)?;
        Ok(bytes.iter().fold(0, |v, b| (v << 8) | u64::from(*b)))
    }

    fn u8(&mut self) -> Read<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Read<u16> {
        // Two bytes: the cast is exact.
        Ok(self.number(2)? as u16)
    }

    fn u32(&mut self) -> Read<u32> {
        // Four bytes: the cast is exact.
        Ok(self.number(4)? as u32)
    }

    fn kind(&mut self) -> Read<[u8; 4]> {
        self.take(4)?.try_into().map_err(|_| Bad("cut short"))
    }

    /// The version and the flags a full box begins with.
    fn full(&mut self) -> Read<(u8, u32)> {
        let both = self.u32()?;
        // The highest byte: the cast is exact.
        Ok(((both >> 24) as u8, both & 0x00FF_FFFF))
    }

    /// An id of 2 bytes, or of 4 in the later versions.
    fn id(&mut self, long: bool) -> Read<u32> {
        if long {
            self.u32()
        } else {
            self.u16().map(u32::from)
        }
    }

    /// The boxes that are left.
    fn boxes(self) -> Read<Vec<([u8; 4], Bytes<'a>)>> {
        let mut rest = self;
        let mut out = Vec::new();
        while !rest.0.is_empty() {
            let whole = rest.0.len() as u64;
            let mut size = u64::from(rest.u32()?);
            let kind = rest.kind()?;
            let mut header = 8;
            if size == 1 {
                size = rest.number(8)?;
                header = 16;
            } else if size == 0 {
                size = whole;
            }
            let inside = size.checked_sub(header).ok_or(Bad("a box too small"))?;
            let inside = usize::try_from(inside).map_err(|_| Bad("a box too large"))?;
            out.push((kind, Bytes(rest.take(inside)?)));
            if out.len() > MOST {
                return Err(Bad("too many boxes"));
            }
        }
        Ok(out)
    }
}

/// What is told of the colours of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cicp {
    pub matrix: u16,
    pub full: bool,
}

/// How an image is turned to be seen, one after the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Turn {
    /// Quarters of a turn, anticlockwise.
    Quarters(u8),
    /// Its top and its bottom are exchanged.
    Upside,
    /// Its left and its right are.
    Sideways,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Property {
    Size(u32, u32),
    Color(Cicp),
    Turn(Turn),
    Alpha,
    Other,
}

#[derive(Clone, Debug, Default)]
struct Item {
    id: u32,
    kind: [u8; 4],
    /// In the file, or in the box of data.
    inside: bool,
    extents: Vec<(u64, u64)>,
    properties: Vec<usize>,
}

/// An image of one piece, or of several side by side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Picture {
    /// The AV1 bytes of each piece, row by row.
    pub tiles: Vec<Vec<u8>>,
    pub columns: usize,
    /// Its size, when it is told.
    pub size: Option<(u32, u32)>,
    pub cicp: Option<Cicp>,
}

/// What an AVIF file shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Avif {
    pub color: Picture,
    pub alpha: Option<Picture>,
    /// The colours are already times the opacity.
    pub premultiplied: bool,
    pub turns: Vec<Turn>,
}

impl Avif {
    /// The size it is seen at.
    pub fn size(&self) -> Option<(u32, u32)> {
        let (w, h) = self.color.size?;
        let odd = self
            .turns
            .iter()
            .filter(|turn| matches!(turn, Turn::Quarters(n) if n % 2 == 1))
            .count()
            % 2
            == 1;
        Some(if odd { (h, w) } else { (w, h) })
    }
}

fn property(kind: [u8; 4], mut b: Bytes<'_>) -> Read<Property> {
    Ok(match &kind {
        b"ispe" => {
            b.full()?;
            Property::Size(b.u32()?, b.u32()?)
        }
        b"colr" => {
            if b.kind()? != *b"nclx" {
                return Ok(Property::Other);
            }
            b.take(4)?;
            let matrix = b.u16()?;
            let full = b.u8()? & 0x80 != 0;
            Property::Color(Cicp { matrix, full })
        }
        b"irot" => Property::Turn(Turn::Quarters(b.u8()? & 3)),
        b"imir" => Property::Turn(if b.u8()? & 1 == 0 {
            Turn::Upside
        } else {
            Turn::Sideways
        }),
        b"auxC" => {
            b.full()?;
            let alpha = [
                &b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0"[..],
                b"urn:mpeg:hevc:2015:auxid:1\0",
            ];
            if alpha.iter().any(|urn| b.0.starts_with(urn)) {
                Property::Alpha
            } else {
                Property::Other
            }
        }
        _ => Property::Other,
    })
}

/// Where the bytes of each item are.
fn places(mut b: Bytes<'_>) -> Read<Vec<Item>> {
    let (version, _) = b.full()?;
    if version > 2 {
        return Err(Bad("an unknown iloc"));
    }
    let sizes = b.u16()?;
    let nibble = |at: u16| usize::from((sizes >> at) & 0xF);
    let (offset, length, base) = (nibble(12), nibble(8), nibble(4));
    let index = if version == 0 { 0 } else { nibble(0) };
    if [offset, length, base, index].iter().any(|n| *n > 8) {
        return Err(Bad("an odd iloc"));
    }
    let count = b.id(version == 2)?;
    let mut out = Vec::new();
    for _ in 0..count {
        if out.len() >= MOST {
            return Err(Bad("too many items"));
        }
        let id = b.id(version == 2)?;
        let method = if version == 0 { 0 } else { b.u16()? & 0xF };
        if method > 1 {
            return Err(Bad("an item made of others"));
        }
        // Where it is told it is: in this file.
        b.u16()?;
        let from = b.number(base)?;
        let extents = b.u16()?;
        let mut list = Vec::new();
        for _ in 0..extents {
            b.number(index)?;
            let at = from.checked_add(b.number(offset)?);
            list.push((at.ok_or(Bad("an odd place"))?, b.number(length)?));
            if list.len() > MOST {
                return Err(Bad("too many pieces"));
            }
        }
        out.push(Item {
            id,
            inside: method == 1,
            extents: list,
            ..Item::default()
        });
    }
    Ok(out)
}

/// The kind of each item.
fn kinds(mut b: Bytes<'_>, items: &mut [Item]) -> Read<()> {
    let (version, _) = b.full()?;
    b.id(version > 0)?;
    for (kind, mut entry) in b.boxes()? {
        if kind != *b"infe" {
            continue;
        }
        let (version, _) = entry.full()?;
        if version < 2 {
            continue;
        }
        let id = entry.id(version > 2)?;
        entry.u16()?;
        let kind = entry.kind()?;
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            item.kind = kind;
        }
    }
    Ok(())
}

/// Which properties each item has.
fn owners(mut b: Bytes<'_>, items: &mut [Item]) -> Read<()> {
    let (version, flags) = b.full()?;
    let count = b.u32()?;
    for _ in 0..count {
        let id = b.id(version > 0)?;
        let properties = b.u8()?;
        let mut list = Vec::new();
        for _ in 0..properties {
            let index = if flags & 1 == 1 {
                usize::from(b.u16()? & 0x7FFF)
            } else {
                usize::from(b.u8()? & 0x7F)
            };
            list.push(index);
        }
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            item.properties = list;
        }
    }
    Ok(())
}

/// What an item says of others: what it is to them, which one and to which.
type Reference = ([u8; 4], u32, Vec<u32>);

/// What items say of others.
fn references(mut b: Bytes<'_>) -> Read<Vec<Reference>> {
    let (version, _) = b.full()?;
    let long = version > 0;
    let mut out = Vec::new();
    for (kind, mut entry) in b.boxes()? {
        let from = entry.id(long)?;
        let count = entry.u16()?;
        let to = (0..count).map(|_| entry.id(long)).collect::<Read<_>>()?;
        out.push((kind, from, to));
    }
    Ok(out)
}

/// What there is in the box that tells of the image.
struct Meta<'a> {
    file: &'a [u8],
    data: &'a [u8],
    items: Vec<Item>,
    properties: Vec<Property>,
    references: Vec<Reference>,
    primary: u32,
}

impl Meta<'_> {
    fn item(&self, id: u32) -> Read<&Item> {
        let mut all = self.items.iter();
        all.find(|item| item.id == id)
            .ok_or(Bad("an item that is not there"))
    }

    fn properties<'a>(&'a self, item: &'a Item) -> impl Iterator<Item = &'a Property> {
        // They are counted from 1.
        let of = |index: &usize| self.properties.get(index.checked_sub(1)?);
        item.properties.iter().filter_map(of)
    }

    fn bytes(&self, item: &Item) -> Read<Vec<u8>> {
        let from = if item.inside { self.data } else { self.file };
        let mut out = Vec::new();
        for (at, length) in &item.extents {
            let place = |v: u64| usize::try_from(v).map_err(|_| Bad("an odd place"));
            let rest = from.get(place(*at)?..).ok_or(Bad("an odd place"))?;
            let piece = if *length == 0 {
                rest
            } else {
                rest.get(..place(*length)?).ok_or(Bad("cut short"))?
            };
            if out.len() + piece.len() > from.len() {
                return Err(Bad("pieces over each other"));
            }
            out.extend_from_slice(piece);
        }
        if out.is_empty() {
            return Err(Bad("an item with nothing"));
        }
        Ok(out)
    }

    fn picture(&self, id: u32) -> Read<Picture> {
        let item = self.item(id)?;
        let cicp = |item: &Item| {
            self.properties(item).find_map(|p| match p {
                Property::Color(cicp) => Some(*cicp),
                _ => None,
            })
        };
        let size = self.properties(item).find_map(|p| match p {
            Property::Size(w, h) => Some((*w, *h)),
            _ => None,
        });
        match &item.kind {
            b"av01" => Ok(Picture {
                tiles: vec![self.bytes(item)?],
                columns: 1,
                size,
                cicp: cicp(item),
            }),
            b"grid" => {
                let told = self.bytes(item)?;
                let mut b = Bytes(&told);
                b.u8()?;
                let wide = b.u8()? & 1 == 1;
                let (rows, columns) = (usize::from(b.u8()?) + 1, usize::from(b.u8()?) + 1);
                let (w, h) = (b.id(wide)?, b.id(wide)?);
                let found = self.references.iter();
                let (_, _, tiles) = found
                    .clone()
                    .find(|(kind, from, _)| kind == b"dimg" && *from == id)
                    .ok_or(Bad("a mosaic of nothing"))?;
                if tiles.len() != rows * columns || tiles.len() > MOST_TILES {
                    return Err(Bad("an odd mosaic"));
                }
                let first = self.item(tiles[0])?;
                let of = |id: &u32| {
                    let tile = self.item(*id)?;
                    if tile.kind != *b"av01" {
                        return Err(Bad("an odd mosaic"));
                    }
                    self.bytes(tile)
                };
                Ok(Picture {
                    tiles: tiles.iter().map(of).collect::<Read<_>>()?,
                    columns,
                    size: Some((w, h)),
                    cicp: cicp(item).or_else(|| cicp(first)),
                })
            }
            _ => Err(Bad("not an AV1 image")),
        }
    }
}

/// Whether `file` begins as an AVIF does.
pub(crate) fn is_avif(file: &[u8]) -> bool {
    let mut b = Bytes(file);
    let (Ok(size), Ok(kind)) = (b.u32(), b.kind()) else {
        return false;
    };
    let brands = file.get(8..(size as usize).min(file.len())).unwrap_or(&[]);
    kind == *b"ftyp"
        && brands
            .chunks_exact(4)
            .any(|brand| brand == b"avif" || brand == b"avis")
}

/// What `file` shows.
pub(crate) fn read(file: &[u8]) -> Read<Avif> {
    if !is_avif(file) {
        return Err(Bad("not an AVIF"));
    }
    let top = Bytes(file).boxes()?;
    let mut meta = top
        .iter()
        .find(|(kind, _)| kind == b"meta")
        .map(|(_, b)| *b)
        .ok_or(Bad("no image"))?;
    meta.full()?;
    let boxes = meta.boxes()?;
    let find = |kind: &[u8; 4]| boxes.iter().find(|(k, _)| k == kind).map(|(_, b)| *b);
    let mut items = places(find(b"iloc").ok_or(Bad("no items"))?)?;
    kinds(find(b"iinf").ok_or(Bad("no items"))?, &mut items)?;
    let mut properties = Vec::new();
    if let Some(told) = find(b"iprp") {
        for (kind, b) in told.boxes()? {
            match &kind {
                b"ipco" => {
                    for (kind, b) in b.boxes()? {
                        properties.push(property(kind, b)?);
                    }
                }
                b"ipma" => owners(b, &mut items)?,
                _ => {}
            }
        }
    }
    let mut primary = find(b"pitm").ok_or(Bad("no image"))?;
    let (version, _) = primary.full()?;
    let primary = primary.id(version > 0)?;
    let meta = Meta {
        file,
        data: find(b"idat").map_or(&[], |b| b.0),
        items,
        properties,
        references: find(b"iref").map_or(Ok(Vec::new()), references)?,
        primary,
    };
    let color = meta.picture(meta.primary)?;
    let of = |kind: &[u8; 4]| {
        let all = meta.references.iter();
        let kind = *kind;
        all.filter(move |(k, _, _)| *k == kind)
    };
    let alpha = of(b"auxl")
        .filter(|(_, _, to)| to.contains(&meta.primary))
        .filter_map(|(_, from, _)| meta.item(*from).ok())
        .find(|item| meta.properties(item).any(|p| *p == Property::Alpha));
    let premultiplied = alpha.is_some_and(|alpha| {
        of(b"prem").any(|(_, from, to)| *from == meta.primary && to.contains(&alpha.id))
    });
    let alpha = alpha.map(|item| meta.picture(item.id)).transpose()?;
    let turns = meta
        .properties(meta.item(meta.primary)?)
        .filter_map(|p| match p {
            Property::Turn(turn) => Some(*turn),
            _ => None,
        })
        .collect();
    Ok(Avif {
        color,
        alpha,
        premultiplied,
        turns,
    })
}
