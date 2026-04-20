use crate::app::utils::debug::SoftExpect;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use priority_queue::PriorityQueue;
use ratelimit::Ratelimiter;
use smallvec::SmallVec;
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

pub struct AsyncRequestsPlugin<K, C>
where
    K: RequestKind + Send + Sync + 'static,
    C: RequestClient<K> + Clone + Send + Sync + 'static,
{
    _marker: PhantomData<K>,
    _marker2: PhantomData<C>,
}

impl<K, C> Default for AsyncRequestsPlugin<K, C>
where
    K: RequestKind + Send + Sync + 'static,
    C: RequestClient<K> + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, C> AsyncRequestsPlugin<K, C>
where
    K: RequestKind + Send + Sync + 'static,
    C: RequestClient<K> + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            _marker2: PhantomData,
        }
    }
}

impl<K, C> Plugin for AsyncRequestsPlugin<K, C>
where
    K: RequestKind + Send + Sync + 'static,
    C: RequestClient<K> + Clone + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_requests::<K, C>);
    }
}

pub trait RequestKind {
    type Key: Default + Debug + Eq + Hash + Clone + Send + Sync + 'static;
    type Value: Send + Sync + 'static;
    type Error: Display + Send + Sync + 'static;
}

pub trait RequestClient<K: RequestKind> {
    fn fetch_preflight(
        &self,
        key: &K::Key,
    ) -> impl Future<Output = Result<Option<K::Value>, K::Error>> + Send;

    fn fetch(&self, key: &K::Key) -> impl Future<Output = Result<K::Value, K::Error>> + Send;
}

pub enum RequestState<K: RequestKind> {
    PendingPreflight,
    LoadingPreflight,
    Pending,
    Loading,
    Completed(Result<K::Value, K::Error>),
}

impl<K: RequestKind> Default for RequestState<K> {
    fn default() -> Self {
        Self::PendingPreflight
    }
}

impl<K: RequestKind> RequestState<K> {
    pub fn is_completed(&self) -> bool {
        matches!(self, RequestState::Completed(_))
    }
}

#[derive(Component, Reflect)]
pub struct Request<K: RequestKind> {
    pub priority: isize,
    #[reflect(ignore)]
    key: K::Key,
    #[reflect(ignore)]
    state: RequestState<K>,
}

impl<K: RequestKind> Request<K> {
    pub fn new(key: K::Key, priority: isize) -> Self {
        Self {
            key,
            priority,
            state: RequestState::PendingPreflight,
        }
    }

    pub fn key(&self) -> &K::Key {
        &self.key
    }

    pub fn state(&self) -> &RequestState<K> {
        &self.state
    }
}

#[derive(Component)]
pub struct RequestManager<K: RequestKind, C: RequestClient<K> + Clone> {
    pub max_concurrent: usize,
    pub rate_limiter: Option<Ratelimiter>,
    fetching: HashSet<(Entity, K::Key)>,
    client: C,
}

impl<K: RequestKind, C: RequestClient<K> + Clone> RequestManager<K, C> {
    pub fn new(max_concurrent: usize, rate_limiter: Option<Ratelimiter>, client: C) -> Self {
        Self {
            max_concurrent,
            rate_limiter,
            fetching: HashSet::new(),
            client,
        }
    }
}

#[derive(Component)]
#[relationship(relationship_target = ManagerWithRequests)]
pub struct RequestWithManager(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = RequestWithManager)]
pub struct ManagerWithRequests(Vec<Entity>);

fn handle_requests<
    K: RequestKind + 'static,
    C: RequestClient<K> + Clone + Send + Sync + 'static,
>(
    runtime: Res<TokioTasksRuntime>,
    managers: Query<(Entity, &mut RequestManager<K, C>, &ManagerWithRequests)>,
    mut requests: Query<(Entity, &mut Request<K>)>,
) {
    for (manager_id, mut manager, ManagerWithRequests(request_ids)) in managers {
        let mut pending_requests = PriorityQueue::<(Entity, K::Key), isize>::new();
        let mut pending_preflight_requests = SmallVec::<[(Entity, K::Key); 10]>::new();

        for (request_id, request) in request_ids
            .iter()
            .filter_map(|id| requests.get(*id).ok().soft_expect(""))
        {
            match request.state() {
                RequestState::PendingPreflight => {
                    pending_preflight_requests.push((request_id, request.key().clone()));
                }
                RequestState::Pending => {
                    pending_requests.push((request_id, request.key().clone()), request.priority);
                }
                _ => {}
            }
        }

        for (req_id, req_key) in pending_preflight_requests.into_iter() {
            let client = manager.client.clone();
            let req_key = req_key.clone();

            if let Some((_, mut request)) = requests.get_mut(req_id).ok().soft_expect("") {
                request.state = RequestState::LoadingPreflight;
            }

            runtime.spawn_background_task(async move |mut task| {
                let res = client.fetch_preflight(&req_key).await;

                task.run_on_main_thread(move |ctx| {
                    if let Ok(mut entity) = ctx.world.get_entity_mut(req_id)
                        && let Ok(mut req) = entity
                            .get_mut::<Request<K>>()
                            .ok_or("Could not find component")
                            .inspect_err(|err| error!("Failed to get request components: {}", err))
                    {
                        req.state = match res {
                            Ok(Some(value)) => RequestState::Completed(Ok(value)),
                            Ok(None) => RequestState::Pending,
                            Err(err) => RequestState::Completed(Err(err)),
                        }
                    }
                })
                .await;
            });
        }

        while manager.fetching.len() < manager.max_concurrent
            && let Some(((req_id, req_key), _)) = pending_requests.pop()
            && manager
                .rate_limiter
                .as_mut()
                .is_none_or(|r| r.try_wait().is_ok())
        {
            manager.fetching.insert((req_id, req_key.clone()));

            if let Some((_, mut request)) = requests.get_mut(req_id).ok().soft_expect("") {
                request.state = RequestState::Loading;
            }

            let client = manager.client.clone();
            let req_key = req_key.clone();

            runtime.spawn_background_task(async move |mut task| {
                debug!("Fetching request: {:?}", req_key);

                let res = client.fetch(&req_key).await;
                let _ = res
                    .as_ref()
                    .inspect_err(|err| error!("Failed to fetch request: {}", err));
                {
                    let req_key = req_key.clone();
                    task.run_on_main_thread(move |ctx| {
                        if let Ok(mut entity) = ctx.world.get_entity_mut(req_id)
                            && let Ok(mut req) = entity
                                .get_mut::<Request<K>>()
                                .ok_or("Could not find component")
                                .inspect_err(|err| {
                                    error!("Failed to get request components: {}", err)
                                })
                        {
                            req.state = RequestState::Completed(res);
                        }

                        if let Ok(mut entity) = ctx.world.get_entity_mut(manager_id)
                            && let Ok(mut manager) = entity
                                .get_mut::<RequestManager<K, C>>()
                                .ok_or("Could not find component")
                                .inspect_err(|err| {
                                    error!("Failed to get request components: {}", err)
                                })
                        {
                            manager.fetching.remove(&(req_id, req_key));
                        }
                    })
                    .await;
                }

                debug!("Finished request: {:?}", req_key);
            });
        }
    }
}
