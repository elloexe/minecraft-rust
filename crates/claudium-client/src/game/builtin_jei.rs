//! The item viewer (`Mod::ItemViewer`): what Just Enough Items does, as Claudium's own
//! code. Every item beside the inventory and container screens, searchable (`@namespace`
//! narrows to one mod's items, `#tag` to a tag); click an item for the recipes that make it,
//! right-click (or press U over any item) for the recipes that use it, R for its recipes;
//! the recipe pages have a tab per station and a button that fills the crafting grid.
//!
//! It drives the game's item panel (`native_panel.rs`), the same panel a native recipe-viewer
//! mod drives, under the owner id [`OWNER`].
use super::{
    Game,
    native_hooks::{
        NativePanelEvent, NativeRecipe, NativeRecipeKind, NativeRecipeLayout, NativeRecipeStack,
        NativeRecipeView, NativeRecipes, NativeStack,
    },
};
use claudium_client::extras::Mod;
use std::sync::Arc;

/// The panel's owner while the item viewer shows it.
pub(super) const OWNER: &str = "claudium:item_viewer";

/// Recipes or uses of one item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Query {
    item: u32,
    uses: bool,
}

#[derive(Default)]
pub(super) struct JeiState {
    /// The panel shows the item viewer's items.
    shown: bool,
    search: String,
    /// The items listed, as indexes into `NativeRecipes::items`.
    listed: Vec<u32>,
    /// The recipe pages opened, the last one shown; empty shows the items.
    history: Vec<Query>,
    /// The recipe pages' tab, by kind.
    tab: usize,
    /// The recipes on the page shown, as indexes into `NativeRecipes::recipes`.
    page: Vec<usize>,
}

/// The tabs, in this order, and the station each shows as its icon.
const TABS: [(&[NativeRecipeKind], &str); 7] = [
    (
        &[
            NativeRecipeKind::CraftingShaped,
            NativeRecipeKind::CraftingShapeless,
        ],
        "minecraft:crafting_table",
    ),
    (&[NativeRecipeKind::Smelting], "minecraft:furnace"),
    (&[NativeRecipeKind::Blasting], "minecraft:blast_furnace"),
    (&[NativeRecipeKind::Smoking], "minecraft:smoker"),
    (&[NativeRecipeKind::Campfire], "minecraft:campfire"),
    (&[NativeRecipeKind::Stonecutting], "minecraft:stonecutter"),
    (&[NativeRecipeKind::Smithing], "minecraft:smithing_table"),
];
const BREWING_TAB: (&[NativeRecipeKind], &str) =
    (&[NativeRecipeKind::Brewing], "minecraft:brewing_stand");

fn tabs() -> impl Iterator<Item = (&'static [NativeRecipeKind], &'static str)> {
    TABS.into_iter().chain(std::iter::once(BREWING_TAB))
}

impl Game<'_> {
    /// Once a tick in the window: shows or takes away the item viewer's panel as its switch
    /// says. It is the window's only: the server's `Game` has no screens.
    pub(super) fn jei_tick(&mut self) {
        if !self.native_window() {
            return;
        }
        let on = self.settings.extras.on(Mod::ItemViewer) && self.player.is_some();
        let shown = self.extras.builtin.jei.shown;
        if on && !shown {
            self.extras.builtin.jei = JeiState::default();
            if self.jei_list() {
                self.extras.builtin.jei.shown = true;
            }
        } else if !on && shown {
            self.native_panel_recipes(OWNER, "", Vec::new(), 0, Vec::new());
            self.native_panel_items(OWNER, Vec::new());
            self.extras.builtin.jei = JeiState::default();
        }
    }

    /// Lists the items the search matches; false outside a world.
    fn jei_list(&mut self) -> bool {
        let Some(recipes) = self.native_recipes() else {
            return false;
        };
        let search = self.extras.builtin.jei.search.clone();
        let listed: Vec<u32> = (0..recipes.items.len() as u32)
            .filter(|&i| matches(&recipes.items[i as usize], &search))
            .collect();
        let stacks = listed
            .iter()
            .map(|&i| NativeStack {
                item: recipes.items[i as usize].id.clone(),
                count: 1,
                components: String::new(),
            })
            .collect();
        self.extras.builtin.jei.listed = listed;
        self.native_panel_items(OWNER, stacks);
        true
    }

    /// The player did `event` on the item viewer's panel.
    pub(super) fn jei_event(&mut self, event: &NativePanelEvent) {
        let Some(recipes) = self.native_recipes() else {
            return;
        };
        let item_of = |id: &str| {
            recipes
                .items
                .iter()
                .position(|i| i.id == id)
                .map(|i| i as u32)
        };
        match event {
            NativePanelEvent::Clicked { index, right } => {
                if let Some(&item) = self.extras.builtin.jei.listed.get(*index as usize) {
                    self.jei_open(&recipes, Query { item, uses: *right }, true);
                }
            }
            NativePanelEvent::Search(text) => {
                self.extras.builtin.jei.search = text.clone();
                self.jei_list();
            }
            NativePanelEvent::Key { key, stack } => {
                let uses = match key.as_str() {
                    "key.keyboard.r" => false,
                    "key.keyboard.u" => true,
                    _ => return,
                };
                if let Some(item) = item_of(&stack.item) {
                    self.jei_open(&recipes, Query { item, uses }, true);
                }
            }
            NativePanelEvent::SlotClicked { stack, right, .. } => {
                if let Some(item) = item_of(&stack.item) {
                    self.jei_open(&recipes, Query { item, uses: *right }, true);
                }
            }
            NativePanelEvent::Action(i) => {
                let book_id = self
                    .extras
                    .builtin
                    .jei
                    .page
                    .get(*i as usize)
                    .map(|&r| recipes.recipes[r].book_id);
                if let Some(id) = book_id.filter(|&id| id >= 0) {
                    let all = self.has_shift_down();
                    if let Err(e) = self.native_place_recipe(id, all) {
                        log::debug!("item viewer: {e}");
                    }
                }
            }
            NativePanelEvent::Back => {
                let state = &mut self.extras.builtin.jei;
                state.history.pop();
                match state.history.last().copied() {
                    Some(query) => self.jei_open(&recipes, query, false),
                    None => self.native_panel_recipes(OWNER, "", Vec::new(), 0, Vec::new()),
                }
            }
            NativePanelEvent::Tab(t) => {
                if let Some(query) = self.extras.builtin.jei.history.last().copied() {
                    let kinds = self.jei_tabs(&recipes, query);
                    if let Some(&kind) = kinds.get(*t as usize) {
                        self.extras.builtin.jei.tab = kind;
                        self.jei_show(&recipes, query);
                    }
                }
            }
            NativePanelEvent::Page(_) => {}
        }
    }

    /// Opens the recipes (or uses) of `query`'s item, on its first tab with any.
    fn jei_open(&mut self, recipes: &Arc<NativeRecipes>, query: Query, push: bool) {
        let tabs = self.jei_tabs(recipes, query);
        let Some(&first) = tabs.first() else {
            let name = recipes.items[query.item as usize].name.clone();
            let what = if query.uses {
                "No recipes use"
            } else {
                "No recipes make"
            };
            self.native_chat(&format!("{what} {name}."));
            return;
        };
        let state = &mut self.extras.builtin.jei;
        if push {
            state.history.push(query);
        }
        state.tab = first;
        self.jei_show(recipes, query);
    }

    /// The tabs (indexes into `tabs()`) that have recipes for `query`.
    fn jei_tabs(&self, recipes: &NativeRecipes, query: Query) -> Vec<usize> {
        tabs()
            .enumerate()
            .filter(|(_, (kinds, _))| {
                recipes
                    .recipes
                    .iter()
                    .any(|r| kinds.contains(&r.kind) && wanted(r, query))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Shows the recipes of `query` on the selected tab.
    fn jei_show(&mut self, recipes: &NativeRecipes, query: Query) {
        let tab_kinds = self.jei_tabs(recipes, query);
        let tab = self.extras.builtin.jei.tab;
        let Some((kinds, _)) = tabs().nth(tab) else {
            return;
        };
        let page: Vec<usize> = (0..recipes.recipes.len())
            .filter(|&i| {
                kinds.contains(&recipes.recipes[i].kind) && wanted(&recipes.recipes[i], query)
            })
            .collect();
        let crafting = self.crafting_open();
        let views = page
            .iter()
            .map(|&i| view(recipes, &recipes.recipes[i], crafting))
            .collect();
        let icons: Vec<NativeStack> = tab_kinds
            .iter()
            .filter_map(|&k| tabs().nth(k))
            .map(|(_, icon)| NativeStack {
                item: icon.to_owned(),
                count: 1,
                components: String::new(),
            })
            .collect();
        let selected = tab_kinds.iter().position(|&k| k == tab).unwrap_or(0) as u32;
        let name = &recipes.items[query.item as usize].name;
        let title = if query.uses {
            format!("Uses of {name}")
        } else {
            format!("Recipes for {name}")
        };
        self.extras.builtin.jei.page = page;
        self.native_panel_recipes(OWNER, &title, icons, selected, views);
    }

    /// A crafting grid is open, so the fill button can do something.
    fn crafting_open(&self) -> bool {
        self.craft_size > 0
    }
}

/// Whether `item` matches the search: words in its name or id, `@mod` for its namespace,
/// `#tag` for one of its tags.
fn matches(item: &super::native_hooks::NativeItem, search: &str) -> bool {
    search.split_whitespace().all(|word| {
        let word = word.to_lowercase();
        if let Some(ns) = word.strip_prefix('@') {
            item.id.split(':').next().is_some_and(|n| n.starts_with(ns))
        } else if let Some(tag) = word.strip_prefix('#') {
            item.tags.iter().any(|t| t.contains(tag))
        } else {
            item.name.to_lowercase().contains(&word) || item.id.contains(&word)
        }
    })
}

/// Whether recipe `r` makes (or, for uses, takes) the query's item.
fn wanted(r: &NativeRecipe, q: Query) -> bool {
    if q.uses {
        r.slots.iter().flatten().any(|s| s.item == q.item)
    } else {
        r.results.iter().any(|s| s.item == q.item)
    }
}

fn stacks(recipes: &NativeRecipes, list: &[NativeRecipeStack]) -> Vec<NativeStack> {
    list.iter()
        .filter_map(|s| {
            let item = recipes.items.get(s.item as usize)?;
            Some(NativeStack {
                item: item.id.clone(),
                count: s.count.max(1),
                components: s.components.clone(),
            })
        })
        .collect()
}

/// A recipe as the panel draws it.
fn view(recipes: &NativeRecipes, r: &NativeRecipe, crafting: bool) -> NativeRecipeView {
    let (layout, width, height) = match r.kind {
        NativeRecipeKind::CraftingShaped => {
            (NativeRecipeLayout::Grid, r.width.max(1), r.height.max(1))
        }
        NativeRecipeKind::CraftingShapeless => {
            let side = shapeless_side(r.slots.len());
            (NativeRecipeLayout::Grid, side, side)
        }
        NativeRecipeKind::Smelting
        | NativeRecipeKind::Blasting
        | NativeRecipeKind::Smoking
        | NativeRecipeKind::Campfire => (NativeRecipeLayout::Cooking, 1, 1),
        NativeRecipeKind::Stonecutting => (NativeRecipeLayout::Single, 1, 1),
        NativeRecipeKind::Smithing => (NativeRecipeLayout::Smithing, 3, 1),
        NativeRecipeKind::Brewing => (NativeRecipeLayout::Brewing, 2, 1),
    };
    let note = if r.cooking_time > 0 {
        cooking_note(r.cooking_time, r.experience)
    } else {
        String::new()
    };
    let craftable = matches!(
        r.kind,
        NativeRecipeKind::CraftingShaped | NativeRecipeKind::CraftingShapeless
    ) && r.book_id >= 0;
    NativeRecipeView {
        layout,
        width,
        height,
        slots: r.slots.iter().map(|s| stacks(recipes, s)).collect(),
        results: stacks(recipes, &r.results),
        note,
        action: craftable.then_some(crafting),
    }
}

/// The smallest square grid a shapeless recipe of `n` ingredients fits.
fn shapeless_side(n: usize) -> u32 {
    match n {
        0 | 1 => 1,
        2..=4 => 2,
        _ => 3,
    }
}

/// `10.0s, 0.1 XP`, as JEI notes a cooking recipe.
fn cooking_note(ticks: u32, xp: f32) -> String {
    let seconds = ticks as f32 / 20.0;
    if xp > 0.0 {
        format!("{seconds:.1}s, {xp} XP")
    } else {
        format!("{seconds:.1}s")
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_hooks::NativeItem;
    use super::*;

    fn item(id: &str, name: &str, tags: &[&str]) -> NativeItem {
        NativeItem {
            id: id.into(),
            name: name.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn search_reads_names_ids_mods_and_tags() {
        let planks = item("minecraft:oak_planks", "Oak Planks", &["minecraft:planks"]);
        assert!(matches(&planks, ""));
        assert!(matches(&planks, "oak"));
        assert!(matches(&planks, "PLANKS"));
        assert!(matches(&planks, "oak planks"));
        assert!(!matches(&planks, "birch"));
        assert!(matches(&planks, "@mine"));
        assert!(!matches(&planks, "@create"));
        assert!(matches(&planks, "#planks"));
        assert!(!matches(&planks, "#logs"));
    }

    #[test]
    fn recipes_are_found_by_result_or_ingredient() {
        let stack = |item| NativeRecipeStack {
            item,
            count: 1,
            components: String::new(),
        };
        let r = NativeRecipe {
            id: "minecraft:stick".into(),
            kind: NativeRecipeKind::CraftingShaped,
            category: String::new(),
            group: String::new(),
            width: 1,
            height: 2,
            slots: vec![vec![stack(0)], vec![stack(0)]],
            results: vec![NativeRecipeStack {
                count: 4,
                ..stack(1)
            }],
            cooking_time: 0,
            experience: 0.0,
            book_id: 7,
        };
        assert!(wanted(
            &r,
            Query {
                item: 1,
                uses: false
            }
        ));
        assert!(!wanted(
            &r,
            Query {
                item: 0,
                uses: false
            }
        ));
        assert!(wanted(
            &r,
            Query {
                item: 0,
                uses: true
            }
        ));
        let recipes = NativeRecipes {
            items: vec![
                item("minecraft:oak_planks", "Oak Planks", &[]),
                item("minecraft:stick", "Stick", &[]),
            ],
            recipes: vec![r.clone()],
        };
        let v = view(&recipes, &r, true);
        assert_eq!(v.layout, NativeRecipeLayout::Grid);
        assert_eq!((v.width, v.height), (1, 2));
        assert_eq!(v.results[0].item, "minecraft:stick");
        assert_eq!(v.results[0].count, 4);
        assert_eq!(v.action, Some(true));
        assert_eq!(
            view(&recipes, &r, false).action,
            Some(false),
            "greyed without a grid"
        );
    }

    #[test]
    fn small_details() {
        assert_eq!(shapeless_side(1), 1);
        assert_eq!(shapeless_side(3), 2);
        assert_eq!(shapeless_side(9), 3);
        assert_eq!(cooking_note(200, 0.1), "10.0s, 0.1 XP");
        assert_eq!(cooking_note(100, 0.0), "5.0s");
    }
}
