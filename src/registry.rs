//! Block registry: maps numeric block ids (singleplayer ids or server block-state ids)
//! to how Claudium renders and collides with them.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Air,
    /// Full opaque cube (or a partial box described by `y0..y1`).
    Solid,
    /// Cube rendered with alpha-cutout (leaves, glass). Neighbours still show through.
    Cutout,
    /// Translucent liquid (water).
    Water,
    /// Two crossed quads (flowers, grass, saplings, torches). No collision.
    Cross,
    /// Present but not drawn and not collidable (signs, rails, buttons...).
    Invisible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    None = 0,
    Grass = 1,
    Foliage = 2,
    Water = 3,
}

#[derive(Clone, Debug)]
pub struct BlockInfo {
    pub name: String,
    pub kind: Kind,
    /// Atlas tiles for [top, bottom, side].
    pub tex: [u16; 3],
    /// Which faces get the biome tint: [top, bottom, side].
    pub tint: [Tint; 3],
    /// Vertical extent of the box in sixteenths (0..=16).
    pub y0: u8,
    pub y1: u8,
    /// Faces are hidden against neighbours with the same id (glass, water).
    pub cull_same: bool,
}

impl BlockInfo {
    pub fn air(name: &str) -> Self {
        BlockInfo { name: name.to_string(), kind: Kind::Air, tex: [0; 3], tint: [Tint::None; 3], y0: 0, y1: 16, cull_same: false }
    }
    /// A full, opaque cube that hides any neighbour face touching it.
    #[inline]
    pub fn occludes(&self) -> bool {
        self.kind == Kind::Solid && self.y0 == 0 && self.y1 == 16
    }
    #[inline]
    pub fn collides(&self) -> bool {
        matches!(self.kind, Kind::Solid | Kind::Cutout)
    }
    #[inline]
    pub fn is_air(&self) -> bool {
        self.kind == Kind::Air
    }
}

pub struct Registry {
    pub blocks: Vec<BlockInfo>,
    /// Tile names in atlas order (index = tile id).
    pub tiles: Vec<String>,
    by_name: HashMap<String, u16>,
    fallback: BlockInfo,
}

impl Registry {
    #[inline]
    pub fn get(&self, id: u16) -> &BlockInfo {
        self.blocks.get(id as usize).unwrap_or(&self.fallback)
    }
    pub fn id_of(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }
}

/// Collects the texture names blocks ask for and assigns atlas tiles.
pub struct TileAlloc {
    pub names: Vec<String>,
    map: HashMap<String, u16>,
}

impl TileAlloc {
    pub fn new() -> Self {
        let mut t = TileAlloc { names: vec![], map: HashMap::new() };
        t.tile("missing"); // tile 0
        t
    }
    pub fn tile(&mut self, name: &str) -> u16 {
        if let Some(&i) = self.map.get(name) {
            return i;
        }
        let i = self.names.len() as u16;
        self.names.push(name.to_string());
        self.map.insert(name.to_string(), i);
        i
    }
}

/// Decides how a block called `name` (without namespace) is drawn.
/// `props` are the block-state properties (e.g. `type=bottom` for slabs).
pub fn classify(name: &str, props: &HashMap<String, String>, textures: &dyn Fn(&str) -> bool, alloc: &mut TileAlloc) -> BlockInfo {
    let mut b = BlockInfo::air(name);
    if matches!(name, "air" | "cave_air" | "void_air" | "structure_void" | "light" | "barrier") {
        return b;
    }

    let has = |t: &str| textures(t);
    let pick = |cands: &[String]| -> Option<String> { cands.iter().find(|c| has(c)).cloned() };

    // Liquids
    if name == "water" || name == "bubble_column" {
        b.kind = Kind::Water;
        let t = alloc.tile("water_still");
        b.tex = [t; 3];
        b.tint = [Tint::Water; 3];
        b.cull_same = true;
        return b;
    }
    if name == "lava" {
        b.kind = Kind::Solid;
        let t = alloc.tile("lava_still");
        b.tex = [t; 3];
        return b;
    }

    // Things we don't draw yet
    let invisible_suffix = [
        "_sign",
        "_wall_sign",
        "_hanging_sign",
        "_banner",
        "_wall_banner",
        "_button",
        "_pressure_plate",
        "rail",
        "_head",
        "_skull",
        "_wall_head",
        "_wall_skull",
    ];
    if invisible_suffix.iter().any(|s| name.ends_with(s))
        || matches!(
            name,
            "redstone_wire"
                | "lever"
                | "tripwire"
                | "tripwire_hook"
                | "fire"
                | "soul_fire"
                | "moving_piston"
                | "end_gateway"
                | "end_portal"
                | "nether_portal"
                | "scaffolding"
                | "string"
        )
    {
        b.kind = Kind::Invisible;
        return b;
    }

    // Cross-shaped plants
    let cross_names = [
        "short_grass",
        "grass",
        "tall_grass",
        "fern",
        "large_fern",
        "dead_bush",
        "dandelion",
        "poppy",
        "blue_orchid",
        "allium",
        "azure_bluet",
        "oxeye_daisy",
        "cornflower",
        "lily_of_the_valley",
        "wither_rose",
        "sunflower",
        "lilac",
        "rose_bush",
        "peony",
        "torchflower",
        "sugar_cane",
        "brown_mushroom",
        "red_mushroom",
        "crimson_fungus",
        "warped_fungus",
        "crimson_roots",
        "warped_roots",
        "nether_sprouts",
        "cobweb",
        "seagrass",
        "tall_seagrass",
        "kelp",
        "kelp_plant",
        "sweet_berry_bush",
        "torch",
        "wall_torch",
        "soul_torch",
        "soul_wall_torch",
        "redstone_torch",
        "redstone_wall_torch",
        "hanging_roots",
        "glow_lichen",
        "vine",
        "bush",
        "firefly_bush",
        "short_dry_grass",
        "tall_dry_grass",
        "pink_petals",
        "wildflowers",
        "leaf_litter",
        "cactus_flower",
        "pitcher_plant",
        "spore_blossom",
        "big_dripleaf_stem",
        "cave_vines",
        "cave_vines_plant",
        "weeping_vines",
        "weeping_vines_plant",
        "twisting_vines",
        "twisting_vines_plant",
    ];
    let is_cross = cross_names.contains(&name)
        || name.ends_with("_sapling")
        || name.ends_with("_tulip")
        || name.ends_with("_propagule")
        || matches!(name, "wheat" | "carrots" | "potatoes" | "beetroots");
    if is_cross {
        b.kind = Kind::Cross;
        let mut cands: Vec<String> = vec![name.to_string()];
        let half = props.get("half").map(|s| s.as_str());
        if matches!(name, "tall_grass" | "large_fern" | "sunflower" | "lilac" | "rose_bush" | "peony" | "tall_seagrass") {
            let part = if half == Some("upper") { "top" } else { "bottom" };
            cands.insert(0, format!("{name}_{part}"));
            if name == "sunflower" {
                cands.insert(0, format!("sunflower_{part}"));
            }
        }
        if name == "grass" {
            cands.push("short_grass".into());
        }
        if name == "wall_torch" {
            cands.insert(0, "torch".into());
        }
        if name == "soul_wall_torch" {
            cands.insert(0, "soul_torch".into());
        }
        if name == "redstone_wall_torch" {
            cands.insert(0, "redstone_torch".into());
        }
        if let Some(age) = props.get("age") {
            let stage = match name {
                "wheat" => age.clone(),
                "carrots" | "potatoes" => format!("{}", age.parse::<u32>().unwrap_or(0) / 2),
                "beetroots" => age.clone(),
                "sweet_berry_bush" => age.clone(),
                _ => age.clone(),
            };
            cands.insert(0, format!("{name}_stage{stage}"));
        }
        if name == "kelp_plant" {
            cands.insert(0, "kelp_plant".into());
        }
        let t = pick(&cands).unwrap_or_else(|| name.to_string());
        let id = alloc.tile(&t);
        b.tex = [id; 3];
        let tinted = matches!(name, "short_grass" | "grass" | "tall_grass" | "fern" | "large_fern" | "sugar_cane" | "vine" | "bush");
        if tinted {
            b.tint = [Tint::Grass; 3];
        }
        return b;
    }

    b.kind = Kind::Solid;

    // Cutout cubes (leaves, glass, ice...)
    if name.ends_with("_leaves") {
        b.kind = Kind::Cutout;
        let foliage_tint = !matches!(name, "cherry_leaves" | "pale_oak_leaves" | "azalea_leaves" | "flowering_azalea_leaves");
        if foliage_tint {
            b.tint = [Tint::Foliage; 3];
        }
    } else if name.contains("glass")
        || matches!(name, "ice" | "spawner" | "slime_block" | "honey_block" | "mangrove_roots" | "iron_bars" | "chain")
        || name.ends_with("_door")
        || name.ends_with("_trapdoor")
    {
        b.kind = Kind::Cutout;
        b.cull_same = true;
    }

    // Partial heights
    if name.ends_with("_slab") {
        match props.get("type").map(|s| s.as_str()) {
            Some("top") => b.y0 = 8,
            Some("double") => {}
            _ => b.y1 = 8,
        }
    } else if name == "snow" {
        let layers: u8 = props.get("layers").and_then(|l| l.parse().ok()).unwrap_or(1);
        b.y1 = (layers * 2).min(16);
    } else if name.ends_with("_carpet") || name == "moss_carpet" {
        b.y1 = 1;
    } else if matches!(name, "farmland" | "dirt_path") {
        b.y1 = 15;
    }

    // Textures. Many blocks reuse the texture of the block they're made from.
    let mut base = name.to_string();
    for suf in ["_slab", "_stairs", "_wall", "_fence_gate", "_fence", "_carpet"] {
        if let Some(stripped) = name.strip_suffix(suf) {
            base = stripped.to_string();
            break;
        }
    }
    let wood = ["oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "cherry", "bamboo", "crimson", "warped", "pale_oak"];
    let mut bases: Vec<String> = vec![base.clone()];
    if base != name {
        if wood.contains(&base.as_str()) {
            bases.insert(0, format!("{base}_planks"));
        }
        bases.push(format!("{base}s")); // stone_brick -> stone_bricks
        bases.push(format!("{base}_block"));
        if name.ends_with("_carpet") {
            bases.insert(0, format!("{base}_wool"));
        }
        if let Some(b2) = base.strip_suffix("_brick") {
            bases.push(format!("{b2}_bricks"));
        }
        if let Some(b2) = base.strip_suffix("_tile") {
            bases.push(format!("{b2}_tiles"));
        }
        bases.push(base.replace("smooth_", "") + "_top");
    }
    if name == "snow" {
        bases.insert(0, "snow".into());
    }

    let first = |suffix: &str| -> Option<String> {
        let c: Vec<String> = bases.iter().map(|b| format!("{b}{suffix}")).collect();
        pick(&c)
    };
    let plain = first("").unwrap_or_else(|| name.to_string());
    let mut top = first("_top").unwrap_or_else(|| plain.clone());
    let mut side = first("_side").unwrap_or_else(|| plain.clone());
    let mut bottom = first("_bottom").unwrap_or_else(|| top.clone());

    // Columns with an axis (logs) put the end texture on the axis faces. We only
    // support upright ones for now; sideways logs show bark on all sides.
    if name.ends_with("_log") || name.ends_with("_wood") || name.ends_with("_stem") || name.ends_with("_hyphae") || name == "bamboo_block" {
        let log = name.replace("_wood", "_log").replace("_hyphae", "_stem");
        side = pick(&[log.clone()]).unwrap_or(side);
        let end = pick(&[format!("{log}_top")]).unwrap_or(side.clone());
        let axis = props.get("axis").map(|s| s.as_str()).unwrap_or("y");
        if name.ends_with("_wood") || name.ends_with("_hyphae") || axis != "y" {
            top = side.clone();
            bottom = side.clone();
        } else {
            top = end.clone();
            bottom = end;
        }
    }

    match name {
        "grass_block" => {
            top = "grass_block_top".into();
            side = if props.get("snowy").map(|s| s == "true").unwrap_or(false) {
                "grass_block_snow".into()
            } else {
                "grass_block_side".into()
            };
            bottom = "dirt".into();
            b.tint[0] = Tint::Grass;
        }
        "mycelium" | "podzol" | "dirt_path" | "farmland" => {
            bottom = "dirt".into();
            if name == "farmland" {
                top = "farmland".into();
                side = "dirt".into();
            }
        }
        "crimson_nylium" | "warped_nylium" => bottom = "netherrack".into(),
        "sandstone" | "red_sandstone" => {
            bottom = format!("{name}_bottom");
        }
        _ => {}
    }

    b.tex = [alloc.tile(&top), alloc.tile(&bottom), alloc.tile(&side)];
    b
}

/// The fixed set of blocks the singleplayer world generator uses.
pub const SP_BLOCKS: &[&str] = &[
    "air",
    "stone",
    "dirt",
    "grass_block",
    "sand",
    "water",
    "oak_log",
    "oak_leaves",
    "bedrock",
    "gravel",
    "snow_block",
    "short_grass",
    "poppy",
    "dandelion",
    "sandstone",
    "coal_ore",
    "iron_ore",
    "oak_planks",
    "cobblestone",
    "glass",
    "bricks",
    "spruce_log",
    "spruce_leaves",
    "cactus",
    "clay",
    "diamond_ore",
];

pub fn build_singleplayer(textures: &dyn Fn(&str) -> bool) -> (Registry, Vec<String>) {
    let mut alloc = TileAlloc::new();
    let empty = HashMap::new();
    let mut blocks = vec![];
    let mut by_name = HashMap::new();
    for (i, n) in SP_BLOCKS.iter().enumerate() {
        let mut info = classify(n, &empty, textures, &mut alloc);
        if *n == "cactus" {
            info.tex = [alloc.tile("cactus_top"), alloc.tile("cactus_bottom"), alloc.tile("cactus_side")];
        }
        if *n == "spruce_leaves" {
            info.tint = [Tint::Foliage; 3];
        }
        blocks.push(info);
        by_name.insert(n.to_string(), i as u16);
    }
    let tiles = alloc.names.clone();
    let fallback = BlockInfo { kind: Kind::Solid, ..BlockInfo::air("unknown") };
    (Registry { blocks, tiles: alloc.names, by_name, fallback }, tiles)
}

/// Builds a registry from the server's block-state list (`blocks.json` produced by
/// the official data generator from the player's own game files).
pub fn build_from_report(json: &serde_json::Value, textures: &dyn Fn(&str) -> bool) -> anyhow::Result<(Registry, Vec<String>)> {
    let obj = json.as_object().ok_or_else(|| anyhow::anyhow!("blocks.json is not an object"))?;
    let mut alloc = TileAlloc::new();
    let mut max_id = 0usize;
    let mut entries: Vec<(usize, BlockInfo)> = Vec::new();
    let mut by_name = HashMap::new();
    for (full, v) in obj {
        let name = full.strip_prefix("minecraft:").unwrap_or(full);
        let Some(states) = v.get("states").and_then(|s| s.as_array()) else { continue };
        for st in states {
            let Some(id) = st.get("id").and_then(|i| i.as_u64()) else { continue };
            let mut props = HashMap::new();
            if let Some(p) = st.get("properties").and_then(|p| p.as_object()) {
                for (k, val) in p {
                    if let Some(s) = val.as_str() {
                        props.insert(k.clone(), s.to_string());
                    }
                }
            }
            let info = classify(name, &props, textures, &mut alloc);
            if st.get("default").and_then(|d| d.as_bool()).unwrap_or(false) {
                by_name.insert(name.to_string(), id as u16);
            }
            max_id = max_id.max(id as usize);
            entries.push((id as usize, info));
        }
    }
    let mut blocks = vec![BlockInfo::air("air"); max_id + 1];
    for (id, info) in entries {
        blocks[id] = info;
    }
    let fallback = BlockInfo { kind: Kind::Solid, ..BlockInfo::air("unknown") };
    let tiles = alloc.names.clone();
    Ok((Registry { blocks, tiles: alloc.names, by_name, fallback }, tiles))
}

/// Without a block report we can't know what the server's state ids mean, so every
/// non-air state gets its own colour. Terrain shape is still correct.
pub fn build_unknown_states() -> (Registry, Vec<String>) {
    let mut alloc = TileAlloc::new();
    let mut blocks = vec![BlockInfo::air("air")];
    for i in 1..32768u32 {
        let h = i.wrapping_mul(2654435761) >> 24; // 256 distinct colours
        let t = alloc.tile(&format!("#color{}", h));
        blocks.push(BlockInfo { kind: Kind::Solid, tex: [t; 3], ..BlockInfo::air("state") });
    }
    let fallback = BlockInfo { kind: Kind::Solid, ..BlockInfo::air("unknown") };
    let tiles = alloc.names.clone();
    (Registry { blocks, tiles: alloc.names, by_name: HashMap::new(), fallback }, tiles)
}
