use bevy::{persist::PersistApp, prelude::*, winit::WinitConfig, app::AppExit};

#[no_mangle]
pub fn __bevy_the_game(mut app: PersistApp) {
    println!("loaded");
    app.add_resource(WinitConfig {
        return_from_run: true,
    })
    .add_serde_preserve_resource(Scoreboard { score: 0 })
    .add_system(click_handler.system())
    .set_runner(|mut app: App| {
        app.initialize();
        let mut app_exit_event_reader = EventReader::<AppExit>::default();
        loop {
            app.update();
            if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
            if app_exit_event_reader.latest(&app_exit_events).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        }
    })
    .run();
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Scoreboard {
    score: usize,
}

fn click_handler(/*key_input: Res<Input<KeyCode>>, */ mut score: ResMut<Scoreboard>) {
    //println!("a");
    //if key_input.just_pressed(KeyCode::Space) {
    score.score += 1;
    println!("a{}", score.score);
    //}
}
