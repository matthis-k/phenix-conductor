use crate::{Authority, EventBus, KernelEvent};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
};

#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    authority: Authority,
}

impl CancellationToken {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn authority(&self) -> &Authority {
        &self.authority
    }
}

pub struct TaskHandle<T> {
    id: u64,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<T>,
    join: JoinHandle<()>,
    events: Arc<EventBus>,
}

impl<T> TaskHandle<T> {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn cancel(&self) {
        if !self.cancelled.swap(true, Ordering::AcqRel) {
            self.events.publish(KernelEvent::TaskCancelled(self.id));
        }
    }

    pub fn join(self) -> thread::Result<T> {
        self.join.join()?;
        Ok(self
            .receiver
            .recv()
            .expect("task worker exited without sending a result"))
    }
}

#[derive(Debug)]
pub struct TaskRuntime {
    next_id: AtomicU64,
    events: Arc<EventBus>,
}

impl Default for TaskRuntime {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            events: Arc::new(EventBus::default()),
        }
    }
}

impl TaskRuntime {
    pub fn events(&self) -> Arc<EventBus> {
        Arc::clone(&self.events)
    }

    pub fn spawn<T, F>(
        &self,
        parent_authority: &Authority,
        requested_authority: &Authority,
        worker: F,
    ) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(CancellationToken) -> T + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let cancelled = Arc::new(AtomicBool::new(false));
        let token = CancellationToken {
            cancelled: Arc::clone(&cancelled),
            authority: parent_authority.attenuate(requested_authority),
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let join = thread::spawn(move || {
            let output = worker(token);
            let _ = sender.send(output);
        });

        TaskHandle {
            id,
            cancelled,
            receiver,
            join,
            events: Arc::clone(&self.events),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapabilityId;
    use std::thread;

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    #[test]
    fn blocking_cancellation_is_observable() {
        let runtime = TaskRuntime::default();
        let events = runtime.events();
        let event_receiver = events.subscribe();
        let authority = Authority::default();

        let handle = runtime.spawn(&authority, &authority, |token| {
            while !token.is_cancelled() {
                thread::yield_now();
            }
            42
        });
        let id = handle.id();
        handle.cancel();

        assert_eq!(
            event_receiver.recv().unwrap(),
            KernelEvent::TaskCancelled(id)
        );
        assert_eq!(handle.join().unwrap(), 42);
    }

    #[test]
    fn blocking_task_authority_is_attenuated_from_parent() {
        let runtime = TaskRuntime::default();
        let read = capability("fs.read");
        let write = capability("fs.write");
        let parent = Authority::new([read.clone()]);
        let requested = Authority::new([read.clone(), write.clone()]);

        let handle = runtime.spawn(&parent, &requested, move |token| {
            (
                token.authority().permits(&read),
                token.authority().permits(&write),
            )
        });

        assert_eq!(handle.join().unwrap(), (true, false));
    }
}
