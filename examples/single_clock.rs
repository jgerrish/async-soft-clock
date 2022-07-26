//! Run a single clock in a Tokio runtime.
//! Shows how to create a clock with a shutdown listener.
//! Run for a few ticks, and then shutdown the clock.
#[warn(missing_docs)]
#[warn(unsafe_code)]
use Default;

use env_logger;
use log::{error, info};

use tokio::sync::mpsc::{self, Receiver, Sender};

use async_soft_clock::async_executor::{AsyncExecutor, AsyncRunner};
use async_soft_clock::clock::Clock;

/// A single listener task that processes clock ticks.
async fn run_listener(mut rx: Receiver<()>, ticks_to_run_for: u32, stopper_tx: Sender<()>) {
    info!("Running listener in main");
    let mut cnt = 0;

    while cnt < ticks_to_run_for {
        if let Some(_res) = rx.recv().await {
            println!("got a tick");
        }
        cnt += 1;
    }

    rx.close();

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

/// A single Clock task that increments the Clock and messages
/// listeners on every tick.
async fn run_clock(mut clock: Clock) {
    info!("Starting the clock in main");

    let result = clock.start().await;

    match result {
        Ok(_) => {
            println!("Finished task");
        }
        Err(e) => {
            println!("Error with task: {}", e);
        }
    }
}

/// This task listens for a message to shutdown the clock from the
/// main application.  When it receives one, it sends a message to the
/// Clock on the channel that was created when the Clock was created.
async fn run_stopper(mut stopper_rx: Receiver<()>, stopper_tx: Sender<()>) {
    async {
        info!("Running stopper listener");
        stopper_rx.recv().await;

        info!("Received message to send stop message");
        stopper_tx.send(()).await.unwrap();
    }
    .await;
}

/// Run all of the tasks using the async runner in the selected
/// runtime.
fn run_tasks(clock: Clock, tick_listener: Receiver<()>, stopper: Sender<()>) {
    let runner = AsyncRunner::default();

    // Create a channel to stop the clock
    let (stopper_tx, stopper_rx) = mpsc::channel(1);

    runner.spawn(run_listener(tick_listener, 3, stopper_tx));
    runner.spawn(run_stopper(stopper_rx, stopper));
    runner.spawn(run_clock(clock));

    runner.run();
}

fn main() {
    // Initialize logger
    if let Err(e) = env_logger::try_init() {
        panic!("couldn't initialize logger: {:?}", e);
    }

    // Create a channel to stop the clock
    let (stopper, rx) = mpsc::channel(100);

    // Tick every two seconds
    let mut clock = Clock::new_with_stopper(0.5, rx);

    let tick_listener = clock.register();

    run_tasks(clock, tick_listener, stopper);
}
