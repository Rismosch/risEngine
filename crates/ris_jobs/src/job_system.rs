use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
};

use crate::{
    errors::{BlockedOrEmpty, IsEmpty},
    job::Job,
    job_buffer::JobBuffer,
};

static DONE: AtomicBool = AtomicBool::new(false);

thread_local! {
    static WORKER_THREAD: RefCell<Option<WorkerThread>> = RefCell::new(None);
}

struct WorkerThread {
    local_buffer: JobBuffer,
    steal_buffers: Vec<JobBuffer>,
    index: usize,
}

pub struct JobSystem {
    handles: Option<Vec<JoinHandle<()>>>,
}

impl JobSystem {
    pub fn new(buffer_capacity: usize, threads: usize) -> Self {
        DONE.store(false, Ordering::SeqCst);

        let mut buffers = Vec::with_capacity(threads);
        for _ in 0..threads {
            buffers.push(JobBuffer::new(buffer_capacity))
        }

        let mut handles = Vec::with_capacity(threads);
        for i in 1..threads {
            let buffers = duplicate_buffers(&mut buffers);
            handles.push(thread::spawn(move || {
                setup_worker_thread(i, buffers);
                run_worker_thread();
            }))
        }

        ris_log::info!("spawned {} worker threads", handles.len());
        let handles = Some(handles);

        let buffers = duplicate_buffers(&mut buffers);
        setup_worker_thread(0, buffers);

        Self { handles }
    }

    pub fn wait_till_done(&mut self) {
        DONE.store(true, Ordering::SeqCst);

        empty_buffer();

        match self.handles.take() {
            Some(handles) => {
                let mut i = 0;
                for handle in handles {
                    i += 1;
                    match handle.join() {
                        Ok(()) => ris_log::trace!("joined thread {}", i),
                        Err(_) => ris_log::fatal!("failed to join thread {}", i),
                    }
                }

                ris_log::info!("job system finished")
            }
            None => ris_log::info!("handles already joined"),
        }
    }
}

pub fn submit<F: FnOnce() + 'static>(job: F) {
    let mut not_pushed = None;

    WORKER_THREAD.with(|worker_thread| {
        if let Some(worker_thread) = worker_thread.borrow_mut().as_mut() {
            let job = Job::new(job);

            match worker_thread.local_buffer.push(job) {
                Ok(()) => (),
                Err(blocked_or_full) => {
                    not_pushed = Some(blocked_or_full.not_pushed);
                }
            }
        } else {
            ris_log::error!("couldn't submit job, calling thread isn't a worker thread");
        }
    });

    if let Some(mut to_invoke) = not_pushed {
        to_invoke.invoke();
    }
}

pub fn run_pending_job() {
    match pop_job() {
        Ok(job) => {
            let mut job = job;
            job.invoke();
        }
        Err(IsEmpty) => match steal_job() {
            Ok(job) => {
                let mut job = job;
                job.invoke();
            }
            Err(BlockedOrEmpty) => thread::yield_now(),
        },
    }
}

pub fn thread_index() -> usize {
    let mut result = 0;

    WORKER_THREAD.with(|worker_thread| {
        if let Some(worker_thread) = worker_thread.borrow().as_ref() {
            result = worker_thread.index;
        } else {
            ris_log::error!("calling thread isn't a worker thread");
            result = usize::MAX;
        }
    });

    result
}

fn duplicate_buffers(buffers: &mut Vec<JobBuffer>) -> Vec<JobBuffer> {
    let mut result = Vec::new();

    for buffer in buffers {
        result.push(buffer.duplicate());
    }

    result
}

fn setup_worker_thread(index: usize, buffers: Vec<JobBuffer>) {
    let mut buffers = buffers;

    let local_buffer = buffers[index].duplicate();
    let mut steal_buffers = Vec::new();
    for (i, steal_buffer) in buffers.iter_mut().enumerate() {
        if i == index {
            continue;
        }

        steal_buffers.push(steal_buffer.duplicate());
    }

    WORKER_THREAD.with(move |worker_thread| {
        *worker_thread.borrow_mut() = Some(WorkerThread {
            local_buffer,
            steal_buffers,
            index,
        });
    });
}

fn run_worker_thread() {
    while !DONE.load(Ordering::SeqCst) {
        run_pending_job();
    }

    empty_buffer();
}

fn empty_buffer() {
    loop {
        ris_log::trace!("emptying {}", thread_index());
        match pop_job() {
            Ok(mut job) => job.invoke(),
            Err(IsEmpty) => break,
        }
    }
}

fn pop_job() -> Result<Job, IsEmpty> {
    let mut result = Err(IsEmpty);

    WORKER_THREAD.with(|worker_thread| {
        if let Some(worker_thread) = worker_thread.borrow_mut().as_mut() {
            result = worker_thread.local_buffer.wait_and_pop();
        } else {
            ris_log::error!("couldn't pop job, calling thread isn't a worker thread");
        }
    });

    result
}

fn steal_job() -> Result<Job, BlockedOrEmpty> {
    let mut result = Err(BlockedOrEmpty);

    WORKER_THREAD.with(|worker_thread| {
        if let Some(worker_thread) = worker_thread.borrow_mut().as_mut() {
            for buffer in &mut worker_thread.steal_buffers {
                result = buffer.steal();
                if result.is_ok() {
                    break;
                }
            }
        } else {
            ris_log::error!("couldn't steal job, calling thread isn't a worker thread");
        }
    });

    result
}
