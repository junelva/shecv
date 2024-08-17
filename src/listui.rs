use std::{cell::RefCell, rc::Rc};

use crate::types::{ColorRGBA, ListItemData, Value};

pub struct ListStyle {
    pub bg: ColorRGBA,

    pub li_selected: ColorRGBA,
    pub li_selected_bg: ColorRGBA,

    pub li_unselected: ColorRGBA,
    pub li_unselected_bg: ColorRGBA,

    pub li_activated: ColorRGBA,
    pub li_activated_bg: ColorRGBA,

    pub li_disabled: ColorRGBA,
    pub li_disabled_bg: ColorRGBA,
}

impl ListStyle {
    fn default() -> ListStyle {
        ListStyle {
            bg: ColorRGBA::grey_darkest(),
            li_selected: ColorRGBA::white(),
            li_selected_bg: ColorRGBA::grey_medium(),
            li_activated: ColorRGBA::white(),
            li_activated_bg: ColorRGBA::grey_lighter(),
            li_unselected: ColorRGBA::grey_light(),
            li_unselected_bg: ColorRGBA::grey_dark(),
            li_disabled: ColorRGBA::grey_dark(),
            li_disabled_bg: ColorRGBA::grey_darker(),
        }
    }
}

#[derive(Default)]
pub enum ListPopoutBehavior {
    #[default]
    AlwaysVisible,
    HiddenWhenUnfocused,
}

pub struct ListPopoutState {
    behavior: ListPopoutBehavior,
    speed: f32,
    delta: f32,
}

impl Default for ListPopoutState {
    fn default() -> Self {
        ListPopoutState {
            behavior: ListPopoutBehavior::AlwaysVisible,
            speed: 1.0,
            delta: 1.0,
        }
    }
}

// A ListInterface provides navigation of a vertical list of items.
pub struct ListInterface {
    pub style: ListStyle,
    pub anchor: ListAnchor,
    pub focused: bool,
    pub popout: ListPopoutState,
    pub resume: ListResumeBehavior,
    pub selected_index: i32,
    pub entries: Vec<ListItem>,
    pub render_group_index: usize,
}

// ListInterface implements custom rendering.
// Complications include styling and animation.
// To facilitate this, it holds a state machine
// and renders in immediate mode.
impl ListInterface {
    pub fn default(render_group_index: usize) -> Self {
        Self {
            style: ListStyle::default(),
            anchor: ListAnchor::Left,
            focused: false,
            popout: ListPopoutState::default(),
            resume: ListResumeBehavior::First,
            selected_index: 0,
            entries: vec![],
            render_group_index,
        }
    }

    pub fn add_labeled_value(&mut self, label: &str, value: Rc<RefCell<Value<dyn ListItemData>>>) {
        self.entries.push(ListItem {
            label: label.to_string(),
            ty: ListItemType::Text,
            selectable: ListItemSelectable::Selectable,
            editable: ListItemEditable::NotEditable,
            value,
        })
    }

    pub fn add_entry(
        &mut self,
        label: &str,
        ty: ListItemType,
        selectable: ListItemSelectable,
        editable: ListItemEditable,
        value: Rc<RefCell<Value<dyn ListItemData>>>,
    ) {
        self.entries.push(ListItem {
            label: label.to_string(),
            ty,
            selectable,
            editable,
            value,
        })
    }
}

// ListInterface can be anchored to left, middle, or right of screen.
// When on a SubList, this determines the opening direction.
// Opening a SubList to Middle causes it to replace the parent.
#[derive(Default, PartialEq, Eq)]
pub enum ListAnchor {
    #[default]
    Left,
    Middle,
    Right,
    Hidden,
}

// When a ListInterface is re-entered, this determines where the cursor starts.
#[derive(Default)]
pub enum ListResumeBehavior {
    #[default]
    First,
    LastUsed,
}

// Basic elements in the UI.
//  - Text is simply a text line.
// Checkbox is a toggle box.
//  - Requires reference to memory value.
// Slider is a numerical value range adjuster.
//  - Requires reference to memory value.
// Button performs a function when activated.
//  - Requires reference to function.
// RowGroup positions multiple list items in a horizontal row.
//  - Requires ListInterface reference.
// SubList causes a submenu to open left or right.
//  - Requires a ListInterface reference; anchor is open direction.
#[derive(Default)]
pub enum ListItemType {
    #[default]
    Text,
    CheckBox,
    Slider,
    Button,
    RowGroup,
    SubList,
}

#[derive(Default)]
pub enum ListItemSelectable {
    #[default]
    Selectable,
    NotSelectable,
}

#[derive(Default)]
pub enum ListItemEditable {
    #[default]
    Editable,
    NotEditable,
}

pub enum OperatorResult {
    Done,
    Cancelled,
    Irrelevant,
}

// ListItems have these options. They also contain data references.
pub struct ListItem {
    pub label: String,
    pub ty: ListItemType,
    pub selectable: ListItemSelectable,
    pub editable: ListItemEditable,
    pub value: Rc<RefCell<Value<dyn ListItemData>>>,
}
