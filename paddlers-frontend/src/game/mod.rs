pub (crate) mod buildings;
pub (crate) mod units;
pub (crate) mod movement;
pub (crate) mod map;
pub (crate) mod town;
pub (crate) mod town_resources;
pub (crate) mod fight;
pub (crate) mod components;
pub (crate) mod forestry;

use crate::prelude::*;
use crate::game::units::worker_factory::create_worker_entities;
use crate::game::units::workers::Worker;
use crate::game::components::*;
use crate::gui::{
    input::{self, UiView},
    render::*,
    sprites::*,
    animation::AnimationState,
    menu::buttons::MenuButtons,
};
use crate::net::{
    NetMsg, 
    game_master_api::RestApiState,
};
use crate::logging::{
    ErrorQueue,
    AsyncErr,
    text_to_user::TextBoard,
    statistics::Statistician,
};

use input::{UiState, Clickable, DefaultShop, pointer::PointerManager};
use movement::*;
use quicksilver::prelude::*;
use specs::prelude::*;
use town::{Town, TOWN_RATIO};
use units::attackers::{Attacker};
use fight::*;
use forestry::*;
use std::sync::mpsc::{Receiver, channel};
use town_resources::TownResources;
use units::worker_system::WorkerSystem;
use map::VillageMetaInfo;

const MENU_BOX_WIDTH: f32 = 300.0;

pub(super) mod resources {
    use crate::prelude::*;

    #[derive(Default)]
    pub struct ClockTick(pub u32);
    #[derive(Default)]
    pub struct UnitLength(pub f32);
    #[derive(Default)]
    pub struct Now(pub Timestamp);
}
use resources::*;

pub(crate) struct Game<'a, 'b> {
    dispatcher: Dispatcher<'a, 'b>,
    pointer_manager: PointerManager<'a, 'b>,
    pub world: World,
    pub sprites: Asset<Sprites>,
    pub font: Asset<Font>,
    pub bold_font: Asset<Font>,
    pub unit_len: Option<f32>,
    pub resources: TownResources,
    net: Option<Receiver<NetMsg>>,
    time_zero: Timestamp,
    total_updates: u64,
    async_err_receiver: Receiver<PadlError>,
    stats: Statistician,
    map: map::GlobalMap,
}

impl Game<'static, 'static> {
    fn with_town(mut self, town: Town) -> Self {
        self.world.insert(town);
        self
    }
    fn with_unit_length(mut self, ul: f32) -> Self {
        self.unit_len = Some(ul);
        self.world.insert(UnitLength(ul));
        self
    }
    fn with_menu_box_area(mut self, area: Rectangle) -> Self {
        {
            self.world.insert(DefaultShop::new());
            let mut data = self.world.write_resource::<UiState>();
            (*data).menu_box_area = area;
        } 
        self
    }
    fn with_network_chan(mut self, net_receiver: Receiver<NetMsg>) -> Self {
        self.net = Some(net_receiver);
        self
    }
}

impl State for Game<'static, 'static> {
    fn new() -> Result<Self> {
        let mut world = init_world();
        let (err_send, err_recv) = channel();
        let err_send_clone = err_send.clone();
        world.insert(ClockTick(0));
        world.insert(UiState::default());
        world.insert(Now);
        world.insert(ErrorQueue::default());
        world.insert(AsyncErr::new(err_send));
        world.insert(TownResources::default());
        world.insert(RestApiState::new(err_send_clone));
        world.insert(TextBoard::default());
        world.insert(MenuButtons::new());

        let mut dispatcher = DispatcherBuilder::new()
            .with(WorkerSystem, "work", &[])
            .with(MoveSystem, "move", &["work"])
            .with(FightSystem::default(), "fight", &["move"])
            .with(ForestrySystem, "forest", &[])
            .build();
        dispatcher.setup(&mut world);

        let pm = PointerManager::init(&mut world);
        let now = utc_now();

        Ok(Game {
            dispatcher: dispatcher,
            pointer_manager: pm,
            world: world,
            sprites: Sprites::new(),
            font: Asset::new(Font::load("fonts/Manjari-Regular.ttf")),
            bold_font: Asset::new(Font::load("fonts/Manjari-Bold.ttf")),
            unit_len: None,
            net: None,
            time_zero: now,
            resources: TownResources::default(),
            total_updates: 0,
            async_err_receiver: err_recv,
            stats: Statistician::new(now),
            map: map::GlobalMap::new_test(),
        })
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        self.total_updates += 1;
        window.set_draw_rate(33.3); // 33ms delay between frames  => 30 fps
        window.set_max_updates(1); // 1 update per frame is enough
        // window.set_fullscreen(true);
        self.update_time_reference();
        {
            let now = self.world.read_resource::<Now>().0;
            self.pointer_manager.run(&mut self.world, now)
        }

        {
            let mut tick = self.world.write_resource::<ClockTick>();
            *tick = ClockTick(tick.0 + 1);
        }
        {
            let mut q = self.world.write_resource::<ErrorQueue>();
            let mut t = self.world.write_resource::<TextBoard>();
            q.pull_async(&mut self.async_err_receiver, &mut t);
            q.run(&mut t);
        }
        {
            let mut res = self.world.write_resource::<TownResources>();
            *res = self.resources;
        }
        {
            use std::sync::mpsc::TryRecvError;
            match self.net.as_ref().unwrap().try_recv() {
                Ok(msg) => {
                    // println!("Received Network data!");
                    match msg {
                        NetMsg::Error(msg) => {
                            println!("Network Error: {}", msg);
                        }
                        NetMsg::Attacks(response) => {
                            if let Some(data) = response.data {
                                for atk in data.village.attacks {
                                    atk.create_entities(&mut self.world, self.unit_len.unwrap());
                                }
                            }
                            else {
                                println!("No data returned");
                            }
                        }
                        NetMsg::Buildings(response) => {
                            if let Some(data) = response.data {
                                data.create_entities(self);
                            }
                            else {
                                println!("No buildings available");
                            }
                        }
                        NetMsg::Map(response) => {
                            if let Some(data) = response.data {
                                let streams = data.map.streams.iter()
                                    .map(
                                        |s| {
                                            s.control_points
                                                .chunks(2)
                                                .map(|slice| (slice[0] as f32, slice[1] as f32))
                                                .collect()
                                        }
                                    )
                                    .collect();
                                let villages = data.map.villages.into_iter().map(VillageMetaInfo::from).collect();
                                self.map = map::GlobalMap::new(streams, villages);
                            }
                            else {
                                println!("No map data available");
                            }
                        },
                        NetMsg::Resources(response) => {
                            if let Some(data) = response.data {
                                self.resources.update(data);
                            }
                            else {
                                println!("No resources available");
                            }
                        }
                        NetMsg::Workers(response) => {
                            let now = self.world.read_resource::<Now>().0;
                            create_worker_entities(&response, &mut self.world, now);
                        }
                        NetMsg::UpdateWorkerTasks(unit) => {
                            let e = self.entity_by_net_id(unit.id.parse().unwrap());
                            if let Some(entity) = e {
                                let workers = &mut self.world.write_storage::<Worker>();
                                let worker = workers.get_mut(entity).unwrap();
                                worker.tasks.clear();
                                for task in unit.tasks {
                                    worker.tasks.push_back((&task).into());
                                }
                            }
                            else {
                                println!("Network error: Unknown worker entity");
                            }
                        }
                    }
                },
                Err(TryRecvError::Disconnected) => { println!("Network connection is dead.")},
                Err(TryRecvError::Empty) => {},
            }
        }
        self.update_time_reference();
        self.dispatcher.dispatch(&mut self.world);
        if self.total_updates % 300 == 15 {
            self.reaper(&Rectangle::new_sized(window.screen_size()));
        }
        self.world.maintain();
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        let tick = self.world.read_resource::<ClockTick>().0;
        let now = utc_now();
        {
            let mut rest = self.world.write_resource::<RestApiState>();
            let err = self.stats.run(&mut *rest, now);
            self.check(err);
        }

        let ui_state = self.world.read_resource::<UiState>();
        let hovered_entity = ui_state.hovered_entity;
        let grabbed_item = ui_state.grabbed_item.clone();
        let view = ui_state.current_view;
        let main_area = Rectangle::new(
            (0,0), 
            (ui_state.menu_box_area.x(), window.screen_size().y)
        );
        std::mem::drop(ui_state);
        window.clear(Color::WHITE)?;
        match view {
            UiView::Town => {
                {
                    let (asset, town, ul) = (&mut self.sprites, &self.world.read_resource::<Town>(), self.unit_len.unwrap());
                    asset.execute(|sprites| town.render(window, sprites, tick, ul))?;
                }
                self.render_entities(window)?;
            },
            UiView::Map => {
                let (sprites, map) = (&mut self.sprites, &mut self.map);
                map.render(window, sprites, &main_area)?;
            }
        }
        
        self.render_menu_box(window)?;
        self.render_text_messages(window)?;

        if let Some(entity) = hovered_entity {
            self.render_hovering(window, entity)?;
        }
        if let Some(grabbed) = grabbed_item {
            self.render_grabbed_item(window, &grabbed)?;
        }
        Ok(())
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        // println!("Event: {:?}", event);
        // {
        //     let mut t = self.world.write_resource::<TextBoard>();
        //     t.display_debug_message(format!("{:?}", event));
        // }
        match event {
            Event::MouseMoved(pos) => {
                self.pointer_manager.move_pointer(&mut self.world, &pos);
            },
            Event::MouseButton(button, state)
            => {
                let now = self.world.read_resource::<Now>().0;
                let pos = &window.mouse().pos();
                self.pointer_manager.button_event(now, pos, *button, *state);  
            }
            Event::Key(key, state) 
                if *key == Key::Escape && *state == ButtonState::Pressed =>
                {
                    let mut ui_state = self.world.write_resource::<UiState>();
                    if (*ui_state).grabbed_item.is_some(){
                        (*ui_state).grabbed_item = None;
                    } else {
                        (*ui_state).selected_entity = None;
                    }
                },
            Event::Key(key, state) 
                if *key == Key::Delete && *state == ButtonState::Pressed =>
                {
                    let mut ui_state = self.world.write_resource::<UiState>();
                    if let Some(e) = ui_state.selected_entity {
                        (*ui_state).selected_entity = None;
                        std::mem::drop(ui_state);

                        let pos_store = self.world.read_storage::<Position>();
                        let pos = pos_store.get(e).unwrap();
                        let tile_index = self.town().tile(pos.area.center());
                        std::mem::drop(pos_store);

                        let r = self.rest().http_delete_building(tile_index);
                        self.check(r);

                        self.town_mut().remove_building(tile_index);
                        self.world.delete_entity(e)
                            .unwrap_or_else(
                                |_|
                                self.check(
                                    PadlErrorCode::DevMsg("Tried to delete wrong Generation").dev()
                                ).unwrap()
                            );
                    }
                },
            Event::Key(key, state) 
                if *key == Key::Tab && *state == ButtonState::Pressed =>
                {
                    let mut ui_state = self.world.write_resource::<UiState>();
                    ui_state.toggle_view();
                },
            _evt => {
                // println!("Event: {:#?}", _evt)
            }
        };
        self.world.maintain();
        Ok(())
    }
}

impl Game<'_,'_> {
    pub fn town(&self) -> specs::shred::Fetch<Town> {
        self.world.read_resource()
    }
    pub fn rest(&mut self) -> specs::shred::FetchMut<RestApiState> {
        self.world.write_resource()
    }
    pub fn town_mut(&mut self) -> specs::shred::FetchMut<Town> {
        self.world.write_resource()
    }
    fn update_time_reference(&mut self) {
        if self.time_zero != 0 {
            let t = utc_now();
            let mut ts = self.world.write_resource::<Now>();
            *ts = Now(t);
        }
    }
    /// Removes entites outside the map
    fn reaper(&mut self, map: &Rectangle) {
        let p = self.world.read_storage::<Position>();
        let mut dead = vec![];
        for (entity, position) in (&self.world.entities(), &p).join() {
            if !position.area.overlaps_rectangle(map)  {
                dead.push(entity);
            }
        }
        std::mem::drop(p);
        self.world.delete_entities(&dead).expect("Something bad happened when deleting dead entities");
    }
    fn entity_by_net_id(&self, net_id: i64) -> Option<Entity> {
        // TODO: Efficient NetId lookup
        let net = self.world.read_storage::<NetObj>();
        let ent = self.world.entities();
        for (e, n) in (&ent, &net).join() {
            if n.id == net_id {
                return Some(e);
            }
        }
        None
    }
    fn check<R>(&self, res: PadlResult<R>) -> Option<R> {
        if let Err(e) = res {
            let mut q = self.world.write_resource::<ErrorQueue>();
            q.push(e);
            None
        }
        else {
            Some(res.unwrap())
        }
    }
}

fn init_world() -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Moving>();
    world.register::<Renderable>();
    world.register::<Clickable>();
    world.register::<Attacker>();
    world.register::<Worker>();
    world.register::<Range>();
    world.register::<Health>();
    world.register::<NetObj>();
    world.register::<AnimationState>();
    world.register::<EntityContainer>();
    world.register::<ForestComponent>();

    world
}

pub fn run(width: f32, height: f32, net_chan: Receiver<NetMsg>) {
    let max_town_width = width - MENU_BOX_WIDTH;
    let (tw, th) = if max_town_width / height <= TOWN_RATIO {
        (max_town_width, max_town_width / TOWN_RATIO)
    } else {
        (TOWN_RATIO * height, height)
    };

    let ul = tw / town::X as f32;
    let menu_box_area = Rectangle::new((tw,0),(MENU_BOX_WIDTH, th));
    quicksilver::lifecycle::run_with::<Game, _>(
        "Paddlers", 
        Vector::new(tw + MENU_BOX_WIDTH, th), 
        Settings::default(), 
        || Ok(
            Game::new().expect("Game initialization")
                .with_town(Town::new(ul)) // TODO: Think of a better way to handle unit lengths in general
                .with_unit_length(ul)
                .with_menu_box_area(menu_box_area)
                .with_network_chan(net_chan)
            )
    );
}
