use bevy::prelude::*;

pub fn ui() -> impl Scene {
    bsn! {
        Node {
            width: percent(20),
            height: percent(20),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            column_gap: px(8),
        }
        Children [
            Text("Spawn Player: Mouse middle") TextFont { font_size: px(15.0) },
            Text("Spawn Enemy: X") TextFont { font_size: px(15.0) },
            Text("Spawn Event: P") TextFont { font_size: px(15.0) },
            Text("Pickup/Drop: E") TextFont { font_size: px(15.0) },
            Text("Debug Plants: T") TextFont { font_size: px(15.0) },
            Text("Debug Attractors: LShift") TextFont { font_size: px(15.0) },
            Text("Spawn Tree: R") TextFont { font_size: px(15.0) },
            Text("Generate Heightmap: H") TextFont { font_size: px(15.0) },
            Text("Bitmap Edit: Mouse L/R") TextFont { font_size: px(15.0) },
            Text("Save Tilemap: K") TextFont { font_size: px(15.0) },
        ]
    }
}
