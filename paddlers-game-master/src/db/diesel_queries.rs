use diesel::prelude::*;
use paddlers_shared_lib::prelude::*;
use paddlers_shared_lib::schema::*;
use paddlers_shared_lib::models::dsl;
use super::*;

impl DB {

    pub fn delete_hobo(&self, hobo: &Hobo) {
        let result = diesel::delete(hobo).execute(self.dbconn());
        if result.is_err() {
            println!("Couldn't delete hobo {:?}", hobo);
        }
    }

    pub fn delete_attack(&self, atk: &Attack) {
        let result = diesel::delete(atk).execute(self.dbconn());
        if result.is_err() {
            println!("Couldn't delete attack {:?}", atk);
        }
    }

    pub fn insert_hobo(&self, u: &NewHobo) -> Hobo {
        diesel::insert_into(hobos::dsl::hobos)
            .values(u)
            .get_result(self.dbconn())
            .expect("Inserting hobo")
    }
    pub fn insert_worker(&self, u: &NewWorker) -> Worker {
        diesel::insert_into(workers::dsl::workers)
            .values(u)
            .get_result(self.dbconn())
            .expect("Inserting worker")
    }
    pub fn update_worker(&self, u: &Worker) {
        diesel::update(u)
            .set(u)
            .execute(self.dbconn())
            .expect("Updating worker");
    }
    pub fn insert_attack(&self, new_attack: &NewAttack) -> Attack {
        diesel::insert_into(attacks::dsl::attacks)
            .values(new_attack)
            .get_result(self.dbconn())
            .expect("Inserting attack")
    }
    pub fn insert_attack_to_hobo(&self, atu: &AttackToHobo) {
        diesel::insert_into(attacks_to_hobos::dsl::attacks_to_hobos)
            .values(atu)
            .execute(self.dbconn())
            .expect("Inserting attack to hobo");
    }
    pub fn insert_resource(&self, res: &Resource) -> QueryResult<usize> {
        diesel::insert_into(dsl::resources)
            .values(res)
            .execute(self.dbconn())
    }
    pub fn add_resource(&self, rt: ResourceType, vk: VillageKey, plus: i64) -> QueryResult<Resource> {
        let target = resources::table.find((rt,vk.num()));
        diesel::update(target)
            .set(resources::amount.eq(resources::amount + plus))
            .get_result(self.dbconn())
    }
    pub fn insert_building(&self, new_building: &NewBuilding) -> Building {
        diesel::insert_into(buildings::dsl::buildings)
            .values(new_building)
            .get_result(self.dbconn())
            .expect("Inserting building")
    }
    pub fn delete_building(&self, building: &Building) {
        diesel::delete(buildings::table
            .filter(buildings::id.eq(building.id)))
            .execute(self.dbconn())
            .expect("Deleting building");
    }
    pub fn insert_task(&self, task: &NewTask) -> Task {
        diesel::insert_into(tasks::dsl::tasks)
            .values(task)
            .get_result(self.dbconn())
            .expect("Inserting task")
    }
    
    pub fn insert_tasks(&self, tasks: &[NewTask]) -> Vec<Task> {
        diesel::insert_into(tasks::dsl::tasks)
            .values(tasks)
            .get_results(self.dbconn())
            .expect("Inserting tasks")
    }
    pub fn update_task(&self, t: &Task) {
        diesel::update(t)
            .set(t)
            .execute(self.dbconn())
            .expect("Updating task");
    }
    pub fn delete_task(&self, task: &Task) {
        diesel::delete(tasks::table
            .filter(tasks::id.eq(task.id)))
            .execute(self.dbconn())
            .expect("Deleting task");
    }
    pub fn flush_task_queue(&self, worker_id: i64) {
        diesel::delete(tasks::table
            .filter(tasks::worker_id.eq(worker_id)))
            .filter(tasks::start_time.gt(diesel::dsl::now.at_time_zone("UTC")))
            .execute(self.dbconn())
            .expect("Deleting task");
    }
    pub fn insert_streams(&self, streams: &[NewStream]) -> Vec<Stream> {
        diesel::insert_into(streams::dsl::streams)
            .values(streams)
            .get_results(self.dbconn())
            .expect("Inserting streams")
    }
    pub fn insert_villages(&self, villages: &[NewVillage]) -> Vec<Village> {
        diesel::insert_into(villages::dsl::villages)
            .values(villages)
            .get_results(self.dbconn())
            .expect("Inserting villages")
    }
    pub fn insert_ability(&self, a: &NewAbility) -> Ability {
        diesel::insert_into(abilities::dsl::abilities)
            .values(a)
            .get_result(self.dbconn())
            .expect("Inserting ability")
    }
}