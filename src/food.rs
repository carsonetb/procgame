use bevy::prelude::*;

use crate::environment::{EventType, SenseEvent, VisualEventType};

#[derive(Component)]
pub struct Food {
    pub size: f32,
    pub color: Color,
    pub pos: Vec2,
}

impl Food {
    pub fn setup(commands: &mut Commands, size: f32, color: Color, pos: Vec2) {
        commands.spawn((
            Food { size, color, pos },
            Sprite::from_color(color, Vec2::new(size, size)),
            Transform::from_xyz(pos.x - size / 2.0, pos.y - size / 2.0, 0.0),
            Visibility::default(),
        ));
    }

    pub fn process(mut commands: Commands, query: Query<&Food>) {
        for food in query {
            commands.spawn(SenseEvent::new(
                EventType::Visual(VisualEventType::Food {
                    size: food.size,
                    color: food.color,
                }),
                food.pos,
                1.0,
                100.0,
            ));
        }
    }
}
