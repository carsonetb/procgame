use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum AuditoryEventType {
    Walk,
    Scuttle,
}

#[derive(Debug, Clone, Copy)]
pub enum VisualEventType {
    Food { size: f32, color: Color },
}

#[derive(Debug, Clone, Copy)]
pub enum InternalEventType {
    Hunger { intensity: f32 },
    Movement { amount: Vec2 },
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Auditory(AuditoryEventType),
    Visual(VisualEventType),
    Internal(InternalEventType),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SenseEvent {
    pub event_type: EventType,
    pub position: Vec2,
    pub intensity: f32,
    decay_speed: f32,
}

impl SenseEvent {
    pub fn new(typ: EventType, position: Vec2, intensity: f32, decay_speed: f32) -> Self {
        Self {
            event_type: typ,
            position,
            intensity,
            decay_speed,
        }
    }

    pub fn internal(typ: EventType) -> Self {
        Self::new(typ, Vec2::new(0.0, 0.0), 0.0, 0.0)
    }
}

pub fn process(time: Res<Time>, query: Query<(Entity, &mut SenseEvent)>, mut commands: Commands) {
    for (entity, mut event) in query {
        event.intensity -= event.decay_speed * time.delta_secs();
        if event.intensity <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
