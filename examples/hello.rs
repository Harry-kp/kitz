use rataframe::prelude::*;

struct App;

impl Application for App {
    type Message = ();

    fn update(&mut self, _msg: (), _ctx: &mut Context<()>) -> Command<()> {
        Command::quit()
    }

    fn view(&self, frame: &mut Frame, _ctx: &ViewContext) {
        frame.render_widget(
            Paragraph::new("Hello, rataframe! Press any key to quit."),
            frame.area(),
        );
    }

    fn handle_event(&self, _event: &Event, _ctx: &EventContext) -> EventResult<()> {
        EventResult::Message(())
    }
}

fn main() -> Result<()> {
    rataframe::run(App)
}
