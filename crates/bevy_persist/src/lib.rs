use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy_app::{prelude::*, AppExit};
use bevy_ecs::{bevy_utils::HashMap, prelude::*};
use notify::{RecursiveMode, Watcher};

#[cfg(feature = "dynamic")]
pub fn load_game(name: &str) {
    use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};

    let persist_context_inner = Arc::new(Mutex::new(PersistContextInner {
        should_reload: false,
        serde_resources: HashMap::default(),
    }));

    let lib_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("{}{}{}", DLL_PREFIX, name, DLL_SUFFIX));

    let persist_context_inner2 = persist_context_inner.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(_) = rx.recv() {
            persist_context_inner2.lock().unwrap().should_reload = true;
        }
    });

    let mut watcher = notify::watcher(tx, Duration::new(0, 0)).unwrap();
    watcher
        .watch(&lib_path, RecursiveMode::NonRecursive)
        .unwrap();

    loop {
        persist_context_inner.lock().unwrap().should_reload = false;
        let game = libloading::Library::new(&lib_path).unwrap();
        unsafe {
            let func: libloading::Symbol<fn(PersistApp)> = game.get(b"__bevy_the_game").unwrap();
            func(PersistContextInner::new_app(&persist_context_inner));
        }
    }
}

pub struct PersistContext {
    serde_resources_save: Vec<Box<dyn FnOnce(&mut Resources) + Send + Sync>>,
    inner: Arc<Mutex<PersistContextInner>>,
}

struct PersistContextInner {
    should_reload: bool,
    serde_resources: HashMap<&'static str, Vec<u8>>,
}

pub struct PersistApp {
    app: AppBuilder,
}

impl PersistApp {
    pub fn add_resource<T: Send + Sync + 'static>(&mut self, res: T) -> &mut Self {
        self.app.add_resource(res);
        self
    }

    pub fn add_serde_preserve_resource<T>(&mut self, mut res: T) -> &mut Self
    where
        T: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync + 'static,
    {
        let mut ctx = self
            .app
            .resources_mut()
            .get_mut::<PersistContext>()
            .unwrap();
        ctx.serde_resources_save.push(Box::new(|res| {
            let serialized = bincode::serialize(&*res.get::<T>().unwrap()).unwrap();
            res.get::<PersistContext>()
                .unwrap()
                .inner
                .lock()
                .unwrap()
                .serde_resources
                .insert(std::any::type_name::<T>(), serialized);
        }));

        if let Some(serialized) = ctx
            .inner
            .lock()
            .unwrap()
            .serde_resources
            .get(std::any::type_name::<T>())
        {
            res = bincode::deserialize(serialized).unwrap();
        }
        std::mem::drop(ctx);
        self.app.add_resource(res);
        self
    }

    pub fn add_system(&mut self, system: Box<dyn System>) -> &mut Self {
        self.app.add_system(system);
        self
    }

    pub fn set_runner(&mut self, runner: impl Fn(App) + 'static) -> &mut Self {
        self.app.set_runner(runner);
        self
    }

    pub fn run(&mut self) {
        self.app.run();
    }
}

impl PersistContextInner {
    fn new_app(this: &Arc<Mutex<Self>>) -> PersistApp {
        let mut app = App::build();
        app.add_system_to_stage_front(stage::FIRST, probe_for_reload.thread_local_system());
        app.add_resource(PersistContext {
            inner: this.clone(),
            serde_resources_save: vec![],
        });
        PersistApp { app }
    }
}

fn probe_for_reload(_: &mut World, res: &mut Resources) {
    let should_reload = res
        .get::<PersistContext>()
        .unwrap()
        .inner
        .lock()
        .unwrap()
        .should_reload;
    if should_reload {
        let serde_resources_save = std::mem::take(
            &mut res
                .get_mut::<PersistContext>()
                .unwrap()
                .serde_resources_save,
        );
        for resource_save in serde_resources_save {
            resource_save(&mut *res);
        }
        dbg!(
            &res.get_mut::<PersistContext>()
                .unwrap()
                .inner
                .lock()
                .unwrap()
                .serde_resources
        );
        res.get_mut::<Events<AppExit>>().unwrap().send(AppExit);
    }
}
