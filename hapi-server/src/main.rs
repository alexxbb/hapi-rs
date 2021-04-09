use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use env_logger;
use log::{debug, info, warn};
use rand;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use warp;
use warp::filters::BoxedFilter;
use warp::hyper::body::Bytes;
use warp::{Filter, Rejection, Reply};

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    show: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct SessionHandle(pub u64);

impl SessionHandle {
    pub fn new(show: &Option<String>) -> Self {
        use std::hash::Hasher;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        if let Some(show) = show {
            h.write(show.as_bytes());
        }
        SessionHandle(h.finish())
    }
}

type SessionsMap = Arc<Mutex<HashMap<SessionHandle, Session>>>;
type Jobs = Arc<Mutex<VecDeque<Job>>>;

#[derive(Debug)]
pub struct Session {
    hars_session: Option<()>,
    jobs: Jobs,
}

#[derive(Debug)]
pub struct Job {}

#[derive(Debug)]
pub struct Kiosk {
    sessions: SessionsMap,
}

#[derive(Debug, Serialize)]
pub struct Ticket {
    session_handle: SessionHandle,
}

impl warp::Reply for Ticket {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::Response::new(format!("{:?}", self.session_handle).into())
    }
}

impl Kiosk {
    pub async fn submit_job(&mut self, req: Request) {
        // find session which satisfy task
        let hdl = SessionHandle::new(&req.show);
        let mut sessions = self.sessions.lock().await;
        match sessions.get_mut(&hdl) {
            None => {
                let mut sessions = Arc::clone(&self.sessions);
                tokio::spawn(async move {
                    let mut session = Kiosk::start_session().await;
                    let job = Job {};
                    session.jobs.lock().await.push_back(job);
                    sessions.lock().await.insert(hdl, session);
                });
            }
            Some(s) => s.jobs.lock().await.push_back(Job {}),
        }
    }

    pub async fn start_job_worker(sessions: SessionsMap) {
        tokio::task::spawn_blocking(move || {
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let sessions = Arc::clone(&sessions);
                tokio::spawn(async move {
                    let sessions = sessions.lock().await;
                    let keys = sessions.keys().collect::<Vec<_>>();
                    dbg!(&keys);
                });
            }
        });
    }

    pub async fn run_task(&mut self, hdl: SessionHandle, task: Job) {
        tokio::task::spawn_blocking(move || {});
    }

    async fn start_session() -> Session {
        // debug!("Starting new session");
        tokio::task::spawn_blocking(move || {
            // 1 start HARS session
            let mut new_session = Session {
                hars_session: Some(()),
                jobs: Default::default(),
            };
            new_session
        })
        .await
        .expect("Oops")
    }

    fn start_session_2(&self, hdl: SessionHandle) {
        debug!("Starting new session: {}", hdl.0);
        let sessions = self.sessions.clone();
        tokio::task::spawn_blocking(move || {
            // 1 start HARS session
            let mut new_session = Session {
                hars_session: Some(()),
                jobs: Default::default(),
            };
            // 2 add to sessions map
            tokio::task::spawn(async move {
                sessions.lock().await.insert(hdl, new_session);
            });
        });
    }
}

pub type State = Arc<Mutex<_State>>;

pub struct _State {
    pub kiosk: Kiosk,
}

fn log_body() -> impl Filter<Extract = (), Error = Rejection> + Copy {
    use std::io::Read;

    warp::body::bytes()
        .map(|b: Bytes| {
            println!(
                "[Request body]: {}",
                std::str::from_utf8(&b).expect("error converting bytes to &str")
            );
        })
        .untuple_one()
}

fn api_entry(state: State) -> BoxedFilter<(impl Reply,)> {
    let state_filter = warp::any().map(move || state.clone());
    let session_get = warp::path("sessions")
        .and(warp::get())
        .and(state_filter.clone())
        .and_then(handlers::list_sessions);
    let new_job = warp::path("jobs")
        .and(warp::post())
        // .and(log_body())
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(handlers::new_job);

    let session_stop = warp::path!("sessions" / i64)
        .and(warp::post())
        .and(state_filter.clone())
        .and_then(handlers::stop_session);

    session_get.or(session_stop).or(new_job).boxed()
}

mod handlers {
    use super::*;

    pub async fn list_sessions(state: State) -> Result<impl Reply, Infallible> {
        Ok(warp::reply::reply())
    }

    pub async fn new_job(req: Request, state: State) -> Result<impl Reply, Infallible> {
        let ticket = state.lock().await.kiosk.submit_job(req).await;
        Ok("Privet")
    }

    pub async fn stop_session(session_id: i64, state: State) -> Result<impl Reply, Infallible> {
        debug!("Stopping session: {}", session_id);
        Ok(warp::reply::reply())
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let state = _State {
        kiosk: Kiosk {
            sessions: Default::default(),
        },
    };
    let sessions = Arc::clone(&state.kiosk.sessions);
    tokio::task::spawn(Kiosk::start_job_worker(sessions));
    let entry = api_entry(Arc::new(Mutex::new(state)));
    warp::serve(entry).run(([127, 0, 0, 1], 3030)).await;
}
