mod app;
mod state;
mod input;
mod graphics;
mod model;

mod resources;

pub use app::App;



// Setup logging and run the event loop
pub fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}