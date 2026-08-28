use crate::{Authority, EventBus, GraphGenerationId, KernelEvent};
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
    graph_generation: GraphGenerationId,
}

impl CancellationToken {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn authority(&self) -> &Authority {
        &self.authority
    }

    pub fn graph_generation(&self) -> &GraphGenerationId {
        &self.graph_generation
    }
}

pub struct TaskHandle<T> {
    id: u64,
    graph_generation: GraphGenerationId,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<T>,
    join: JoinHandle<()>,
    events: Arc<EventBus>,
}

impl<T> TaskHandle<T> {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn graph_generation(&self) -> &GraphGenerationId {
        &self.graph_generation
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
        graph_generation: &GraphGenerationId,
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
            graph_generation: graph_generation.clone(),
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let join = thread::spawn(move || {
            let output = worker(token);
            let _ = sender.send(output);
        });

        TaskHandle {
            id,
            graph_generation: graph_generation.clone(),
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
    use crate::{
        CapabilityId, ComponentManifest, ConfigContribution, PluginManifest, ResolvedHarness,
    };
    use std::thread;

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn generation_with_authority(authority: &Authority) -> GraphGenerationId {
        ResolvedHarness::resolve(
            Vec::<PluginManifest>::new(),
            Vec::<ComponentManifest>::new(),
            Vec::<ConfigContribution>::new(),
            authority,
        )
        .unwrap()
        .generation()
        .clone()
    }

    fn generation() -> GraphGenerationId {
        generation_with_authority(&Authority::default())
    }

    #[test]
    fn blocking_cancellation_is_observable() {
        let runtime = TaskRuntime::default();
        let events = runtime.events();
        let event_receiver = events.subscribe();
        let authority = Authority::default();

        let handle = runtime.spawn(&generation(), &authority, &authority, |token| {
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

        let handle = runtime.spawn(&generation(), &parent, &requested, move |token| {
            (
                token.authority().permits(&read),
                token.authority().permits(&write),
            )
        });

        assert_eq!(handle.join().unwrap(), (true, false));
    }

    #[test]
    fn task_and_worker_remain_pinned_to_starting_graph_generation() {
        let runtime = TaskRuntime::default();
        let authority = Authority::default();
        let starting_generation = generation();
        let expected_worker_generation = starting_generation.clone();

        let handle = runtime.spawn(&starting_generation, &authority, &authority, move |token| {
            token.graph_generation().clone()
        });
        let replacement_generation =
            generation_with_authority(&Authority::new([capability("generation.replacement")]));

        assert_ne!(replacement_generation, starting_generation);
        assert_ne!(handle.graph_generation(), &replacement_generation);
        assert_eq!(handle.graph_generation(), &starting_generation);
        assert_eq!(handle.join().unwrap(), expected_worker_generation);
    }
}
