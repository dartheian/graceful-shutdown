use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_util::sync::CancellationToken;

async fn send_ack(sender: UnboundedSender<u32>, message: u32) {
    match sender.send(message) {
        Ok(()) => {
            println!("{message}: Shutdown ack sent.");
        }
        Err(send_error) => {
            println!("{message}: Failed to send ack. ({send_error})");
        }
    }
}

async fn task(
    id: u32,
    seconds: u64,
    sender: UnboundedSender<u32>,
    token: CancellationToken,
) -> Option<u32> {
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(seconds)) => {
            println!("{id}: Job finished.");
            println!("{id}: Send shutdown signal to remaining tasks.");
            token.cancel();
            Some(id)
        }
        _ = token.cancelled() => {
            println!("{id}: Shutdown signal received.");
            send_ack(sender, id).await;
            None
        },
    }
}

#[tokio::main]
async fn main() {
    let (shutdown_sender, mut shutdown_receiver) = mpsc::unbounded_channel();
    let cancellation_token = CancellationToken::new();

    tokio::spawn(task(
        1,
        3,
        shutdown_sender.clone(),
        cancellation_token.clone(),
    ));

    for i in 2..=3 {
        tokio::spawn(task(
            i,
            10,
            shutdown_sender.clone(),
            cancellation_token.clone(),
        ));
    }

    drop(shutdown_sender);

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("0: Ctrl-C received.");
            println!("0: Shutting down tasks...");
            cancellation_token.cancel();
            while let Some(_) = shutdown_receiver.recv().await {};
            println!("0: All task have been shut down.");
        },
        Some(id) = shutdown_receiver.recv() => {
            println!("0: Shutdown ack received from {id}");
            while let Some(id) = shutdown_receiver.recv().await {
                println!("0: Shutdown ack received from {id}");
            };
            println!("0: All task have finished.");
        },
    }
}
