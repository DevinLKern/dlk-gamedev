/*
use std::atomic::AtomicU32;
use std::sync::Arc;

pub struct Job {
    task: Box<dyn FnOnce() + Send>
}

pub impl Job {
    
}

pub struct Scheduler {
    dependencies: types::Array2d<bool>, // if get(job1_id, job2_id) then job1 depends on job2
    jobs: Vec<Job>
}

pub impl Scheduler {
    pub fn schedule(&mut self, job: Job, wait_for: Option<&[usize]>) -> usize {
        let id1 = self.jobs.len();
        self.jobs.push(job);

        for id2 wait_for.iter() {
            if let Some(b) = self.dependencies.get_mut(id1, id2) {
                b = true;
            } else {
                // invalid usage
            }
        }
    }
    pub fn run_jobs(&self) {
        let (n, _) = self.dependencies.len();
        
        //
    }
}
*/
