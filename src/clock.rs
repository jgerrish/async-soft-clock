//! The clock module contains structures and functions for working
//! with software clocks.
use log::{error, info};

// use std::sync::Mutex;

#[cfg(feature = "async_tokio")]
use tokio::{
    self,
    sync::{
        mpsc::{channel as bounded, Receiver, Sender},
        // Use the heavy weight Tokio mutex which doesn't allow crossing
        // between async boundaries.
        // We use this to protect the entire start function.
        Mutex,
        // Use the Tokio RwLock reader-writer lock to protect access
        // to the clock listeners.
        RwLock,
    },
};

use crate::{
    error::{Error, ErrorKind},
    listener::ClockListener,
    time::{interval, Duration},
};

/// A Clock structure.
///
/// Clocks have a single clock source, whether it's the built in period or
/// another clock driving them.
///
/// They can be started once and provide an authoritative source of time.
/// TODO: Make sure the start function can only be run one at a time.
pub struct Clock {
    /// The clock period in nanoseconds
    clock_period: f64,

    /// A mutex to protect the variable controlling whether the clock
    /// is running or stopped so tasks can't start the clock at the
    /// same time.
    running: Mutex<bool>,
    // running: bool,
    /// Channel endpoint that listens for stop events
    stop_receiver: Receiver<()>,

    /// Vector of listeners to send signals on clock state change.
    /// The listeners are protected by a reader-writer lock.
    /// The current use case doesn't have multiple readers using
    /// listeners, but the protection is there for future use cases.
    listeners: RwLock<Vec<ClockListener>>,
}

impl Default for Clock {
    fn default() -> Clock {
        let (_tx, rx) = bounded(100);

        Clock {
            clock_period: 0.0,
            running: Mutex::new(false),
            // running: false,
            stop_receiver: rx,
            listeners: RwLock::new(Vec::new()),
        }
    }
}

impl Clock {
    /// Create a new clock with a given clock frequency in Hz
    ///
    /// # Examples
    ///
    /// ```
    /// use async_soft_clock::clock::Clock;
    ///
    /// // Create a clock that ticks every two seconds
    /// let mut clock = Clock::new(0.5);
    /// ```
    pub fn new(freq: f64) -> Clock {
        let nano_seconds: f64 = (1.0 / freq) / 0.000000001;
        // Create a channel to stop the clock.
        // With the channel / bounded function, the sender will block if the queue
        // is full.  If we can't risk other tasks failing this is bad.
        // There is a send_timeout method on Sender, which will timeout even if the
        // queue is full.  If we can't risk losing data this is bad.
        // The alternative is to use unbounded_channel.  The failure mode there is
        // running out of memory.  If we can't risk crashing this is bad.
        let (_tx, rx) = bounded(100);

        Clock {
            clock_period: nano_seconds,
            running: Mutex::new(false),
            // running: false,
            stop_receiver: rx,
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Create a new clock with a given clock frequency in Hz
    /// and a stopper Receiver.
    /// Sending the stopper a unit message () will stop the clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use tokio::{runtime, sync::mpsc};
    /// use async_soft_clock::clock::Clock;
    ///
    /// // Create a channel to stop the clock
    /// let (stopper, rx) = mpsc::channel(100);
    ///
    /// // Create a clock that ticks every two seconds
    /// let mut clock = Clock::new_with_stopper(0.5, rx);
    ///
    pub fn new_with_stopper(freq: f64, rx: Receiver<()>) -> Clock {
        let nano_seconds: f64 = (1.0 / freq) / 0.000000001;

        Clock {
            clock_period: nano_seconds,
            running: Mutex::new(false),
            // running: false,
            stop_receiver: rx,
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Called for every tick
    pub async fn tick(&self) {
        let listeners = self.listeners.read().await;

        info!("Clock ticking");

        for listener in (*listeners).iter() {
            // Attempt to send until a timeout is reached.
            let res = listener
                .tx
                .send_timeout((), Duration::from_millis(100).try_into().unwrap())
                .await;
            // We could add another test here for whether we are
            // running.  For now, send out updates to all clients if
            // we send them out to any.
            match res {
                Ok(()) => {}
                Err(e) => {
                    error!("Send error: {}", e);
                }
            }
        }
    }

    /// Start the Clock
    ///
    /// This starts the interval timer on the clock.  It updates any
    /// listeners on every tick.
    /// Multiple listeners may subscribe to the clock.  A start
    /// command starts the clock for all listeners.
    ///
    /// The clock listens for stop messages at yield points between
    /// it's own tick and sending out tick updates to listeners.
    ///
    /// If another task attempts to start the clock while it is
    /// running, the call to start will return with an
    /// ClockAlreadyRunning error.
    ///
    /// Using Tokio 1.19.2 this doesn't seem to behave as expected,
    /// try_lock on a single-threaded runtime will block until the
    /// lock is released.
    ///
    /// # Returns
    ///
    /// Returns true if the Clock exited through normal means.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use tokio::{runtime, sync::mpsc, task};
    /// use async_soft_clock::clock::{Clock, stop_clock};
    ///
    /// async fn run_clock(mut clock: Clock) {
    ///     clock.start().await.unwrap();
    /// }
    ///
    /// // Create a channel to stop the clock
    /// let (stopper, rx) = mpsc::channel(100);
    ///
    /// // Create a clock that ticks every two seconds
    /// let mut clock = Clock::new_with_stopper(0.5, rx);
    ///
    /// let rt =
    ///     runtime::Builder::new_current_thread().enable_time().build();
    ///
    /// rt.unwrap().block_on(async {
    ///     let local = task::LocalSet::new();
    ///     local.spawn_local(run_clock(clock));
    ///     // Immediately stop the clock
    ///     local.spawn_local(stop_clock(stopper));
    ///     local.await;
    /// });
    ///
    /// ```
    pub async fn start(&mut self) -> Result<bool, Error> {
        let running_result = self.running.try_lock();

        let mut running = match running_result {
            Ok(r) => r,
            Err(_e) => {
                error!("Already started the clock");
                return Err(Error::new(ErrorKind::ClockAlreadyRunning));
            }
        };

        info!("Starting clock");

        let mut interval = interval(Duration::from_nanos(self.clock_period.round() as u64));

        *running = true;

        // Run the clock.
        // Listen for stop requests at the beginning, and before
        // sending out ticks to listeners.
        while *running {
            let res = self.stop_receiver.try_recv();
            if let Ok(_result) = res {
                *running = false;
                info!("Received message to stop clock");
            } else {
                interval.tick().await;
            }
            let res = self.stop_receiver.try_recv();
            if let Ok(_result) = res {
                *running = false;
                info!("Received message to stop clock");
            } else {
                self.tick().await;
            }
        }

        Ok(true)
    }

    /// This is the standard registration method, register for when a
    /// clock pulse occurs and the clock goes "high".
    ///
    /// This blocks if other tasks are trying to add listeners until
    /// they complete.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use tokio::{runtime, sync::mpsc::{self, Receiver}, task};
    /// use async_soft_clock::clock::{Clock, stop_clock};
    ///
    /// // A simple function to run the clock
    /// async fn run_clock(mut clock: Clock) {
    ///     clock.start().await.unwrap();
    /// }
    ///
    /// // A simple function to listen for ticks and print when it receives them
    /// async fn run_listener(mut rx: Receiver<()>) {
    ///     while let Some(_res) = rx.recv().await {
    ///         println!("got a tick");
    ///     }
    /// }
    ///
    /// // Create a channel to stop the clock
    /// let (stopper, rx) = mpsc::channel(100);
    ///
    /// // Create a clock that ticks every two seconds
    /// let mut clock = Clock::new_with_stopper(0.5, rx);
    ///
    /// // Create a listener to listen for ticks
    /// let tick_listener = clock.register();
    ///
    /// let rt =
    ///     runtime::Builder::new_current_thread().enable_time().build();
    ///
    /// rt.unwrap().block_on(async {
    ///     let local = task::LocalSet::new();
    ///     local.spawn_local(run_clock(clock));
    ///     local.spawn_local(run_listener(tick_listener));
    ///     // Immediately stop the clock
    ///     local.spawn_local(stop_clock(stopper));
    ///     local.await;
    /// });
    ///
    /// ```
    pub fn register(&mut self) -> Receiver<()> {
        let mut listeners = self.listeners.blocking_write();

        info!("Registering tick listener");

        // TODO: Figure out number for this
        let (tx, rx) = bounded(100);

        (*listeners).push(ClockListener { tx });
        rx
    }
}

/// Stop the clock.
/// Calling this function stops the clock.
/// This may potentially be called by other threads or tasks.
/// The clock may be shared among multiple listeners,
/// tasks or threads.  Once a stop is requested, the clock will be
/// stopped and the current start task safely exited.
/// This will stop the clock for all listeners.
/// Any subsequent stop requests during the shutdown process have no
/// additional effect.
/// The clock is stopped as soon as possible,
/// If it processed a tick, but hasn't sent out the tick to listening
/// clients, it may not send it out after receiving a shutdown
/// request.
pub async fn stop_clock(stopper: Sender<()>) {
    info!("Stopping the clock");
    stopper.send(()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::Clock;
    use env_logger;
    use log::{error, info};
    use tokio::{
        runtime,
        sync::mpsc::{self, Receiver, Sender},
        task,
    };

    async fn run_listener(mut rx: Receiver<()>, ticks_to_run_for: u32, stopper_tx: Sender<()>) {
        info!("Running listener in main");
        let mut cnt = 0;
        while cnt < ticks_to_run_for {
            if let Some(_res) = rx.recv().await {}
            cnt += 1;
        }

        // Shutdown the clock
        let res = stopper_tx.send(()).await;
        match res {
            Ok(()) => {
                info!("Sent shutdown message");
            }
            Err(e) => {
                error!("Couldn't send shutdown message: {}", e);
            }
        }
    }

    /// This task listens for a message to shutdown the clock from the
    /// main application.  When it receives one, it sends a message to the
    /// Clock on the channel that was created when the Clock was created.
    async fn run_stopper(mut stopper_rx: Receiver<()>, stopper_tx: Sender<()>) {
        info!("Running stopper listener");
        stopper_rx.recv().await;

        info!("Received message to send stop message");
        stopper_tx.send(()).await.unwrap();
        info!("Received message to send stop message");
        stopper_tx.send(()).await.unwrap();
    }

    async fn run_clock(mut clock: Clock) {
        let result = clock.start().await;
        match result {
            Ok(true) => {}
            Ok(false) => {
                println!("Couldn't start clock");
            }
            Err(err) => {
                println!("Error: {}", err);
            }
        }
        let result = clock.start().await;
        match result {
            Ok(true) => {}
            Ok(false) => {
                println!("Couldn't start clock");
            }
            Err(err) => {
                println!("Error: {}", err);
            }
        }
    }

    /// Test that trying to start the clock twice fails
    #[test]
    fn start_twice_fails() {
        // Initialize logger
        if let Err(e) = env_logger::try_init() {
            panic!("couldn't initialize logger: {:?}", e);
        }

        // Create a channel to stop the clock
        let (stopper, rx) = mpsc::channel(1);

        // Create a clock that ticks every 0.1 seconds
        let mut clock = Clock::new_with_stopper(100.0, rx);

        let tick_listener = clock.register();

        let rt = runtime::Builder::new_current_thread().enable_time().build();

        rt.unwrap().block_on(async {
            // Create a channel to stop the clock
            let (stopper_tx, stopper_rx) = mpsc::channel(100);

            let local = task::LocalSet::new();
            local.spawn_local(run_listener(tick_listener, 3, stopper_tx));
            local.spawn_local(run_stopper(stopper_rx, stopper));
            local.spawn_local(run_clock(clock));

            local.await;
        });
    }
}
