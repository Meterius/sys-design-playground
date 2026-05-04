use bevy::app::App;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

#[derive(Default)]
pub struct SettingsPlugin {}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Settings>()
            .insert_resource(Settings { debug_mode: false });

        app.add_systems(Update, handle_keyboard_input);
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct Settings {
    pub debug_mode: bool,
}

impl Settings {
    pub fn in_debug_mode(settings: Option<Res<Settings>>) -> bool {
        settings.is_some_and(|settings| settings.debug_mode)
    }
}

fn handle_keyboard_input(
    mut keyboard: MessageReader<KeyboardInput>,
    mut settings: ResMut<Settings>,
) {
    for input in keyboard.read() {
        if input.key_code == KeyCode::F1 && input.state == ButtonState::Released {
            settings.debug_mode = !settings.debug_mode;
        }
    }
}
