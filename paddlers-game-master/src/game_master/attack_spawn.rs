//! Spawns random attacks on villages

use crate::db::*;
use crate::game_master::attack_funnel::{AttackFunnel, PlannedAttack};
use actix::prelude::*;
use futures::future::join_all;
use paddlers_shared_lib::game_mechanics::hobos::HoboLevel;
use paddlers_shared_lib::prelude::*;
use rand::Rng;

pub struct AttackSpawner {
    dbpool: Pool,
    db_actor: Addr<DbActor>,
    attack_funnel_actor: Addr<AttackFunnel>,
}

impl Actor for AttackSpawner {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        println!("Attack Spawner is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        println!("Attack Spawner is stopped");
    }
}

/// Attack from no specific origin
pub(super) struct SendAnarchistAttack {
    pub village: VillageKey,
    pub level: HoboLevel,
}
impl Message for SendAnarchistAttack {
    type Result = ();
}
impl Handler<SendAnarchistAttack> for AttackSpawner {
    type Result = ();

    fn handle(&mut self, msg: SendAnarchistAttack, _ctx: &mut Context<Self>) -> Self::Result {
        self.spawn_anarchists(msg.village, msg.level);
    }
}

impl AttackSpawner {
    pub fn new(
        dbpool: Pool,
        db_actor: Addr<DbActor>,
        attack_funnel_actor: Addr<AttackFunnel>,
    ) -> Self {
        AttackSpawner {
            dbpool,
            db_actor,
            attack_funnel_actor,
        }
    }
    fn db(&self) -> DB {
        (&self.dbpool).into()
    }

    fn spawn_anarchists(&self, village: VillageKey, level: HoboLevel) {
        // Send a random number of weak and hurried hobos + 1 stronger which is nor hurried
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(2, 4);

        // weak hobos
        let (min_hp, max_hp) = level.hurried_anarchist_hp_range();
        let mut futures: Vec<Request<DbActor, NewHoboMessage>> = (0..n)
            .map(|_| {
                let hobo = NewHobo {
                    color: Some(Self::gen_color(&mut rng)),
                    hp: rng.gen_range(min_hp, max_hp),
                    speed: 0.0625,
                    home: village.num(), // TODO: anarchists home
                    hurried: true,
                    nest: None,
                };
                let msg = NewHoboMessage(hobo);
                self.db_actor.send(msg)
            })
            .collect();
        // strong unhurried hobo
        futures.push({
            let hobo = NewHobo {
                color: Some(Self::gen_color(&mut rng)),
                hp: level.unhurried_anarchist_hp(),
                speed: 0.25,
                home: village.num(), // TODO: anarchists home
                hurried: false,
                nest: None,
            };
            let msg = NewHoboMessage(hobo);
            self.db_actor.send(msg)
        });

        let attack_funnel = self.attack_funnel_actor.clone();
        let pool = self.dbpool.clone();
        Arbiter::spawn(
            join_all(futures)
                .map_err(|e| eprintln!("Attack spawn failed: {:?}", e))
                .map(move |hobos| {
                    let hobos = hobos.into_iter().map(|h| h.0).collect();
                    let db: DB = (&pool).into();

                    let pa = PlannedAttack {
                        origin_village: None,
                        destination_village: db.village(village).unwrap(),
                        hobos: hobos,
                    };
                    attack_funnel.try_send(pa).expect("Spawning attack failed");
                }),
        );
    }

    fn gen_color<R>(rng: &mut R) -> UnitColor
    where
        R: Rng,
    {
        match rng.gen_range(0, 100) {
            0..85 => UnitColor::Yellow,
            85..99 => UnitColor::Camo,
            99 => UnitColor::White,
            _ => panic!("RNG bug?"),
        }
    }
}
