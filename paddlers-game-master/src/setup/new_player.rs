use paddlers_shared_lib::prelude::*;
use crate::db::DB;

impl DB {
    pub (super) fn new_player(&self, display_name: String) -> Player {
        let player = NewPlayer {
            display_name: Some(display_name),
            karma: 0,
        };
        let player = self.insert_player(&player);
        let village = self.new_village(player.key());
        self.insert_hero(village.key());
        player
    }


    fn insert_hero(&self, vid: VillageKey) -> Worker {
        let (x,y) = (2,2);
        let worker = NewWorker {
            unit_type: UnitType::Hero,
            x: x,
            y: y,
            color: None,
            speed: 0.5,
            home: vid.num(),
            mana: Some(10),
            level: 1,
            exp: 0,
        };
        let worker = self.insert_worker(&worker);
        let task = NewTask {
            worker_id: worker.id,
            task_type: TaskType::Idle,
            x: x,
            y: y,
            start_time: None,
            target_hobo_id: None,
        };
        self.insert_task(&task);
        let work_ability = NewAbility {
            worker_id: worker.id,
            ability_type: AbilityType::Work,
        };
        self.insert_ability(&work_ability);
        let welcome_ability = NewAbility {
            worker_id: worker.id,
            ability_type: AbilityType::Welcome,
        };
        self.insert_ability(&welcome_ability);
        self.insert_worker_flag(
            WorkerFlag {
                worker_id: worker.id,
                flag_type: WorkerFlagType::ManaRegeneration,
                last_update: chrono::Utc::now().naive_utc(),
            }
        );
        self.insert_worker_flag(
            WorkerFlag {
                worker_id: worker.id,
                flag_type: WorkerFlagType::Work,
                last_update: chrono::Utc::now().naive_utc(),
            }
        );
        worker
    }

    fn new_village(&self, pid: PlayerKey) -> Village{
        let village = self.add_village(pid).expect("Village insertion failed");
        self.insert_initial_resources(village.key());
        village
    }

    fn insert_initial_resources(&self, vid: VillageKey) {
        self.init_resources(vid);
        self.add_resource(ResourceType::Feathers, vid, 50).expect("Adding initial resources");
        self.add_resource(ResourceType::Sticks, vid, 50).expect("Adding initial resources");
        self.add_resource(ResourceType::Logs, vid, 50).expect("Adding initial resources");
    }
}
