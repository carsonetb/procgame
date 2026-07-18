use bevy::prelude::*;

#[derive(Resource, Debug, Default, Deref, DerefMut, Clone)]
pub struct Controllers(Vec<Entity>);

#[derive(Debug, Default, Clone)]
pub struct Action {
    pub horizontal: f32,
    pub vertical: f32,
    pub grab_drop: bool,
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(Controllers::default());
}

pub fn track_controllers(
    mut controllers: ResMut<Controllers>,
    new_gamepads: Query<Entity, Added<Gamepad>>,
    mut disconnected: RemovedComponents<Gamepad>,
) {
    for entity in new_gamepads {
        if !controllers.contains(&entity) {
            controllers.push(entity);
            info!("Controller connected (index {})", controllers.len());
        }
    }

    for entity in disconnected.read() {
        if let Some(index) = controllers.iter().position(|&e| e == entity) {
            controllers.remove(index);
            info!("Controller disconnected (index {})", index + 1);
        }
    }
}

pub fn get_action(
    keyboard: &Res<ButtonInput<KeyCode>>,
    controllers: &Res<Controllers>,
    q_gamepads: &Query<&Gamepad>,
    index: usize,
) -> Action {
    let mut action = Action::default();

    if index == 0 {
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            action.horizontal -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            action.horizontal += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            action.vertical -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            // TODO: Might want to override down
            action.vertical += 1.0;
        }
        action.grab_drop = keyboard.just_pressed(KeyCode::KeyE);

        return action;
    }

    if index as i32 - 1 > controllers.len() as i32 - 1 {
        warn!("More players than controllers.");
        return action;
    }

    let entity = controllers[index];
    let gamepad = q_gamepads.get(entity).unwrap();

    action.horizontal += gamepad.left_stick().x;
    action.vertical += gamepad.left_stick().y;

    action.grab_drop = gamepad.just_pressed(GamepadButton::West);

    action
}
